//! # SV39 三级页表
//!
//! 本练习模拟 RISC-V SV39 三级页表的构造和地址翻译。
//! 注意，实际上的三级页表实现并非如本练习中使用 HashMap 模拟，本练习仅作为模拟帮助学习。
//! 你需要实现页表的创建、映射和地址翻译（页表遍历）。
//!
//! ## 知识点
//! - SV39：39 位虚拟地址，三级页表
//! - VPN 拆分：VPN[2] (9bit) | VPN[1] (9bit) | VPN[0] (9bit)
//! - 页表遍历（page table walk）逐级查找
//! - 大页（2MB superpage）映射
//!
//! ## SV39 虚拟地址布局
//! ```text
//! 38        30 29       21 20       12 11        0
//! ┌──────────┬───────────┬───────────┬───────────┐
//! │ VPN[2]   │  VPN[1]   │  VPN[0]   │  offset   │
//! │  9 bits  │  9 bits   │  9 bits   │  12 bits  │
//! └──────────┴───────────┴───────────┴───────────┘
//! ```
//! 
//! 
//! 总结成一版完整的 SV39 数据结构描述如下：
// 1. 基本单位  
// - 物理内存按 **4KB 页**组织，`PAGE_SIZE = 4096`。  
// - “4KB 页”只是容器，既可以是**数据页**，也可以是**页表页**。

// 2. 页表页结构  
// - 一张页表页大小也是 4KB。  
// - 每个 PTE 是 8B，所以每张页表有 `4096 / 8 = 512` 个表项（`PT_ENTRIES = 512`）。  
// - 512 = `2^9`，因此每一级索引都是 9 bit。

// 3. 虚拟地址分解（SV39）  
// - VA = `VPN[2](9)` | `VPN[1](9)` | `VPN[0](9)` | `offset(12)`。  
// - `offset=12` 只由 4KB 页大小决定，和几级页表无关。

// 4. 三级页表遍历  
// - 用 `VPN[2]` 在根页表（L2）512 项中选一个 PTE。  
// - 若该 PTE 非叶子，则指向下一张页表（L1）。  
// - 用 `VPN[1]` 在 L1 选 PTE；若仍非叶子，则到 L0。  
// - 用 `VPN[0]` 在 L0 选 PTE；通常这里是叶子，给出物理页号 PPN。  
// - 最终物理地址：`PA = (PPN << 12) + offset`。

// 5. 叶子与大页  
// - 叶子不一定只在 L0。  
// - L1 叶子可映射 2MB superpage，L2 叶子可映射 1GB superpage。  
// - 原理不变：高位由叶子 PTE 决定，低位由 VA 的对应偏移位保留。

// 一句话：SV39 是“用三级 9-bit 索引在 4KB 页表页中逐级找 PTE，最后用 12-bit offset 落到目标物理页内字节”。

use std::collections::HashMap;

/// 页大小 4KB
pub const PAGE_SIZE: usize = 4096; // 2^12, 一个页表占用 4096 字节， 在这个练习（RISC-V SV39）里，标准基页就是 4KB
/// 每级页表有 512 个条目 (2^9)
pub const PT_ENTRIES: usize = 512; // 每个PTE是8个字节，512个PTE占用4096字节，正好一页。

/// PTE 标志位
pub const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;

/// PPN 在 PTE 中的偏移
const PPN_SHIFT: u32 = 10; // PTE 中 PPN 从 bit 10 开始，占 44 位（53:10）

/// 页表节点：一个包含 512 个条目的数组
#[derive(Clone)]
pub struct PageTableNode {
    pub entries: [u64; PT_ENTRIES], // key是ppn，value是pte
}

impl PageTableNode {
    pub fn new() -> Self {
        Self {
            entries: [0; PT_ENTRIES],
        }
    }
}

impl Default for PageTableNode {
    fn default() -> Self {
        Self::new()
    }
}

/// 模拟的三级页表。
///
/// 使用 HashMap<u64, PageTableNode> 模拟物理内存中的页表页。
/// `root_ppn` 是根页表所在的物理页号。
pub struct Sv39PageTable {
    /// 物理页号 -> 页表节点
    nodes: HashMap<u64, PageTableNode>,
    /// 根页表的物理页号
    pub root_ppn: u64,
    /// 下一个可分配的物理页号（简易分配器）
    next_ppn: u64,
}

