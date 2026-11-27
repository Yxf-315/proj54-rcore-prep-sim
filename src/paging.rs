//! Sv39 三级页表与地址翻译。
//!
//! 对应 rCore-Tutorial 的 `os/src/mm/page_table.rs`：
//! RISC-V Sv39 把 39 位虚拟地址切成 `VPN[2] | VPN[1] | VPN[0] | offset`，
//! 每级页表 512 项、每项 8 字节，页表项里存的是下一级页表的**物理页帧号**。
//!
//! 本模块把页表直接放在 [`PhysMemory`] 里，所以 [`PageTable::translate`]
//! 是逐级读内存完成的，和硬件 MMU 走的是同一条路：
//! 任何一级 PTE 无效就是缺页，任何一级权限不满足就是权限异常。

use core::fmt;

use crate::frame::{FrameAllocator, PhysFrame};
use crate::memory::{hex64, PhysAddr, PhysMemory, VirtAddr, PAGE_SIZE, PTE_PER_TABLE, SV39_LEVELS};

/// PTE 有效位。
pub const PTE_V: u64 = 1 << 0;
/// 可读。
pub const PTE_R: u64 = 1 << 1;
/// 可写。
pub const PTE_W: u64 = 1 << 2;
/// 可执行。
pub const PTE_X: u64 = 1 << 3;
/// 用户态可访问。
pub const PTE_U: u64 = 1 << 4;
/// 全局映射。
pub const PTE_G: u64 = 1 << 5;
/// 硬件已访问。
pub const PTE_A: u64 = 1 << 6;
/// 硬件已写过（脏页）。
pub const PTE_D: u64 = 1 << 7;

/// 叶子页表项：R/W/X 至少一位置位。
const PTE_LEAF: u64 = PTE_R | PTE_W | PTE_X;

/// 物理页帧号在 PTE 中的位宽（Sv39 为 44 位）。
const PPN_MASK: u64 = (1 << 44) - 1;

/// 一个 64 位 Sv39 页表项。
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    /// 由 PPN 与权限位构造 PTE。`V` 位需要调用者显式包含在 `flags` 里。
    pub const fn new(ppn: usize, flags: u64) -> Self {
        Self(((ppn as u64) & PPN_MASK) << 10 | (flags & 0x3ff))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn flags(self) -> u64 {
        self.0 & 0x3ff
    }

    pub const fn is_valid(self) -> bool {
        self.0 & PTE_V != 0
    }

    /// 是否是叶子项：R/W/X 至少一个非零。非叶子项只指向下一级页表。
    pub const fn is_leaf(self) -> bool {
        self.0 & PTE_LEAF != 0
    }

    pub const fn ppn(self) -> usize {
        ((self.0 >> 10) & PPN_MASK) as usize
    }

    /// 非叶子项指向的下一级页表物理地址。
    pub const fn next_table_pa(self) -> PhysAddr {
        PhysAddr(self.ppn() * PAGE_SIZE)
    }

    pub const fn readable(self) -> bool {
        self.0 & PTE_R != 0
    }

    pub const fn writable(self) -> bool {
        self.0 & PTE_W != 0
    }

    pub const fn executable(self) -> bool {
        self.0 & PTE_X != 0
    }

    pub const fn user(self) -> bool {
        self.0 & PTE_U != 0
    }

    /// 人类可读的权限串，例如 `V R W -`。
    pub fn perm_string(self) -> String {
        let mut out = String::new();
        out.push(if self.is_valid() { 'V' } else { '-' });
        out.push(' ');
        out.push(if self.readable() { 'R' } else { '-' });
        out.push(' ');
        out.push(if self.writable() { 'W' } else { '-' });
        out.push(' ');
        out.push(if self.executable() { 'X' } else { '-' });
        out
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PTE({}, [{}])", hex64(self.0), self.perm_string())
    }
}

/// 一次访存的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    Execute,
}

