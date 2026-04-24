//! # Page Table Entry Flags
//!
//! In this exercise, you will learn the structure of RISC-V SV39 Page Table Entry (PTE),
//! and construct/parse page table entries through bit operations.
//!
//! ## Concepts
//! - RISC-V SV39 page table entry 64-bit layout
//! - Bit operations to construct/extract fields
//! - Meaning of PTE flags (V/R/W/X/U/G/A/D)
//!
//! ## SV39 PTE Layout (64-bit)
//! ```text
//! 63    54 53        10 9  8 7 6 5 4 3 2 1 0
//! ┌───────┬────────────┬────┬─┬─┬─┬─┬─┬─┬─┬─┐
//! │ Rsvd  │  PPN[2:0]  │ RSW│D│A│G│U│X│W│R│V│
//! │ 10bit │  44 bits   │ 2b │ │ │ │ │ │ │ │ │
//! └───────┴────────────┴────┴─┴─┴─┴─┴─┴─┴─┴─┘
//! ```
//! - V (Valid): Valid bit indicating whether the page table entry is valid.
//!
//! - R/W/X (Read/Write/Execute): Permission bits for read, write, and execute access respectively.
//!
//! - U (User): User-accessible bit, allowing access from user mode.
//!
//! - G (Global): Global mapping bit (typically used for kernel space to avoid TLB flushes).
//!
//! - A (Accessed): Accessed bit, set by hardware when the page is accessed.
//!
//! - D (Dirty): Dirty bit, set by hardware when the page is written to.
//!
//! - RSW (Reserved for Supervisor Software): Two bits reserved for operating system software use.
//!
//! - PPN (Physical Page Number): Physical page number occupying 44 bits (bits [53:10]), specifying the base address of the physical page frame.
//! - PPN[2:0] (Physical Page Number): In the RISC-V SV39 paging mechanism, the Physical Page Number (PPN) is divided into three parts, which are referred to as PPN[2], PPN[1], and PPN[0]. This division is designed to support the indexing of multi-level page tables.
//! - Rsvd (Reserved): Reserved bits, typically set to 0.

/// PTE flag constants
pub const PTE_V: u64 = 1 << 0; // Valid
pub const PTE_R: u64 = 1 << 1; // Readable
pub const PTE_W: u64 = 1 << 2; // Writable
pub const PTE_X: u64 = 1 << 3; // Executable
pub const PTE_U: u64 = 1 << 4; // User accessible
pub const PTE_G: u64 = 1 << 5; // Global
pub const PTE_A: u64 = 1 << 6; // Accessed
pub const PTE_D: u64 = 1 << 7; // Dirty

/// PPN field offset and mask in PTE
const PPN_SHIFT: u32 = 10;
const PPN_MASK: u64 = (1u64 << 44) - 1; // 44-bit PPN

/// Construct a page table entry from physical page number (PPN) and flags.
///
/// PPN occupies bits [53:10], flags occupy bits [7:0].
///
/// Example: ppn=0x12345, flags=PTE_V|PTE_R|PTE_W
/// Result should be: (0x12345 << 10) | 0b111 = 0x48D1407
///
/// Hint: Shift PPN left by PPN_SHIFT bits, then OR with flags.
pub fn make_pte(ppn: u64, flags: u64) -> u64 {
    // TODO: Construct page table entry using ppn and flags
    (ppn & PPN_MASK) << PPN_SHIFT | (flags & 0xFF)
}

/// Extract physical page number (PPN) from page table entry.
///
/// Hint: Right shift by PPN_SHIFT bits, then AND with PPN_MASK.
pub fn extract_ppn(pte: u64) -> u64 {
    // TODO: Extract PPN from pte
    pte >> PPN_SHIFT & PPN_MASK
}

/// Extract flags (lower 8 bits) from page table entry.
pub fn extract_flags(pte: u64) -> u64 {
    // TODO: Extract lower 8-bit flags
    pte & 0xFF
}

/// Check whether page table entry is valid (V bit set).
pub fn is_valid(pte: u64) -> bool {
    // TODO: Check PTE_V
    (pte & PTE_V) != 0
}

/// Determine whether page table entry is a leaf PTE.
///
/// In SV39, if any of R, W, X bits is set, the PTE is a leaf,
/// pointing to the final physical page. Otherwise it points to next-level page table.
/// 
/// 因为在 RISC-V 规范里，R/W/X 被定义为“对被映射页面本身的访问权限”。
/// 如果 R/W/X 全是 0，说明它不是在描述“一个页面可怎么访问”，而是在描述“下一层页表在哪”，所以是中间节点。
/// 只要 R/W/X 任一位为 1，就已经在声明最终页面权限了，硬件就把它当作最终映射项，不再继续下钻，所以是叶子项。
pub fn is_leaf(pte: u64) -> bool {
    // TODO: Check if any of R/W/X bits is set
    (pte & (PTE_R | PTE_W | PTE_X)) != 0
}

