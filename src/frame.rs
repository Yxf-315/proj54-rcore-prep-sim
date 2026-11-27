//! 栈式物理帧分配器（stack frame allocator）与 RAII 句柄。
//!
//! 对应 rCore-Tutorial 的 `os/src/mm/frame_allocator.rs`：
//! - 空闲帧从一个"水位线" `current` 向上增长分配；
//! - 被回收的帧压入 `recycled` 栈，下次分配优先复用；
//! - 释放不是手动调用 `free`，而是 `FrameTracker` 的 `Drop` 自动完成。
//!
//! 这三条正是本实验台想讲清楚的东西：**Rust 的所有权让"泄漏"和"双重释放"
//! 从运行期错误变成了类型系统里不可能发生的事**。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::memory::{hex_ppn, PhysAddr, PhysMemory, PAGE_SIZE};

/// 物理页帧号（PPN）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhysFrame(pub usize);

impl PhysFrame {
    pub const fn ppn(self) -> usize {
        self.0
    }

    /// 帧的起始物理地址。
    pub const fn start_pa(self) -> PhysAddr {
        PhysAddr(self.0 * PAGE_SIZE)
    }
}

impl fmt::Display for PhysFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex_ppn(self.0))
    }
}

/// 分配失败：物理内存耗尽。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfMemory;

impl fmt::Display for OutOfMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("物理帧耗尽（rCore 中对应 `FrameAllocator::alloc` 返回 None）")
    }
}

impl std::error::Error for OutOfMemory {}

/// 某一时刻分配器的统计快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameStats {
    /// 可用帧总数。
    pub total: usize,
    /// 当前被持有的帧数。
    pub allocated: usize,
    /// 当前空闲帧数。
    pub free: usize,
    /// 水位线曾经到达过的最高帧数（"曾经用过多少物理内存"）。
    pub high_water: usize,
    /// 历史峰值持有量。
    pub peak: usize,
    /// 回收栈中等待复用的帧数。
    pub recycled: usize,
}

/// 栈式物理帧分配器。
///
/// 真实 rCore 用一个 `UPSafeCell` 包住 `current` 与 `recycled`；
/// 用户态实验台用 `RefCell` 即可，行为完全一致。
pub struct FrameAllocator {
    start_ppn: usize,
    end_ppn: usize,
    /// 从未使用过的第一帧，只增不减。
    frontier: usize,
    /// 已回收、等待复用的帧（LIFO）。
    recycled: Vec<usize>,
    allocated: usize,
    peak_allocated: usize,
}

impl FrameAllocator {
    /// 用物理内存区间 `[base, end)` 构造分配器。
    pub fn new(base: PhysAddr, end: PhysAddr) -> Self {
        assert!(base.is_page_aligned(), "分配器基址必须页对齐");
        assert!(end.0 > base.0, "分配器区间必须非空");
        assert!(end.is_page_aligned(), "分配器末尾必须页对齐");
        let start_ppn = base.page_index();
        let end_ppn = end.page_index();
        Self {
            start_ppn,
            end_ppn,
            frontier: start_ppn,
            recycled: Vec::new(),
            allocated: 0,
            peak_allocated: 0,
        }
    }

    pub fn start_ppn(&self) -> usize {
        self.start_ppn
    }

    pub fn end_ppn(&self) -> usize {
        self.end_ppn
    }

    pub fn stats(&self) -> FrameStats {
        let total = self.end_ppn - self.start_ppn;
        FrameStats {
            total,
            allocated: self.allocated,
            free: total - self.allocated,
            high_water: self.frontier - self.start_ppn,
            peak: self.peak_allocated,
            recycled: self.recycled.len(),
        }
    }

    /// 分配一帧：优先复用回收栈，否则推进水位线。
    pub fn alloc(&mut self) -> Result<PhysFrame, OutOfMemory> {
        let ppn = if let Some(reused) = self.recycled.pop() {
            reused
        } else if self.frontier < self.end_ppn {
            let fresh = self.frontier;
            self.frontier += 1;
            fresh
        } else {
            return Err(OutOfMemory);
        };
        self.allocated += 1;
        self.peak_allocated = self.peak_allocated.max(self.allocated);
        Ok(PhysFrame(ppn))
    }

    /// 归还一帧。只应由 `FrameTracker::drop` 调用。
    fn dealloc(&mut self, ppn: usize) {
        debug_assert!(
            (self.start_ppn..self.end_ppn).contains(&ppn),
            "归还了不属于分配器的帧"
        );
        debug_assert!(self.allocated > 0, "归还次数多于分配次数");
        self.recycled.push(ppn);
        self.allocated -= 1;
    }