/// 翻译结果
#[derive(Debug, PartialEq)]
pub enum TranslateResult {
    Ok(u64),
    PageFault,
}

impl Sv39PageTable {
    pub fn new() -> Self {
        let mut pt = Self {
            nodes: HashMap::new(),
            root_ppn: 0x80000,
            next_ppn: 0x80001,
        };
        pt.nodes.insert(pt.root_ppn, PageTableNode::new());
        pt
    }

    /// 分配一个新的物理页并初始化为空页表节点，返回其 PPN。
    fn alloc_node(&mut self) -> u64 {
        let ppn = self.next_ppn;
        self.next_ppn += 1;
        self.nodes.insert(ppn, PageTableNode::new());
        ppn
    }

    /// 从 39 位虚拟地址中提取第 `level` 级的 VPN。
    ///
    /// - level=2: 取 bits [38:30]
    /// - level=1: 取 bits [29:21]
    /// - level=0: 取 bits [20:12]
    ///
    /// 提示：右移 (12 + level * 9) 位，然后与 0x1FF 做掩码。
    pub fn extract_vpn(va: u64, level: usize) -> usize {
        // TODO: 从虚拟地址中提取指定级别的 VPN 索引
        let right_shift = 12 + level * 9; // 计算右移位数
        ((va >> right_shift) & 0x1FF) as usize// 取出 9 位 VPN 索引
    }

    /// 建立从虚拟页到物理页的映射（4KB 页）。
    ///
    /// 参数：
    /// - `va`: 虚拟地址（会自动对齐到页边界）
    /// - `pa`: 物理地址（会自动对齐到页边界）
    /// - `flags`: 标志位（如 PTE_V | PTE_R | PTE_W）
    pub fn map_page(&mut self, va: u64, pa: u64, flags: u64) {
        // TODO: 实现三级页表的映射
        //
        // 提示：你需要从根页表开始，逐级向下遍历页表层级（level 2 → level 1 → level 0）。
        // 对于中间层级（level 2 和 level 1），如果对应 VPN 的页表项（PTE）无效（PTE_V == 0），
        // 则需要分配一个新的页表节点（使用 alloc_node），并将新节点的 PPN 写入当前 PTE（仅设置 PTE_V 标志）。
        // 最后在 level 0 的 PTE 中写入目标物理页号（pa >> 12）和 flags。
        let vpn2 = Self::extract_vpn(va, 2);
        let vpn1 = Self::extract_vpn(va, 1);
        let vpn0 = Self::extract_vpn(va, 0);

        // 从根页表开始
        let root_table = self.nodes.get(&self.root_ppn).unwrap();
        
        // 处理level 2的PTE
        let mut level2_pte = root_table.entries[vpn2];
        // 如果无效，分配新节点并更新PTE
        if level2_pte & PTE_V == 0 {
            // 分配新的页表节点
            let new_ppn = self.alloc_node();
            // 更新level2_pte, 设置PTE_V并写入新节点的PPN
            level2_pte = (new_ppn << PPN_SHIFT) | PTE_V;
            self.nodes.get_mut(&self.root_ppn).unwrap().entries[vpn2] = level2_pte;
        }

        let level2_table = self.nodes.get(&(level2_pte >> PPN_SHIFT)).unwrap();

        // 处理level 1的PTE
        let mut level1_pte = level2_table.entries[vpn1];
        // 如果无效，分配新节点并更新PTE
        if level1_pte & PTE_V == 0 {
            let new_ppn = self.alloc_node();
            // 更新level1_pte, 设置PTE_V并写入新节点的PPN
            level1_pte = (new_ppn << PPN_SHIFT) | PTE_V;
            self.nodes.get_mut(&(level2_pte >> PPN_SHIFT)).unwrap().entries[vpn1] = level1_pte;
        }

        // 处理level 0的PTE，直接写入物理页号和flags
        let level0_pte = ((pa >> 12) << PPN_SHIFT) | flags; // 物理页号和标志位
        self.nodes.get_mut(&(level1_pte >> PPN_SHIFT)).unwrap().entries[vpn0] = level0_pte;
    }