impl AccessType {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "r" | "read" | "读" => Some(Self::Read),
            "w" | "write" | "写" => Some(Self::Write),
            "x" | "exec" | "执行" => Some(Self::Execute),
            _ => None,
        }
    }
}

impl fmt::Display for AccessType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AccessType::Read => "读",
            AccessType::Write => "写",
            AccessType::Execute => "执行",
        })
    }
}

/// 页表遍历结束后的一条记录，用于把三级页表"摊开"给用户看。
#[derive(Debug, Clone, Copy)]
pub struct WalkStep {
    pub level: usize,
    pub index: usize,
    pub pte_addr: PhysAddr,
    pub raw: u64,
    pub valid: bool,
    pub leaf: bool,
}

/// 缺页 / 权限异常。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFault {
    /// 页表遍历到第 `level` 级时 PTE 无效：rCore 里对应"缺页"。
    NotMapped { level: usize, va: VirtAddr },
    /// PTE 有效，但 R/W/X 不允许本次访问。
    PermissionDenied {
        level: usize,
        va: VirtAddr,
        access: AccessType,
    },
    /// 页表结构损坏：非叶子项指向了非法内存。
    Corrupt { va: VirtAddr },
}

impl fmt::Display for PageFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PageFault::NotMapped { level, va } => write!(
                f,
                "缺页异常：VA={} 在第 {level} 级页表（L{level}）处 PTE 无效（V=0）",
                va
            ),
            PageFault::PermissionDenied { level, va, access } => write!(
                f,
                "权限异常：VA={} 尝试{}，但 L{level} 的 PTE 权限位不允许",
                va, access
            ),
            PageFault::Corrupt { va } => {
                write!(f, "页表损坏：VA={} 的遍历路径指向了非法内存", va)
            }
        }
    }
}

impl std::error::Error for PageFault {}

/// 建立映射时的错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    /// 页表帧耗尽。
    OutOfMemory,
    /// 该虚拟页已经映射过。
    AlreadyMapped { va: VirtAddr },
    /// 该虚拟页没有映射。
    NotMapped { va: VirtAddr },
    /// 地址没有按 4 KiB 对齐。
    Misaligned { va: VirtAddr, pa: PhysAddr },
    /// 物理地址不在模拟内存范围内。
    BadPhysAddr { pa: PhysAddr },
    /// 页表结构损坏。
    Corrupt { va: VirtAddr },
}

impl fmt::Display for MapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapError::OutOfMemory => f.write_str("建立映射失败：物理帧耗尽"),
            MapError::AlreadyMapped { va } => {
                write!(
                    f,
                    "VA={va} 已经映射过（rCore 的 `map` 遇到重复映射会 panic）"
                )
            }
            MapError::NotMapped { va } => write!(f, "VA={va} 没有建立映射，无法 unmap"),
            MapError::Misaligned { va, pa } => {
                write!(f, "地址必须 4 KiB 对齐：VA={va}，PA={pa}")
            }
            MapError::BadPhysAddr { pa } => write!(f, "物理地址 {pa} 不在模拟内存范围内"),
            MapError::Corrupt { va } => write!(f, "VA={va} 的页表结构已损坏"),
        }
    }
}

impl std::error::Error for MapError {}

impl From<crate::frame::OutOfMemory> for MapError {
    fn from(_: crate::frame::OutOfMemory) -> Self {
        MapError::OutOfMemory
    }
}

/// 一棵 Sv39 页表（持有根页帧）。
#[derive(Debug)]
pub struct PageTable {
    root: PhysFrame,
}

impl PageTable {
    /// 申请一帧作为根页表，并清零。
    pub fn new(mem: &mut PhysMemory, fa: &mut FrameAllocator) -> Result<Self, MapError> {
        let root = fa.alloc()?;
        let ok = mem.zero_frame(root.start_pa());
        debug_assert!(ok, "根页表帧必须位于物理内存中");
        Ok(Self { root })
    }

    pub const fn root(&self) -> PhysFrame {
        self.root
    }

