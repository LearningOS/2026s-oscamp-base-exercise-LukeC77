//! # Single‑Level Page Table Address Translation
//!
//! This exercise simulates a simple single‑level page table to help you understand the process of virtual‑to‑physical address translation.
//!
//! ## Concepts
//! - Virtual address = Virtual Page Number (VPN) + Page Offset (offset)
//! - Page table: VPN → PPN mapping table
//! - Address translation: Physical address = PPN × PAGE_SIZE + offset
//! - Page fault: accessing an unmapped virtual page
//!
//! ## Address Format (Simplified Model)
//! ```text
//! Virtual address (32‑bit):
//! 31          12 11          0
//! ┌──────────────┬────────────┐
//! │   VPN (20 bits)  │ offset (12 bits) │
//! └──────────────┴────────────┘
//!
//! Page size: 4KB (2^12 = 4096 bytes)
//! 
//! offset有12位，所以它能表示2^12=4096个位置，一页有4096字节，因此页大小为4KB。
//! 
//! 关系链路就是：virtual address -> 拆成 VPN + offset -> 用 VPN 找 PTE -> 取 PPN -> 和 offset 拼成 physical address。
//! 
//! VPN找PTE在单级页表里通常是这样：
//! - 页表是一个 PTE 数组，page_table[vpn] 就是对应的 PTE。
//! - 硬件/内核先拿到页表基址（PTBR），PTBR 指向页表起始地址。
//! - 用 VPN 计算 PTE 地址：
//!     - pte_addr = PTBR + vpn * sizeof(PTE)   页表由若干 PTE 组成，每个 PTE 大小固定，vpn 就是索引。
//!     - 读出该 PTE，检查 valid 位等标志。
//! - 若有效则取出 PPN
//! ```

/// 页大小 4KB
pub const PAGE_SIZE: usize = 4096;
/// 页内偏移位数
pub const PAGE_OFFSET_BITS: u32 = 12;

/// 页表项标志
/// 目的：
/// - 内存保护，防止程序乱写不该写的页（代码段通常只读/可执行）；
/// - 隔离与安全，不同进程、内核/用户的页权限不同。
/// - 支持常见机制
pub const PTE_VALID: u8 = 1 << 0; // 这页是否存在合法映射。
pub const PTE_READ: u8 = 1 << 1; // 读操作需要 PTE_READ, 没权限就触发保护异常（不是“没映射”，而是“禁止这么用”）。
pub const PTE_WRITE: u8 = 1 << 2; // 写操作需要 PTE_WRITE, 没权限就触发保护异常（不是“没映射”，而是“禁止这么用”）。

/// 页表项
#[derive(Clone, Copy, Debug)]
pub struct PageTableEntry {
    pub ppn: u32,
    pub flags: u8,
}

/// 翻译结果
#[derive(Debug, PartialEq)]
pub enum TranslateResult {
    /// 翻译成功，得到物理地址
    Ok(u32), // 物理地址位数取决于架构设计，本练习使用32位物理地址。
    /// 缺页：虚拟页未映射
    PageFault,
    /// 权限错误：尝试写入只读页
    PermissionDenied,
}

/// 单级页表，最多支持 `MAX_PAGES` 个虚拟页。
pub struct SingleLevelPageTable {
    entries: Vec<Option<PageTableEntry>>,
}

impl SingleLevelPageTable {
    /// 创建一个空页表，支持 `max_pages` 个虚拟页。
    pub fn new(max_pages: usize) -> Self {
        Self {
            entries: vec![None; max_pages],
        }
    }

    /// 将虚拟页号 `vpn` 映射到物理页号 `ppn`，并设置标志位 `flags`。
    ///
    /// 提示：在 `entries[vpn]` 处存放一个 `PageTableEntry`。
    /// 
    /// 检查 valid/read/write，发生在访问时（translate / load / store）。
    /// 这个 map() 是在建映射时，把这些标志先写进对应的 PTE。
    pub fn map(&mut self, vpn: usize, ppn: u32, flags: u8) {
        // TODO: 在页表中建立 vpn -> ppn 的映射
        self.entries[vpn] = Some(PageTableEntry {ppn, flags});
    }

    /// 取消虚拟页号 `vpn` 的映射。
    pub fn unmap(&mut self, vpn: usize) {
        // TODO: 将 entries[vpn] 设为 None
        self.entries[vpn] = None;
    }

    /// 查询虚拟页号 `vpn` 对应的页表项。
    pub fn lookup(&self, vpn: usize) -> Option<&PageTableEntry> {
        // TODO: 返回 entries[vpn] 的引用（如果存在）
        if vpn >= self.entries.len() {
            return None; // 超出页表范围
        }
        self.entries[vpn].as_ref()
    }

    /// 将虚拟地址翻译为物理地址。
    ///
    /// 步骤：
    /// 1. 从虚拟地址中提取 VPN（高 20 位）和 offset（低 12 位）
    /// 2. 用 VPN 查页表，如果未映射返回 PageFault
    /// 3. 检查 PTE_VALID 标志，未置位返回 PageFault
    /// 4. 如果 `is_write` 为 true，检查 PTE_WRITE 标志
    /// 5. 计算物理地址 = ppn * PAGE_SIZE + offset
    /// 
    /// 因为 PPN 表示“第几个物理页”，offset 表示“该页内第几个字节”。
    /// 物理内存按页连续排布时：
    /// 第 PPN 页的起始地址 = PPN * PAGE_SIZE
    /// 再加页内偏移 offset
    /// 得到最终物理地址：PPN * PAGE_SIZE + offset
    /// 
    pub fn translate(&self, va: u32, is_write: bool) -> TranslateResult {
        // TODO: 实现虚拟地址到物理地址的翻译
        // 提示：
        //   let vpn = (va >> PAGE_OFFSET_BITS) as usize;
        //   let offset = va & ((1 << PAGE_OFFSET_BITS) - 1);
        let vpn = va_to_vpn(va);
        let offset = va_to_offset(va);
        let pte = match self.lookup(vpn) {
            Some(entry) => entry,
            None => return TranslateResult::PageFault, // 未映射
        };
        if (pte.flags & PTE_VALID) == 0 {
            return TranslateResult::PageFault; // 映射无效
        }
        if is_write && (pte.flags & PTE_WRITE) == 0 {
            return TranslateResult::PermissionDenied; // 写权限不足
        }
        let pa = make_pa(pte.ppn, offset);
        TranslateResult::Ok(pa)
    }
}