    /// 遍历三级页表，将虚拟地址翻译为物理地址。
    ///
    /// 步骤：
    /// 1. 从根页表（root_ppn）开始
    /// 2. 对每一级（2, 1, 0）：
    ///    a. 用 VPN[level] 索引当前页表节点
    ///    b. 如果 PTE 无效（!PTE_V），返回 PageFault
    ///    c. 如果 PTE 是叶节点（R|W|X 有任一置位），提取 PPN 计算物理地址
    ///    d. 否则用 PTE 中的 PPN 进入下一级页表
    /// 3. level 0 的 PTE 必须是叶节点
    pub fn translate(&self, va: u64) -> TranslateResult {
        // TODO: 实现三级页表遍历
        //
        // 提示：你需要从根页表开始，按 level 2 → level 1 → level 0 的顺序逐级遍历。
        // 每一级都需要通过 VPN[level] 索引当前页表节点的条目（PTE）。
        // 如果 PTE 无效（PTE_V == 0）则产生页错误（PageFault）。
        // 如果 PTE 是叶节点（即 R、W、X 标志位中有至少一个被置位），则可以直接使用该 PTE 中的物理页号（PPN）计算最终的物理地址。
        // 否则，该 PTE 指向下一级页表节点，继续遍历下一级。
        // 遍历到 level 0 时，PTE 必须是叶节点。
        let vpn2 = Self::extract_vpn(va, 2);
        let vpn1 = Self::extract_vpn(va, 1);
        let vpn0 = Self::extract_vpn(va, 0);

        // 从根页表开始
        let root_table = self.nodes.get(&self.root_ppn).unwrap();

        // 处理level 2的PTE
        let level2_pte = root_table.entries[vpn2];
        if (level2_pte & PTE_V) == 0 {
            return TranslateResult::PageFault; // level 2 PTE 无效
        }
        if level2_pte & (PTE_R | PTE_W | PTE_X) != 0 {
            // level 2 是叶子，计算物理地址
            let ppn = level2_pte >> PPN_SHIFT;
            let pa = (ppn << 12) | (va & ((1 << 30) - 1)); // 大页偏移：VPN[1:0] + 12 位页内偏移
            return TranslateResult::Ok(pa);
        }
        let level2_table = self.nodes.get(&(level2_pte >> PPN_SHIFT)).unwrap();

        // 处理level 1的PTE
        let level1_pte = level2_table.entries[vpn1];
        if (level1_pte & PTE_V) == 0 {
            return TranslateResult::PageFault; // level 1 PTE 无效
        }
        if level1_pte & (PTE_R | PTE_W | PTE_X) != 0 {
            // level 1 是叶子，计算物理地址
            let ppn = level1_pte >> PPN_SHIFT;
            let pa = (ppn << 12) | (va & ((1 << 21) - 1)); // 大页偏移：VPN[0] + 12 位页内偏移
            return TranslateResult::Ok(pa);
        }
        
        // 处理level 0的PTE
        let level0_pte = self.nodes.get(&(level1_pte >> PPN_SHIFT)).unwrap().entries[vpn0];
        if (level0_pte & PTE_V) == 0 {
            return TranslateResult::PageFault; // level 0 PTE 无效
        }
        if level0_pte & (PTE_R | PTE_W | PTE_X) == 0 {
            return TranslateResult::PageFault; // level 0 PTE 不是叶子
        }
        let ppn = level0_pte >> PPN_SHIFT;
        let pa = (ppn << 12) | (va & 0xFFF); // 页内偏移
        TranslateResult::Ok(pa)
    }
/// 对于translate中计算pa的疑问：
/// ppn是从root_ppn逐渐递增得到的。为什么ppn * PAGE_SIZE + offset正好是物理地址？ 如果这个ppn前有一些PTE table的页，是不是就错乱了？
/// 答：
/// 关键在于：页表页本来就是物理页的一种，不会“错乱”。
/// - PPN 是“物理页号”，全局编号，不区分“数据页”还是“页表页”。
/// - PPN * PAGE_SIZE 永远只是把“页号”换算成“这个页的物理起始地址”。
/// - + offset 是页内偏移，所以公式始终成立。
/// 
/// 你担心的“前面有很多页表页”其实没问题，因为：
/// 1. 页表页也占物理内存，这是正常的。
/// 2. 数据页和页表页都在同一个物理地址空间里，只是用途不同。
/// 3. 公式不关心用途，只做地址换算。




/// 关于 map_superpage 的问题：
/// 为什么va和pa要和2MB对齐，三级页表也没有要求对齐啊，解释原因。mega_size是一页的大小有2MB，也就是能存2MB的内容吗？va的vpn2,vpn1,vpn0和offset大小是否有改变
/// 答：
/// - 为什么 2MB 对齐
///     4KB 普通页是 L0 叶子，页内偏移是 12 位。
///     2MB 大页是 L1 叶子，此时 VPN[0] 不再参与查下一级，而是并入偏移。
///     所以偏移变成 21 位（VPN[0] 9 位 + 原 offset 12 位）。
///     这要求映射基址 va 和 pa 的低 21 位必须为 0，也就是 2MB 对齐。
/// - “三级页表没要求对齐”这句话怎么理解
///     三级结构本身一直在。
///     对齐要求是“当你在某层做叶子映射时”才有的：
///     L0 叶子 -> 4KB 对齐
///     L1 叶子 -> 2MB 对齐
///     L2 叶子 -> 1GB 对齐
/// - mega_size = 2MB 是什么
///     不是“页表页大小”。页表页始终 4KB。
///     它是“一条 L1 叶子 PTE 能覆盖的虚拟地址范围”，即 2MB 连续内存内容。
/// - va 的 vpn2/vpn1/vpn0/offset 会变吗
///     位宽不变：还是 9/9/9/12。
///     变的是翻译时的用法：
///     普通页（L0 叶子）：offset 用低 12 位
///     2MB 大页（L1 叶子）：offset 用低 21 位 (VPN[0]+12位offset)