    /// satp 寄存器的值（MODE=8，即 Sv39）。
    pub fn satp(&self) -> u64 {
        (8u64 << 60) | (self.root.ppn() as u64)
    }

    /// 建立 4 KiB 页映射；中间级页表缺失时自动申请。
    pub fn map(
        &mut self,
        va: VirtAddr,
        pa: PhysAddr,
        flags: u64,
        mem: &mut PhysMemory,
        fa: &mut FrameAllocator,
    ) -> Result<(), MapError> {
        if !va.is_page_aligned() || !pa.is_page_aligned() {
            return Err(MapError::Misaligned { va, pa });
        }
        if !mem.contains(pa) {
            return Err(MapError::BadPhysAddr { pa });
        }

        let mut table_pa = self.root.start_pa();
        for level in (0..SV39_LEVELS).rev() {
            let index = va.index(level);
            let pte_pa = PhysAddr(table_pa.raw() + index * 8);
            let Some(raw) = mem.read_u64(pte_pa) else {
                return Err(MapError::Corrupt { va });
            };
            let pte = PageTableEntry(raw);

            if pte.is_valid() && pte.is_leaf() {
                return Err(MapError::AlreadyMapped { va });
            }

            if level == 0 {
                let leaf = PageTableEntry::new(pa.page_index(), flags | PTE_V);
                mem.write_u64(pte_pa, leaf.raw());
                return Ok(());
            }

            if pte.is_valid() {
                table_pa = pte.next_table_pa();
            } else {
                let child = fa.alloc().map_err(MapError::from)?;
                let child_pa = child.start_pa();
                if !mem.zero_frame(child_pa) || !mem.contains(child_pa) {
                    return Err(MapError::Corrupt { va });
                }
                let link = PageTableEntry::new(child.ppn(), PTE_V);
                mem.write_u64(pte_pa, link.raw());
                table_pa = child_pa;
            }
        }

        Err(MapError::Corrupt { va })
    }

    /// 拆除 4 KiB 映射，返回被映射的物理帧号。
    ///
    /// 注意：中间级页表不会在这里回收（rCore 的 `unmap` 也不做），
    /// 整体回收请调用 [`PageTable::release`]。
    pub fn unmap(&mut self, va: VirtAddr, mem: &mut PhysMemory) -> Result<usize, MapError> {
        let mut table_pa = self.root.start_pa();
        for level in (0..SV39_LEVELS).rev() {
            let index = va.index(level);
            let pte_pa = PhysAddr(table_pa.raw() + index * 8);
            let Some(raw) = mem.read_u64(pte_pa) else {
                return Err(MapError::Corrupt { va });
            };
            let pte = PageTableEntry(raw);
            if !pte.is_valid() {
                return Err(MapError::NotMapped { va });
            }
            if level == 0 {
                if !pte.is_leaf() {
                    return Err(MapError::Corrupt { va });
                }
                mem.write_u64(pte_pa, 0);
                return Ok(pte.ppn());
            }
            if pte.is_leaf() {
                // 大页：整页拆除。
                mem.write_u64(pte_pa, 0);
                return Ok(pte.ppn());
            }
            table_pa = pte.next_table_pa();
        }
        Err(MapError::Corrupt { va })
    }

    /// 逐级遍历，返回每一级的 PTE 情况（用于 `walk` 命令与教学展示）。
    pub fn walk(&self, va: VirtAddr, mem: &PhysMemory) -> Vec<WalkStep> {
        let mut steps = Vec::new();
        let mut table_pa = self.root.start_pa();
        for level in (0..SV39_LEVELS).rev() {
            let index = va.index(level);
            let pte_pa = PhysAddr(table_pa.raw() + index * 8);
            let raw = mem.read_u64(pte_pa).unwrap_or(0);
            let pte = PageTableEntry(raw);
            let leaf = pte.is_valid() && pte.is_leaf();
            steps.push(WalkStep {
                level,
                index,
                pte_addr: pte_pa,
                raw,
                valid: pte.is_valid(),
                leaf,
            });
            if !pte.is_valid() || leaf {
                break;
            }
            table_pa = pte.next_table_pa();
        }
        steps
    }

