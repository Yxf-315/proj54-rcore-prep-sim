//! 物理内存与地址抽象。
//!
//! 对应 rCore-Tutorial 的 `os/src/mm/address.rs`：
//! 内核里 `PhysAddr` / `VirtAddr` 只是对 `usize` 的 newtype，真正的检查靠
//! 页表项与硬件。这里为了让用户态实验能"看见"错误，额外做了 Sv39 合法性检查。

use core::fmt;

/// 页大小：4 KiB。
pub const PAGE_SIZE: usize = 4096;
/// 页内偏移位数。
pub const PAGE_SHIFT: usize = 12;
/// 一个页表包含 512 个页表项。
pub const PTE_PER_TABLE: usize = 512;
/// Sv39 的页表级数。
pub const SV39_LEVELS: usize = 3;
/// Sv39 低半区（canonical）虚拟地址上界（不含）。`0x0000_003f_ffff_ffff` 是最后一个合法值。
pub const SV39_MAX_VA: usize = 1 << 38;

/// 把整数格式化成带下划线的十六进制，例如 `0x8020_0000`。
///
/// 内核日志里这种写法很常见，阅读页表时比一长串数字友好得多。
pub fn hex(v: usize) -> String {
    let digits = format!("{v:x}");
    let width = digits.len().max(8);
    let width = width.div_ceil(4) * 4;
    let padded = format!("{digits:0>width$}");
    let mut out = String::from("0x");
    for (i, ch) in padded.chars().enumerate() {
        if i > 0 && (padded.len() - i) % 4 == 0 {
            out.push('_');
        }
        out.push(ch);
    }
    out
}

/// 把 64 位数格式化成带下划线的十六进制。
pub fn hex64(v: u64) -> String {
    let digits = format!("{v:x}");
    let width = digits.len().max(16);
    let width = width.div_ceil(4) * 4;
    let padded = format!("{digits:0>width$}");
    let mut out = String::from("0x");
    for (i, ch) in padded.chars().enumerate() {
        if i > 0 && (padded.len() - i) % 4 == 0 {
            out.push('_');
        }
        out.push(ch);
    }
    out
}

/// 页帧号（PPN）的短格式：不补齐到 8 位，只按 4 位一组分隔。
///
/// 例如 `0x80000` → `0x8_0000`，避免和 32 位物理地址混淆。
pub fn hex_ppn(v: usize) -> String {
    let digits = format!("{v:x}");
    let len = digits.len();
    let mut out = String::from("0x");
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (len - i) % 4 == 0 {
            out.push('_');
        }
        out.push(ch);
    }
    out
}

/// 物理地址。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    pub const fn new(raw: usize) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> usize {
        self.0
    }

    /// 所在物理帧号（PPN）。
    pub const fn page_index(self) -> usize {
        self.0 / PAGE_SIZE
    }

    /// 页内偏移。
    pub const fn offset(self) -> usize {
        self.0 % PAGE_SIZE
    }

    pub const fn is_page_aligned(self) -> bool {
        self.0 % PAGE_SIZE == 0
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysAddr({})", hex(self.0))
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(self.0))
    }
}

/// 虚拟地址（Sv39）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    /// 构造一个合法（低半区 canonical）的 Sv39 虚拟地址。
    pub fn new(raw: usize) -> Result<Self, AddrError> {
        if raw >= SV39_MAX_VA {
            Err(AddrError::NotCanonical(raw))
        } else {
            Ok(Self(raw))
        }
    }

    pub const fn raw(self) -> usize {
        self.0
    }

    /// 第 `level` 级页表的 9 位索引：level=2/1/0 对应 VPN[2]/VPN[1]/VPN[0]。
    pub const fn index(self, level: usize) -> usize {
        (self.0 >> (PAGE_SHIFT + 9 * level)) & 0x1ff
    }

    pub const fn offset(self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }

    pub const fn is_page_aligned(self) -> bool {
        self.0 % PAGE_SIZE == 0
    }

    /// 页基址（清掉低 12 位）。
    pub const fn page_base(self) -> usize {
        self.0 & !(PAGE_SIZE - 1)
    }
}

impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtAddr({})", hex(self.0))
    }
}

impl fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(self.0))
    }
}

/// 地址错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddrError {
    /// 超出 Sv39 低半区范围（高半区需要符号扩展，本实验台暂不支持）。
    NotCanonical(usize),
}

impl fmt::Display for AddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddrError::NotCanonical(v) => write!(
                f,
                "{} 不是合法的 Sv39 低半区虚拟地址（上界为 {}）",
                hex(*v),
                hex(SV39_MAX_VA - 1)
            ),
        }
    }
}

impl std::error::Error for AddrError {}