    /// 建立大页映射（2MB superpage，在 level 1 设叶子 PTE）。
    ///
    /// 2MB = 512 × 4KB，对齐要求：va 和 pa 都必须 2MB 对齐。
    ///
    /// 与 map_page 类似，但只遍历到 level 1 就写入叶子 PTE。
    pub fn map_superpage(&mut self, va: u64, pa: u64, flags: u64) {
        let mega_size: u64 = (PAGE_SIZE * PT_ENTRIES) as u64; // 2MB
        assert_eq!(va % mega_size, 0, "va must be 2MB-aligned");
        assert_eq!(pa % mega_size, 0, "pa must be 2MB-aligned");

        // TODO: 实现大页映射
        //
        // 提示：大页映射与普通页映射类似，但只需要遍历到 level 1。
        // 你需要在 level 2 找到或创建中间页表节点，然后在 level 1 写入叶子 PTE。
        // 注意大页的物理页号计算方式与普通页相同（pa >> 12），
        // 但翻译时 offset 包含虚拟地址的低 21 位（VPN[0] 部分 + 12 位页内偏移）。
        
        let vpn2 = Self::extract_vpn(va, 2);
        let vpn1 = Self::extract_vpn(va, 1);

        // 从根页表开始
        let root_table = self.nodes.get(&self.root_ppn).unwrap();

        // 处理level 2的PTE
        let mut level2_pte = root_table.entries[vpn2];
        // 如果无效，分配新节点并更新PTE
        if level2_pte & PTE_V == 0 {
            let new_ppn = self.alloc_node();
            level2_pte = (new_ppn << PPN_SHIFT) | PTE_V;
            self.nodes.get_mut(&self.root_ppn).unwrap().entries[vpn2] = level2_pte;
        }

        // 处理level 1的PTE，直接写入物理页号和flags
        let level1_pte = ((pa >> 12) << PPN_SHIFT) | flags; // 物理页号和标志位
        self.nodes.get_mut(&(level2_pte >> PPN_SHIFT)).unwrap().entries[vpn1] = level1_pte;
    }
}