/// Check whether page table entry permits the requested access based on given permissions.
///
/// - `read`: requires read permission
/// - `write`: requires write permission
/// - `execute`: requires execute permission
///
/// Returns true iff: PTE is valid, and each requested permission is satisfied.
/// 
/// Valid (V) 的作用是标记“这个 PTE 当前是否可用”。
/// V=1：这个条目是有效格式，硬件可以按它继续解释（下级表或叶子映射）。
/// V=0：这个条目无效，硬件把它当作不存在映射，访问会触发 page fault。
/// 
/// 为什么要允许无效：
/// 表示未映射区域：不是所有虚拟地址都要映射。
/// 支持按需分配：先留空，访问时再分配物理页（demand paging）。
/// 支持回收/换出：页被回收后把 PTE 置无效。
/// 安全隔离：防止访问不该访问的地址范围。
/// 所以 V 是页表里最基础的“是否存在映射”开关。
/// 
/// 
/// 正确规则是：
/// request 里为 true 的权限，pte 里必须也为 true。
/// request 里为 false 的权限，pte 里可以是 false 或 true，都不影响这次访问。
/// 
pub fn check_permission(pte: u64, read: bool, write: bool, execute: bool) -> bool {
    // TODO: First check if valid, then check each requested permission
    if (pte & PTE_V) == 0 {
        return false; // Not valid
    }

    if read && (pte & PTE_R) == 0 {
        return false; // Read requested but not allowed
    }
    if write && (pte & PTE_W) == 0 {
        return false; // Write requested but not allowed
    }
    if execute && (pte & PTE_X) == 0 {
        return false; // Execute requested but not allowed
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_pte_basic() {
        let pte = make_pte(0x12345, PTE_V | PTE_R | PTE_W);
        assert_eq!(extract_ppn(pte), 0x12345);
        assert_eq!(extract_flags(pte), PTE_V | PTE_R | PTE_W);
    }

    #[test]
    fn test_make_pte_zero() {
        let pte = make_pte(0, 0);
        assert_eq!(pte, 0);
        assert_eq!(extract_ppn(pte), 0);
        assert_eq!(extract_flags(pte), 0);
    }

    #[test]
    fn test_make_pte_all_flags() {
        let all = PTE_V | PTE_R | PTE_W | PTE_X | PTE_U | PTE_G | PTE_A | PTE_D;
        let pte = make_pte(0xABC, all);
        assert_eq!(extract_ppn(pte), 0xABC);
        assert_eq!(extract_flags(pte), all);
    }

    #[test]
    fn test_make_pte_large_ppn() {
        let ppn = (1u64 << 44) - 1; // maximum PPN
        let pte = make_pte(ppn, PTE_V);
        assert_eq!(extract_ppn(pte), ppn);
    }

    #[test]
    fn test_is_valid() {
        assert!(is_valid(make_pte(1, PTE_V)));
        assert!(!is_valid(make_pte(1, PTE_R))); // R set but V not set
        assert!(!is_valid(0));
    }

    #[test]
    fn test_is_leaf() {
        assert!(is_leaf(make_pte(1, PTE_V | PTE_R)));
        assert!(is_leaf(make_pte(1, PTE_V | PTE_X)));
        assert!(is_leaf(make_pte(1, PTE_V | PTE_R | PTE_W | PTE_X)));
        // Non-leaf: only V set, R/W/X all cleared
        assert!(!is_leaf(make_pte(1, PTE_V)));
        assert!(!is_leaf(make_pte(1, PTE_V | PTE_A | PTE_D)));
    }

    #[test]
    fn test_check_permission_read() {
        let pte = make_pte(1, PTE_V | PTE_R);
        assert!(check_permission(pte, true, false, false));
        assert!(!check_permission(pte, false, true, false));
        assert!(!check_permission(pte, false, false, true));
    }

    #[test]
    fn test_check_permission_rw() {
        let pte = make_pte(1, PTE_V | PTE_R | PTE_W);
        assert!(check_permission(pte, true, true, false));
        assert!(!check_permission(pte, true, true, true));
    }

    #[test]
    fn test_check_permission_all() {
        let pte = make_pte(1, PTE_V | PTE_R | PTE_W | PTE_X);
        assert!(check_permission(pte, true, true, true));
        assert!(check_permission(pte, true, false, false));
        assert!(check_permission(pte, false, false, false)); // no requirement = OK
    }

    #[test]
    fn test_check_permission_invalid() {
        // V not set, should return false even if R/W/X flags present
        let pte = make_pte(1, PTE_R | PTE_W | PTE_X);
        assert!(!check_permission(pte, true, false, false));
    }
}
