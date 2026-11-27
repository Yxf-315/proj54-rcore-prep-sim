//! 所有权演示脚本。
//!
//! 每个演示都先用真实的 [`FrameTracker`] 分配/回收物理帧，
//! 再把"编译器会拦住什么"用代码片段讲出来。演示结束时分配器状态会恢复原样
//! （`leak` 例外——泄漏就是要让你看见）。

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use crate::frame::{FrameAllocator, FrameTracker};
use crate::memory::hex_ppn;

/// 可用演示的名字与一句话说明。
pub const NAMES: &[(&str, &str)] = &[
    ("move", "所有权转移：句柄只能有一个主人"),
    ("clone", "Rc 共享所有权：引用计数归零才回收"),
    ("raii", "RAII：Drop 自动归还物理帧"),
    ("leak", "mem::forget 造成真实的帧泄漏"),
    ("borrow", "借用规则如何保护页表"),
    ("dangling", "悬垂指针在 Rust 里是编译错误"),
    ("double-free", "双重释放为什么写不出来"),
];

pub fn demo(name: &str, fa: &Rc<RefCell<FrameAllocator>>) -> Result<String, String> {
    match name {
        "move" => demo_move(fa),
        "clone" => demo_clone(fa),
        "raii" => demo_raii(fa),
        "leak" => demo_leak(fa),
        "borrow" => Ok(text_borrow()),
        "dangling" => Ok(text_dangling()),
        "double-free" => Ok(text_double_free()),
        other => Err(format!(
            "未知演示 `{other}`，可选：move / clone / raii / leak / borrow / dangling / double-free"
        )),
    }
}

fn demo_move(fa: &Rc<RefCell<FrameAllocator>>) -> Result<String, String> {
    let mut out = String::from("【move】所有权转移：一块物理帧只能有一个主人\n\n");
    let before = fa.borrow().stats();

    let frame = FrameTracker::alloc(fa).map_err(|e| e.to_string())?;
    let ppn = frame.ppn();
    writeln!(
        out,
        "  let frame = FrameTracker::alloc(&fa)?;   // 拿到 PPN {}",
        hex_ppn(ppn)
    )
    .unwrap();

    let moved = frame;
    writeln!(
        out,
        "  let moved = frame;                        // 所有权转移，frame 立刻失效"
    )
    .unwrap();
    writeln!(
        out,
        "  // println!(\"{{:?}}\", frame);            // ← E0382: use of moved value"
    )
    .unwrap();
    writeln!(
        out,
        "\n  转移之后：moved 持有 PPN {}，分配器持有 {} 帧。",
        hex_ppn(ppn),
        fa.borrow().stats().allocated
    )
    .unwrap();

    drop(moved);
    writeln!(
        out,
        "  drop(moved);                              // 离开作用域，帧自动归还"
    )
    .unwrap();
    writeln!(
        out,
        "  分配器持有 {} 帧（演示前 {} 帧）。",
        fa.borrow().stats().allocated,
        before.allocated
    )
    .unwrap();
    writeln!(
        out,
        "\n对照 rCore：`PageTable::new` 会**拿走**一个 `FrameTracker` 作为根页表。\n\
         交出之后调用者手里就没有句柄了，所以绝不可能出现两个所有者各 free 一次。"
    )
    .unwrap();
    Ok(out)
}

fn demo_clone(fa: &Rc<RefCell<FrameAllocator>>) -> Result<String, String> {
    let mut out = String::from("【Rc】共享所有权：最后一个引用消失时才回收\n\n");
    let before = fa.borrow().stats().allocated;

    let a = Rc::new(FrameTracker::alloc(fa).map_err(|e| e.to_string())?);
    let ppn = a.ppn();
    writeln!(
        out,
        "  let a = Rc::new(FrameTracker::alloc(&fa)?);  // PPN {}，strong_count = {}",
        hex_ppn(ppn),
        Rc::strong_count(&a)
    )
    .unwrap();

    let b = Rc::clone(&a);
    writeln!(
        out,
        "  let b = Rc::clone(&a);                       // strong_count = {}",
        Rc::strong_count(&a)
    )
    .unwrap();
    writeln!(
        out,
        "  两块\"变量\"指向同一帧，但物理帧仍然只有一份。分配器持有 {} 帧。",
        fa.borrow().stats().allocated
    )
    .unwrap();

    drop(b);
    writeln!(
        out,
        "\n  drop(b);                                     // strong_count = {}，帧还没回收",
        Rc::strong_count(&a)
    )
    .unwrap();
    writeln!(out, "  分配器持有 {} 帧。", fa.borrow().stats().allocated).unwrap();

    drop(a);
    writeln!(
        out,
        "  drop(a);                                     // strong_count = 0，此刻才真正 free",
    )
    .unwrap();
    writeln!(
        out,
        "  分配器持有 {} 帧（演示前 {} 帧）。",
        fa.borrow().stats().allocated,
        before
    )
    .unwrap();
    writeln!(
        out,
        "\n对照 rCore：fork 之后的写时复制（COW）会让父子进程共享同一批物理页。\n\
         用 `Arc<FrameTracker>` 记录引用数，最后一个持有者析构时才归还帧 —— 这就是\n\
         Lab3 里 COW 实验能够安全实现的基础。"
    )
    .unwrap();
    Ok(out)
}