    /// 由页表释放子表时使用：直接归还一个已知属于本分配器的帧。
    pub fn free_frame(&mut self, ppn: usize) {
        self.dealloc(ppn);
    }

    /// 该帧当前是否被持有。
    pub fn is_allocated(&self, ppn: usize) -> bool {
        if !(self.start_ppn..self.end_ppn).contains(&ppn) {
            return false;
        }
        // 位于水位线以下、且不在回收栈里，才算被持有。
        ppn < self.frontier && !self.recycled.contains(&ppn)
    }
}

/// RAII 物理帧句柄：持有它 = 拥有这一帧；`drop` 它 = 归还这一帧。
///
/// 与 rCore 的 `FrameTracker` 同构，只是把 `Arc` 换成了单线程的 `Rc`。
pub struct FrameTracker {
    ppn: usize,
    allocator: Rc<RefCell<FrameAllocator>>,
}

impl FrameTracker {
    /// 从共享分配器申请一帧。
    pub fn alloc(allocator: &Rc<RefCell<FrameAllocator>>) -> Result<Self, OutOfMemory> {
        let frame = allocator.borrow_mut().alloc()?;
        Ok(Self {
            ppn: frame.ppn(),
            allocator: Rc::clone(allocator),
        })
    }

    pub const fn ppn(&self) -> usize {
        self.ppn
    }

    pub const fn frame(&self) -> PhysFrame {
        PhysFrame(self.ppn)
    }

    pub const fn start_pa(&self) -> PhysAddr {
        PhysAddr(self.ppn * PAGE_SIZE)
    }

    /// 把这一帧清零（分配页表帧后必须做）。
    pub fn clear(&self, mem: &mut PhysMemory) {
        mem.zero_frame(self.start_pa());
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        self.allocator.borrow_mut().dealloc(self.ppn);
    }
}

impl fmt::Debug for FrameTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FrameTracker(ppn={}, pa={})",
            hex_ppn(self.ppn),
            self.start_pa()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allocator(frames: usize) -> Rc<RefCell<FrameAllocator>> {
        let base = PhysAddr::new(0x8000_0000);
        let end = PhysAddr::new(0x8000_0000 + frames * PAGE_SIZE);
        Rc::new(RefCell::new(FrameAllocator::new(base, end)))
    }

    #[test]
    fn allocates_sequential_frames() {
        let fa = allocator(4);
        let a = FrameTracker::alloc(&fa).unwrap();
        let b = FrameTracker::alloc(&fa).unwrap();
        assert_eq!(b.ppn(), a.ppn() + 1);
        assert_eq!(fa.borrow().stats().allocated, 2);
    }

    #[test]
    fn drop_returns_frame_and_reuse_is_lifo() {
        let fa = allocator(4);
        let a = FrameTracker::alloc(&fa).unwrap();
        let b = FrameTracker::alloc(&fa).unwrap();
        let a_ppn = a.ppn();
        let b_ppn = b.ppn();
        drop(a);
        drop(b);
        assert_eq!(fa.borrow().stats().allocated, 0);
        // 回收栈是 LIFO：最后释放的 b 会被最先复用。
        let c = FrameTracker::alloc(&fa).unwrap();
        assert_eq!(c.ppn(), b_ppn);
        let d = FrameTracker::alloc(&fa).unwrap();
        assert_eq!(d.ppn(), a_ppn);
    }

    #[test]
    fn exhaustion_reports_oom() {
        let fa = allocator(2);
        let _a = FrameTracker::alloc(&fa).unwrap();
        let _b = FrameTracker::alloc(&fa).unwrap();
        assert!(FrameTracker::alloc(&fa).is_err());
        assert_eq!(fa.borrow().stats().free, 0);
    }

    #[test]
    fn high_water_never_shrinks() {
        let fa = allocator(8);
        {
            let _a = FrameTracker::alloc(&fa).unwrap();
            let _b = FrameTracker::alloc(&fa).unwrap();
        }
        let stats = fa.borrow().stats();
        assert_eq!(stats.allocated, 0);
        assert_eq!(stats.high_water, 2);
        assert_eq!(stats.recycled, 2);
    }

    #[test]
    fn leak_keeps_frame_allocated() {
        let fa = allocator(4);
        let t = FrameTracker::alloc(&fa).unwrap();
        std::mem::forget(t);
        assert_eq!(fa.borrow().stats().allocated, 1);
    }
}