    /// 地址翻译：成功返回物理地址，失败返回具体的缺页 / 权限原因。
    ///
    /// 成功时会像硬件一样置上 A（已访问）位，写访问还会置 D（脏）位。
    pub fn translate(
        &self,
        va: VirtAddr,
        access: AccessType,
        mem: &mut PhysMemory,
    ) -> Result<PhysAddr, PageFault> {
        let mut table_pa = self.root.start_pa();
        for level in (0..SV39_LEVELS).rev() {
            let index = va.index(level);
            let pte_pa = PhysAddr(table_pa.raw() + index * 8);
            let raw = mem.read_u64(pte_pa).ok_or(PageFault::Corrupt { va })?;
            let pte = PageTableEntry(raw);

            if !pte.is_valid() {
                return Err(PageFault::NotMapped { level, va });
            }

            if pte.is_leaf() {
                let allowed = match access {
                    AccessType::Read => pte.readable(),
                    AccessType::Write => pte.writable(),
                    AccessType::Execute => pte.executable(),
                };
                if !allowed {
                    return Err(PageFault::PermissionDenied { level, va, access });
                }
                let bits = crate::memory::PAGE_SHIFT + 9 * level;
                let offset = va.raw() & ((1usize << bits) - 1);
                let pa = PhysAddr((pte.ppn() << crate::memory::PAGE_SHIFT) + offset);

                let mut updated = raw | PTE_A;
                if access == AccessType::Write {
                    updated |= PTE_D;
                }
                if updated != raw {
                    mem.write_u64(pte_pa, updated);
                }
                return Ok(pa);
            }

            table_pa = pte.next_table_pa();
        }
        Err(PageFault::Corrupt { va })
    }

    /// 递归释放整棵页表树（含根帧）。
    pub fn release(self, mem: &mut PhysMemory, fa: &mut FrameAllocator) {
        free_table(self.root.ppn(), mem, fa);
    }
}