/// 一块线性物理内存。真实硬件由内存控制器提供，这里用 `Vec<u8>` 模拟。
///
/// 页表本身也放在这块内存里，因此地址翻译是"真的去读内存"，
/// 而不是查一张预先写好的 HashMap。
pub struct PhysMemory {
    base: usize,
    data: Vec<u8>,
}

impl PhysMemory {
    /// `base` 必须页对齐，`size` 必须是页大小的整数倍。
    pub fn new(base: usize, size: usize) -> Self {
        assert!(base % PAGE_SIZE == 0, "物理内存基址必须页对齐");
        assert!(
            size > 0 && size % PAGE_SIZE == 0,
            "物理内存大小必须是页的整数倍"
        );
        Self {
            base,
            data: vec![0; size],
        }
    }

    pub const fn base(&self) -> PhysAddr {
        PhysAddr(self.base)
    }

    pub fn end(&self) -> PhysAddr {
        PhysAddr(self.base + self.data.len())
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 总帧数。
    pub fn frames(&self) -> usize {
        self.data.len() / PAGE_SIZE
    }

    fn offset(&self, pa: PhysAddr) -> Option<usize> {
        let end = self.base + self.data.len();
        if pa.0 < self.base || pa.0 >= end {
            None
        } else {
            Some(pa.0 - self.base)
        }
    }

    pub fn contains(&self, pa: PhysAddr) -> bool {
        self.offset(pa).is_some()
    }

    /// 读 8 字节（未对齐也允许，模拟总线的字节访问语义）。
    pub fn read_u64(&self, pa: PhysAddr) -> Option<u64> {
        let off = self.offset(pa)?;
        if off + 8 > self.data.len() {
            return None;
        }
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.data[off..off + 8]);
        Some(u64::from_le_bytes(buf))
    }

    pub fn write_u64(&mut self, pa: PhysAddr, value: u64) -> bool {
        let Some(off) = self.offset(pa) else {
            return false;
        };
        if off + 8 > self.data.len() {
            return false;
        }
        self.data[off..off + 8].copy_from_slice(&value.to_le_bytes());
        true
    }

    pub fn read_byte(&self, pa: PhysAddr) -> Option<u8> {
        self.offset(pa).map(|off| self.data[off])
    }

    pub fn write_byte(&mut self, pa: PhysAddr, value: u8) -> bool {
        match self.offset(pa) {
            Some(off) => {
                self.data[off] = value;
                true
            }
            None => false,
        }
    }

    /// 把一整帧清零（rCore 分配页表帧后必须清零，否则残留的 PTE 会被误用）。
    pub fn zero_frame(&mut self, pa: PhysAddr) -> bool {
        let Some(off) = self.offset(pa) else {
            return false;
        };
        let end = off + PAGE_SIZE;
        if end > self.data.len() {
            return false;
        }
        self.data[off..end].fill(0);
        true
    }

    /// 从某物理地址开始写入一段字节（用于把内核数据放进"内存"）。
    pub fn write_bytes(&mut self, pa: PhysAddr, bytes: &[u8]) -> bool {
        let Some(off) = self.offset(pa) else {
            return false;
        };
        if off + bytes.len() > self.data.len() {
            return false;
        }
        self.data[off..off + bytes.len()].copy_from_slice(bytes);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_formatting_groups_digits() {
        assert_eq!(hex(0x8020_0000), "0x8020_0000");
        assert_eq!(hex(0x1000), "0x0000_1000");
        assert_eq!(hex(0), "0x0000_0000");
    }

    #[test]
    fn virt_index_extracts_vpn() {
        // 0x0000_0000_0000_1000 -> VPN[2]=0, VPN[1]=0, VPN[0]=1
        let va = VirtAddr::new(0x1000).unwrap();
        assert_eq!(va.index(2), 0);
        assert_eq!(va.index(1), 0);
        assert_eq!(va.index(0), 1);
        assert_eq!(va.offset(), 0);
        // 0x0000_0000_4000_0000 -> VPN[2]=1
        let va = VirtAddr::new(0x4000_0000).unwrap();
        assert_eq!(va.index(2), 1);
        assert_eq!(va.index(1), 0);
    }

    #[test]
    fn sv39_upper_bound() {
        assert!(VirtAddr::new(0x3fff_ffff).is_ok());
        assert!(VirtAddr::new(SV39_MAX_VA).is_err());
    }

    #[test]
    fn memory_bounds() {
        let mut mem = PhysMemory::new(0x8000_0000, 2 * PAGE_SIZE);
        assert!(mem.write_u64(PhysAddr::new(0x8000_0000), 42));
        assert_eq!(mem.read_u64(PhysAddr::new(0x8000_0000)), Some(42));
        assert!(!mem.write_u64(PhysAddr::new(0x8000_2000 - 4), 1));
        assert!(!mem.contains(PhysAddr::new(0x7fff_f000)));
    }
}