fn demo_raii(fa: &Rc<RefCell<FrameAllocator>>) -> Result<String, String> {
    let mut out = String::from("【RAII】Drop 自动归还：没有显式 free，也不会忘记 free\n\n");
    let before = fa.borrow().stats();

    writeln!(
        out,
        "  // 进入作用域，分配 2 帧\n  let _a = FrameTracker::alloc(&fa)?;\n  let _b = FrameTracker::alloc(&fa)?;"
    )
    .unwrap();

    {
        let _a = FrameTracker::alloc(fa).map_err(|e| e.to_string())?;
        let _b = FrameTracker::alloc(fa).map_err(|e| e.to_string())?;
        let inside = fa.borrow().stats();
        writeln!(
            out,
            "  作用域内：已持有 {} 帧，空闲 {} 帧。",
            inside.allocated, inside.free
        )
        .unwrap();
    }

    let after = fa.borrow().stats();
    writeln!(
        out,
        "  // 作用域结束，两个 FrameTracker 依次 drop\n  作用域外：已持有 {} 帧（回到演示前 {} 帧），空闲 {} 帧。",
        after.allocated, before.allocated, after.free
    )
    .unwrap();
    writeln!(
        out,
        "\n对照 C 语言的内核：`kfree(ptr)` 忘记调用一次就是泄漏，调用两次就是崩溃。\n\
         Rust 把\"释放\"绑在作用域上，`?` 提前返回、panic 展开都会正确触发 Drop。"
    )
    .unwrap();
    Ok(out)
}

fn demo_leak(fa: &Rc<RefCell<FrameAllocator>>) -> Result<String, String> {
    let mut out = String::from("【leak】mem::forget：真的泄漏一帧\n\n");
    let before = fa.borrow().stats().allocated;

    let frame = FrameTracker::alloc(fa).map_err(|e| e.to_string())?;
    let ppn = frame.ppn();
    writeln!(
        out,
        "  let frame = FrameTracker::alloc(&fa)?;   // PPN {}",
        hex_ppn(ppn)
    )
    .unwrap();
    std::mem::forget(frame);
    writeln!(
        out,
        "  std::mem::forget(frame);                  // 跳过 Drop，帧再也回不到空闲链表"
    )
    .unwrap();

    let after = fa.borrow().stats();
    writeln!(
        out,
        "\n  分配器持有 {} 帧（演示前 {} 帧）；空闲 {} 帧。",
        after.allocated, before, after.free
    )
    .unwrap();
    writeln!(out, "  这一帧会一直被占用到进程退出 —— 这就是泄漏。").unwrap();
    writeln!(
        out,
        "\n注意：safe Rust 里泄漏需要显式 `mem::forget`、`Box::leak` 或 `Rc` 循环引用，\n\
         不会像 C 那样\"忘了写 free\"就悄悄发生。rCore 的内核对象（FrameTracker、\n\
         Arc<FrameTracker>、TaskControlBlock）都靠这条规则守住物理内存。"
    )
    .unwrap();
    Ok(out)
}

fn text_borrow() -> String {
    String::from(
        "【借用】&T / &mut T：共享只读，独占可写\n\n\
         \x20   fn walk(pt: &PageTable, va: VirtAddr) -> Option<PhysAddr>   // 只读遍历\n\
         \x20   fn map(pt: &mut PageTable, va: VirtAddr, ...)                // 修改页表\n\n\
         \x20   let pt = PageTable::new(&mut mem, &mut fa)?;\n\
         \x20   let r1 = &pt;        // OK\n\
         \x20   let r2 = &pt;        // OK：多个不可变借用可以共存\n\
         \x20   let w = &mut pt;     // ← E0502: 已有不可变借用时不能再可变借用\n\n\
         对照 rCore：`PageTable::map` 拿到的是 `&mut self`。只要还有人在遍历页表，\n\
         编译器就不允许你改它。内核里最难查的\"边遍历边改页表\"数据竞争，\n\
         在 safe Rust 里根本编译不过。\n\n\
         小练习：把 `walk` 的签名改成 `&mut self` 试试，看看 `translate` 还能不能\n\
         在 `walk` 之后立刻调用（提示：借用还没结束）。",
    )
}

fn text_dangling() -> String {
    String::from(
        "【悬垂指针】引用不能比它指向的数据活得更久\n\n\
         \x20   fn bad() -> &PageTable {\n\
         \x20       let pt = PageTable::new(&mut mem, &mut fa)?;\n\
         \x20       &pt                  // ← E0515: 返回了局部变量的引用\n\
         \x20   }\n\n\
         在 C 里这会返回一个指向已释放栈帧的指针，调用者一读就是未定义行为。\n\
         在 Rust 里，生命周期检查（lifetime）在编译期就把它拦下了。\n\n\
         对照 rCore：页表、任务控制块都装在 `Arc`/`UPSafeCell` 里，\n\
         引用只在锁的保护范围内存在，出了作用域就拿不到 —— 悬垂指针无从谈起。",
    )
}

fn text_double_free() -> String {
    String::from(
        "【双重释放】为什么 safe Rust 写不出来\n\n\
         \x20   let a = FrameTracker::alloc(&fa)?;\n\
         \x20   let b = a;      // 移动：a 失效\n\
         \x20   drop(a);        // ← E0382: use of moved value\n\
         \x20   drop(b);        // 如果上面能编译，这里就会 double free\n\n\
         rCore 的 `FrameTracker` 没有实现 `Copy`/`Clone`，所以同一物理帧在同一时刻\n\
         只可能有一个所有者。需要共享时用 `Arc` 增加引用计数，由最后一个持有者\n\
         负责释放 —— 引用计数把\"谁负责 free\"变成了类型系统能检查的问题。\n\n\
         这正好解释了 rCore 的两个设计选择：\n\
         1. `FrameTracker: !Clone` —— 独占所有权；\n\
         2. `Arc<FrameTracker>` —— 共享所有权，计数归零才 Drop。",
    )
}