fn free_table(ppn: usize, mem: &mut PhysMemory, fa: &mut FrameAllocator) {
    let base = PhysAddr(ppn * PAGE_SIZE);
    for i in 0..PTE_PER_TABLE {
        let pte_addr = PhysAddr(base.raw() + i * 8);
        let Some(raw) = mem.read_u64(pte_addr) else {
            continue;
        };
        let pte = PageTableEntry(raw);
        if pte.is_valid() && !pte.is_leaf() {
            free_table(pte.ppn(), mem, fa);
        }
    }
    fa.free_frame(ppn);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameAllocator;

    struct Fixture {
        mem: PhysMemory,
        fa: FrameAllocator,
        pt: PageTable,
    }

    fn fixture() -> Fixture {
        let base = PhysAddr::new(0x8000_0000);
        let mut mem = PhysMemory::new(base.raw(), 64 * PAGE_SIZE);
        let mut fa = FrameAllocator::new(base, PhysAddr::new(base.raw() + 64 * PAGE_SIZE));
        let pt = PageTable::new(&mut mem, &mut fa).unwrap();
        Fixture { mem, fa, pt }
    }

    fn alloc_frame(f: &mut Fixture) -> PhysAddr {
        let frame = f.fa.alloc().unwrap();
        frame.start_pa()
    }

    #[test]
    fn map_then_translate() {
        let mut f = fixture();
        let va = VirtAddr::new(0x1000).unwrap();
        let pa = alloc_frame(&mut f);
        f.pt.map(va, pa, PTE_R | PTE_W | PTE_U, &mut f.mem, &mut f.fa)
            .unwrap();
        let got = f.pt.translate(va, AccessType::Read, &mut f.mem).unwrap();
        assert_eq!(got, pa);
        let got =
            f.pt.translate(
                VirtAddr::new(0x1abc).unwrap(),
                AccessType::Write,
                &mut f.mem,
            )
            .unwrap();
        assert_eq!(got, PhysAddr::new(pa.raw() + 0xabc));
    }

    #[test]
    fn walk_shows_three_levels() {
        let mut f = fixture();
        let va = VirtAddr::new(0x4000_0000).unwrap();
        let pa = alloc_frame(&mut f);
        f.pt.map(va, pa, PTE_R | PTE_W, &mut f.mem, &mut f.fa)
            .unwrap();
        let steps = f.pt.walk(va, &f.mem);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].level, 2);
        assert_eq!(steps[0].index, 1);
        assert!(steps.iter().all(|s| s.valid));
        assert!(steps[2].leaf);
    }

    #[test]
    fn unmapped_page_faults_at_level() {
        let mut f = fixture();
        let va = VirtAddr::new(0x2000).unwrap();
        let err =
            f.pt.translate(va, AccessType::Read, &mut f.mem)
                .unwrap_err();
        assert_eq!(err, PageFault::NotMapped { level: 2, va });
    }

    #[test]
    fn permission_violation_is_reported() {
        let mut f = fixture();
        let va = VirtAddr::new(0x3000).unwrap();
        let pa = alloc_frame(&mut f);
        f.pt.map(va, pa, PTE_R | PTE_X, &mut f.mem, &mut f.fa)
            .unwrap();
        let err =
            f.pt.translate(va, AccessType::Write, &mut f.mem)
                .unwrap_err();
        assert_eq!(
            err,
            PageFault::PermissionDenied {
                level: 0,
                va,
                access: AccessType::Write
            }
        );
    }

    #[test]
    fn double_map_is_rejected() {
        let mut f = fixture();
        let va = VirtAddr::new(0x5000).unwrap();
        let pa = alloc_frame(&mut f);
        f.pt.map(va, pa, PTE_R | PTE_W, &mut f.mem, &mut f.fa)
            .unwrap();
        let err =
            f.pt.map(va, pa, PTE_R | PTE_W, &mut f.mem, &mut f.fa)
                .unwrap_err();
        assert_eq!(err, MapError::AlreadyMapped { va });
    }

    #[test]
    fn unmap_returns_ppn_and_clears_pte() {
        let mut f = fixture();
        let va = VirtAddr::new(0x6000).unwrap();
        let pa = alloc_frame(&mut f);
        f.pt.map(va, pa, PTE_R, &mut f.mem, &mut f.fa).unwrap();
        let ppn = f.pt.unmap(va, &mut f.mem).unwrap();
        assert_eq!(ppn, pa.page_index());
        assert!(f.pt.translate(va, AccessType::Read, &mut f.mem).is_err());
    }

    #[test]
    fn access_sets_a_and_d_bits() {
        let mut f = fixture();
        let va = VirtAddr::new(0x7000).unwrap();
        let pa = alloc_frame(&mut f);
        f.pt.map(va, pa, PTE_R | PTE_W, &mut f.mem, &mut f.fa)
            .unwrap();
        f.pt.translate(va, AccessType::Write, &mut f.mem).unwrap();
        let steps = f.pt.walk(va, &f.mem);
        let leaf = steps.last().unwrap();
        assert_ne!(leaf.raw & PTE_A, 0);
        assert_ne!(leaf.raw & PTE_D, 0);
    }

    #[test]
    fn release_returns_all_table_frames() {
        let mut f = fixture();
        let before = f.fa.stats().allocated;
        let va = VirtAddr::new(0x8000).unwrap();
        let pa = alloc_frame(&mut f);
        f.pt.map(va, pa, PTE_R, &mut f.mem, &mut f.fa).unwrap();
        let after_map = f.fa.stats().allocated;
        assert!(after_map > before);
        f.pt.release(&mut f.mem, &mut f.fa);
        // 只剩那一个数据帧（它没有 Tracker 管理）。
        assert_eq!(f.fa.stats().allocated, 1);
    }

    #[test]
    fn satp_encodes_sv39_mode() {
        let f = fixture();
        assert_eq!(f.pt.satp() >> 60, 8);
    }
}