/// 从虚拟地址中提取虚拟页号。
///
/// 提示：右移 PAGE_OFFSET_BITS 位。
pub fn va_to_vpn(va: u32) -> usize {
    // TODO
    (va >> PAGE_OFFSET_BITS) as usize
}

/// 从虚拟地址中提取页内偏移。
///
/// 提示：用掩码提取低 PAGE_OFFSET_BITS 位。
pub fn va_to_offset(va: u32) -> u32 {
    // TODO
    va & ((1 << PAGE_OFFSET_BITS) - 1)
}

/// 由物理页号和偏移量拼出物理地址。
pub fn make_pa(ppn: u32, offset: u32) -> u32 {
    // TODO
    ppn * PAGE_SIZE as u32 + offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_va_decompose() {
        // 虚拟地址 0x12345678
        // VPN = 0x12345, offset = 0x678
        assert_eq!(va_to_vpn(0x12345678), 0x12345);
        assert_eq!(va_to_offset(0x12345678), 0x678);
    }

    #[test]
    fn test_va_decompose_zero() {
        assert_eq!(va_to_vpn(0), 0);
        assert_eq!(va_to_offset(0), 0);
    }

    #[test]
    fn test_va_decompose_page_boundary() {
        // 正好在页边界，offset 应为 0
        assert_eq!(va_to_vpn(0x3000), 3);
        assert_eq!(va_to_offset(0x3000), 0);
    }

    #[test]
    fn test_make_pa() {
        assert_eq!(make_pa(0x80, 0x100), 0x80 * 4096 + 0x100);
        assert_eq!(make_pa(0, 0), 0);
        assert_eq!(make_pa(1, 0), 4096);
    }

    #[test]
    fn test_map_and_lookup() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(5, 100, PTE_VALID | PTE_READ);

        let entry = pt.lookup(5).expect("应该找到映射");
        assert_eq!(entry.ppn, 100);
        assert_eq!(entry.flags, PTE_VALID | PTE_READ);
    }

    #[test]
    fn test_lookup_unmapped() {
        let pt = SingleLevelPageTable::new(1024);
        assert!(pt.lookup(0).is_none());
    }

    #[test]
    fn test_unmap() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(10, 200, PTE_VALID | PTE_READ);
        assert!(pt.lookup(10).is_some());

        pt.unmap(10);
        assert!(pt.lookup(10).is_none());
    }

    #[test]
    fn test_translate_basic() {
        let mut pt = SingleLevelPageTable::new(1024);
        // 虚拟页 1 -> 物理页 0x80
        pt.map(1, 0x80, PTE_VALID | PTE_READ);

        // VA = 页1 + offset 0x100 = 0x1100
        let result = pt.translate(0x1100, false);
        // PA = 0x80 * 4096 + 0x100 = 0x80100
        assert_eq!(result, TranslateResult::Ok(0x80100));
    }

    #[test]
    fn test_translate_page_fault() {
        let pt = SingleLevelPageTable::new(1024);
        assert_eq!(pt.translate(0x5000, false), TranslateResult::PageFault);
    }

    #[test]
    fn test_translate_write_permission() {
        let mut pt = SingleLevelPageTable::new(1024);
        // 只读页
        pt.map(2, 0x90, PTE_VALID | PTE_READ);

        // 读取应成功
        assert_eq!(
            pt.translate(0x2000, false),
            TranslateResult::Ok(0x90 * PAGE_SIZE as u32)
        );
        // 写入应拒绝
        assert_eq!(
            pt.translate(0x2000, true),
            TranslateResult::PermissionDenied
        );
    }

    #[test]
    fn test_translate_writable_page() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(3, 0xA0, PTE_VALID | PTE_READ | PTE_WRITE);

        // 写入可写页应成功
        assert_eq!(
            pt.translate(0x3456, true),
            TranslateResult::Ok(0xA0 * PAGE_SIZE as u32 + 0x456)
        );
    }

    #[test]
    fn test_translate_invalid_entry() {
        let mut pt = SingleLevelPageTable::new(1024);
        // 映射了但 VALID 未置位
        pt.map(4, 0x50, PTE_READ);
        assert_eq!(pt.translate(0x4000, false), TranslateResult::PageFault);
    }

    #[test]
    fn test_multiple_mappings() {
        let mut pt = SingleLevelPageTable::new(1024);
        pt.map(0, 0x10, PTE_VALID | PTE_READ);
        pt.map(1, 0x20, PTE_VALID | PTE_READ | PTE_WRITE);
        pt.map(2, 0x30, PTE_VALID | PTE_READ);

        assert_eq!(pt.translate(0x0FFF, false), TranslateResult::Ok(0x10FFF));
        assert_eq!(pt.translate(0x1000, true), TranslateResult::Ok(0x20000));
        assert_eq!(pt.translate(0x2800, false), TranslateResult::Ok(0x30800));
    }
}