impl Default for Sv39PageTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_vpn() {
        // VA = 0x0000_003F_FFFF_F000 (最大的 39 位地址的页边界)
        // VPN[2] = 0xFF (bits 38:30)
        // VPN[1] = 0x1FF (bits 29:21)
        // VPN[0] = 0x1FF (bits 20:12)
        let va: u64 = 0x7FFFFFF000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0x1FF);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0x1FF);
    }

    #[test]
    fn test_extract_vpn_simple() {
        // VA = 0x00000000 + page 1 = 0x1000
        // VPN[2] = 0, VPN[1] = 0, VPN[0] = 1
        let va: u64 = 0x1000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 1);
    }

    #[test]
    fn test_extract_vpn_level2() {
        // VPN[2] = 1 means bit 30 set -> VA >= 0x40000000
        let va: u64 = 0x40000000;
        assert_eq!(Sv39PageTable::extract_vpn(va, 2), 1);
        assert_eq!(Sv39PageTable::extract_vpn(va, 1), 0);
        assert_eq!(Sv39PageTable::extract_vpn(va, 0), 0);
    }

    #[test]
    fn test_map_and_translate_single() {
        let mut pt = Sv39PageTable::new();
        // 映射：VA 0x1000 -> PA 0x80001000
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);

        let result = pt.translate(0x1000);
        assert_eq!(result, TranslateResult::Ok(0x80001000));
    }

    #[test]
    fn test_translate_with_offset() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x2000, 0x90000000, PTE_V | PTE_R | PTE_W);

        // 访问 VA 0x2ABC -> PA 应为 0x90000ABC
        let result = pt.translate(0x2ABC);
        assert_eq!(result, TranslateResult::Ok(0x90000ABC));
    }

    #[test]
    fn test_translate_page_fault() {
        let pt = Sv39PageTable::new();
        assert_eq!(pt.translate(0x1000), TranslateResult::PageFault);
    }

    #[test]
    fn test_multiple_mappings() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x0000_1000, 0x8000_1000, PTE_V | PTE_R);
        pt.map_page(0x0000_2000, 0x8000_5000, PTE_V | PTE_R | PTE_W);
        pt.map_page(0x0040_0000, 0x9000_0000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x1234), TranslateResult::Ok(0x80001234));
        assert_eq!(pt.translate(0x2000), TranslateResult::Ok(0x80005000));
        assert_eq!(pt.translate(0x400100), TranslateResult::Ok(0x90000100));
    }

    #[test]
    fn test_map_overwrite() {
        let mut pt = Sv39PageTable::new();
        pt.map_page(0x1000, 0x80001000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x80001000));

        pt.map_page(0x1000, 0x90002000, PTE_V | PTE_R);
        assert_eq!(pt.translate(0x1000), TranslateResult::Ok(0x90002000));
    }

    #[test]
    fn test_superpage_mapping() {
        let mut pt = Sv39PageTable::new();
        // 2MB 大页映射：VA 0x200000 -> PA 0x80200000
        pt.map_superpage(0x200000, 0x80200000, PTE_V | PTE_R | PTE_W);

        // 大页内不同偏移都应命中
        assert_eq!(pt.translate(0x200000), TranslateResult::Ok(0x80200000));
        assert_eq!(pt.translate(0x200ABC), TranslateResult::Ok(0x80200ABC));
        assert_eq!(pt.translate(0x2FF000), TranslateResult::Ok(0x802FF000));
    }

    #[test]
    fn test_superpage_and_normal_coexist() {
        let mut pt = Sv39PageTable::new();
        // 大页映射在第一个 2MB 区域
        pt.map_superpage(0x0, 0x80000000, PTE_V | PTE_R);
        // 普通页在不同的 VPN[2] 区域
        pt.map_page(0x40000000, 0x90001000, PTE_V | PTE_R);

        assert_eq!(pt.translate(0x100), TranslateResult::Ok(0x80000100));
        assert_eq!(pt.translate(0x40000000), TranslateResult::Ok(0x90001000));
    }
}
