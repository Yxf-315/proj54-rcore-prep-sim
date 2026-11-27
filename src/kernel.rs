//! 命令状态机：把内存、分配器、页表组装成一个可脚本驱动的实验台。
//!
//! [`Kernel::exec`] 接收一行命令、返回要打印的文本。
//! 这样交互模式（[`crate::repl`]）与自动化测试共用同一套逻辑，
//! 命令行里看到的每一个数字都是真的算出来的。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use crate::frame::{FrameAllocator, FrameStats, FrameTracker};
use crate::memory::{hex, hex64, hex_ppn, PhysAddr, PhysMemory, VirtAddr, PAGE_SIZE};
use crate::ownership;
use crate::paging::{AccessType, MapError, PageTable, PageTableEntry, PTE_R, PTE_U, PTE_W, PTE_X};
use crate::ui::Ui;

/// 模拟物理内存基址：与 rCore 的 `MEMORY_START` 一致（QEMU virt 平台）。
pub const MEM_BASE: usize = 0x8000_0000;
/// 模拟物理内存大小：64 MiB，共 16384 帧。
pub const MEM_SIZE: usize = 64 * 1024 * 1024;

/// 实验台内核状态。
pub struct Kernel {
    mem: PhysMemory,
    fa: Rc<RefCell<FrameAllocator>>,
    pt: PageTable,
    /// 由实验台"用户"持有的帧（对应 rCore 中落在 `FrameTracker` 变量里的帧）。
    owned: BTreeMap<usize, FrameTracker>,
    /// 已建立映射：VA -> PA。
    mappings: BTreeMap<usize, usize>,
    ui: Ui,
    quit: bool,
}

impl Kernel {
    /// 启动：初始化 64 MiB 物理内存、帧分配器与根页表。
    pub fn new(ui: Ui) -> Self {
        let mut mem = PhysMemory::new(MEM_BASE, MEM_SIZE);
        let fa = Rc::new(RefCell::new(FrameAllocator::new(mem.base(), mem.end())));
        let pt = {
            let mut allocator = fa.borrow_mut();
            PageTable::new(&mut mem, &mut allocator).expect("启动失败：物理内存不足以放下根页表")
        };
        Self {
            mem,
            fa,
            pt,
            owned: BTreeMap::new(),
            mappings: BTreeMap::new(),
            ui,
            quit: false,
        }
    }

    pub fn ui(&self) -> Ui {
        self.ui
    }

    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// 当前帧分配器统计（给测试用）。
    pub fn frame_stats(&self) -> FrameStats {
        self.fa.borrow().stats()
    }

    /// 执行一行命令。空行与 `#` 注释返回空字符串。
    pub fn exec(&mut self, line: &str) -> Result<String, String> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return Ok(String::new());
        }
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = tokens[0];
        let args = &tokens[1..];

        match cmd {
            "help" | "?" => self.cmd_help(args),
            "about" => Ok(self.about()),
            "status" | "info" => Ok(self.status()),
            "alloc" => self.cmd_alloc(args),
            "free" => self.cmd_free(args),
            "frames" | "frame" => Ok(self.frames_view()),
            "map" => self.cmd_map(args),
            "unmap" => self.cmd_unmap(args),
            "walk" => self.cmd_walk(args),
            "fault" => self.cmd_fault(args),
            "demo" => self.cmd_demo(args),
            "lab" => self.cmd_lab(args),
            "reset" => self.cmd_reset(),
            "clear" => Ok(self.clear_screen()),
            "quit" | "exit" => {
                self.quit = true;
                Ok(self.ui.dim("再见。记得把本次实验过程写进设计文档。"))
            }
            other => Err(format!(
                "未知命令 `{other}`。输入 `help` 查看全部命令，或 `lab lab3` 直接看演示。"
            )),
        }
    }

    // ---------------------------------------------------------------- 帮助

    fn cmd_help(&self, args: &[&str]) -> Result<String, String> {
        if let Some(topic) = args.first() {
            return Ok(self.help_topic(topic));
        }
        let rows = vec![
            row3("help [命令]", "显示帮助", "help map"),
            row3("about", "项目与赛道说明", "about"),
            row3("status", "系统总览", "status"),
            row3("alloc [n]", "分配 n 个物理帧", "alloc 3"),
            row3("free <ppn>", "释放物理帧", "free 0x8_0001"),
            row3("frames", "帧分配器统计", "frames"),
            row3(
                "map <va> <pa|auto> [rwxu]",
                "建立 4 KiB 映射",
                "map 0x1000 auto rw",
            ),
            row3("unmap <va>", "拆除映射", "unmap 0x1000"),
            row3("walk <va>", "逐级遍历 Sv39 页表", "walk 0x1000"),
            row3("fault <va> [r|w|x]", "模拟一次访存", "fault 0x2000 w"),
            row3("demo <名称>", "所有权演示", "demo raii"),
            row3("lab <lab2|lab3>", "运行实验脚本", "lab lab3"),
            row3("reset", "重建空白页表", "reset"),
            row3("clear", "清屏", "clear"),
            row3("quit", "退出", "quit"),
        ];
        Ok(format!(
            "{}\n{}\n{}",
            self.ui.title("rcore-memlab 命令一览"),
            self.ui.table(&["命令", "作用", "示例"], &rows),
            self.ui
                .dim("提示：`help <命令>` 查看细节；`lab lab2` / `lab lab3` 直接跑实验。")
        ))
    }

    fn help_topic(&self, topic: &str) -> String {
        let text = match topic {
            "map" => {
                "map <va> <pa|auto> [rwxu]\n\n\
                 把虚拟页 va 映射到物理页 pa；`auto` 会自动让分配器给一帧并记在你名下。\n\
                 中间级页表缺失时会自动申请物理帧，这正是 rCore `PageTable::map` 的行为。\n\n\
                 例子：\n\
                 \x20 map 0x1000 auto rw      # 自动分配一帧，可读可写\n\
                 \x20 map 0x2000 0x8021000 rx # 手工指定物理地址\n\n\
                 权限位：r 可读、w 可写、x 可执行、u 用户态可访问。"
            }
            "walk" => {
                "walk <va>\n\n\
                 按 Sv39 规则逐级读取 L2/L1/L0 页表项，打印每一项的物理地址、原始值与权限位，\n\
                 最后给出翻译结果。这一屏就是内核调试缺页时最想看的信息。"
            }
            "fault" => {
                "fault <va> [r|w|x]\n\n\
                 模拟一次读/写/取指访存：能翻译就返回物理地址，并把 PTE 的 A 位（写访问还有 D 位）\n\
                 置 1；翻译失败就打印缺页或权限异常发生在第几级页表。"
            }
            "demo" => {
                "demo <move|clone|raii|leak|borrow|dangling|double-free>\n\n\
                 用真实的物理帧演示 Rust 所有权语义，并说明它对应 rCore 的哪个设计。"
            }
            "lab" => {
                "lab <lab2|lab3>\n\n\
                 lab2：栈式物理帧分配器、LIFO 复用、RAII 回收；\n\
                 lab3：Sv39 页表、地址翻译、缺页与权限异常。"
            }
            "alloc" | "free" => {
                "alloc [n] / free <ppn>\n\n\
                 alloc 从共享分配器拿 n 帧（1..=64），记在实验台名下；\n\
                 free 把某一帧的 FrameTracker drop 掉，帧回到 recycled 栈。"
            }
            other => {
                return format!("没有 `{other}` 的详细帮助。输入 `help` 查看命令列表。");
            }
        };
        text.to_string()
    }

    fn about(&self) -> String {
        format!(
            "{}\n\
             rcore-memlab 是一个零依赖的 Rust 教学实验台：在用户态复刻 rCore-Tutorial\n\
             Lab2 的物理帧分配器与 Lab3 的 Sv39 三级页表，并用可交互命令把\n\
             Rust 的所有权 / 借用 / RAII 与内核资源管理对应起来。\n\n\
             对应赛题：面向操作系统课程的操作系统竞赛和实验（OS 功能挑战赛道）。\n\
             交付物：实验指导文档 + 测例 + 参考实现。\n\n\
             版本 {}  ·  BSD-3-Clause",
            self.ui.title("rcore-memlab"),
            crate::VERSION
        )
    }

    // ---------------------------------------------------------------- 状态

    fn status(&self) -> String {
        let stats = self.fa.borrow().stats();
        let mut out = String::new();
        out.push_str(&self.ui.section("系统状态"));
        out.push('\n');
        out.push_str(&self.ui.kv(&[
            (
                "物理内存",
                format!(
                    "{} .. {}（{} MiB，{} 帧，每帧 4 KiB）",
                    self.mem.base(),
                    self.mem.end(),
                    self.mem.len() / 1024 / 1024,
                    self.mem.frames()
                ),
            ),
            (
                "帧分配器",
                format!(
                    "持有 {} / {} 帧，空闲 {}，高水位 {}，峰值 {}",
                    stats.allocated, stats.total, stats.free, stats.high_water, stats.peak
                ),
            ),
            (
                "根页表",
                format!(
                    "PPN {}（{}）",
                    hex_ppn(self.pt.root().ppn()),
                    self.pt.root().start_pa()
                ),
            ),
            ("satp", format!("{}  (MODE=8, Sv39)", hex64(self.pt.satp()))),
            (
                "用户映射",
                format!(
                    "{} 个页面；实验台持有的物理帧 {} 个",
                    self.mappings.len(),
                    self.owned.len()
                ),
            ),
        ]));

        if !self.mappings.is_empty() {
            let rows: Vec<Vec<String>> = self
                .mappings
                .iter()
                .map(|(va, pa)| vec![hex(*va), hex(*pa)])
                .collect();
            out.push('\n');
            out.push('\n');
            out.push_str(&self.ui.dim("已建立的映射（VA → PA）："));
            out.push('\n');
            out.push_str(&self.ui.table(&["虚拟地址", "物理地址"], &rows));
        }
        out
    }

    fn frames_view(&self) -> String {
        let s = self.fa.borrow().stats();
        let rows = vec![
            vec!["可用帧总数".to_string(), s.total.to_string()],
            vec!["当前持有".to_string(), s.allocated.to_string()],
            vec!["当前空闲".to_string(), s.free.to_string()],
            vec!["水位线（曾用帧数）".to_string(), s.high_water.to_string()],
            vec!["历史峰值".to_string(), s.peak.to_string()],
            vec!["回收栈长度".to_string(), s.recycled.to_string()],
        ];
        format!(
            "{}\n{}\n{}",
            self.ui.section("帧分配器统计"),
            self.ui.table(&["指标", "数值"], &rows),
            self.ui.dim(
                "水位线 = frontier - start_ppn；回收栈 = 已 Drop、等待复用的帧。\
                 rCore 的 stack allocator 就是这两个变量。"
            )
        )
    }

    // ---------------------------------------------------------------- 帧命令

    fn cmd_alloc(&mut self, args: &[&str]) -> Result<String, String> {
        let n = match args.first() {
            Some(raw) => parse_usize(raw).ok_or_else(|| format!("无法解析帧数 `{raw}`"))?,
            None => 1,
        };
        if n == 0 || n > 64 {
            return Err("一次分配 1..=64 帧".into());
        }

        let mut rows = Vec::new();
        let mut oom = false;
        for i in 0..n {
            match FrameTracker::alloc(&self.fa) {
                Ok(tracker) => {
                    let ppn = tracker.ppn();
                    rows.push(vec![
                        (i + 1).to_string(),
                        hex_ppn(ppn),
                        tracker.start_pa().to_string(),
                    ]);
                    self.owned.insert(ppn, tracker);
                }
                Err(_) => {
                    oom = true;
                    break;
                }
            }
        }

        let mut out = format!(
            "已分配 {} 个物理帧（FrameTracker 持有，Drop 时自动归还）：\n",
            rows.len()
        );
        out.push_str(&self.ui.table(&["序号", "PPN", "物理地址"], &rows));
        if oom {
            write!(
                out,
                "\n{}",
                self.ui.warn(
                    "物理帧耗尽：rCore 中 `FrameAllocator::alloc` 返回 None，页表构建会失败。"
                )
            )
            .unwrap();
        }
        Ok(out)
    }

    fn cmd_free(&mut self, args: &[&str]) -> Result<String, String> {
        let raw = args.first().ok_or("用法：free <ppn>，例如 free 0x8_0001")?;
        let ppn = parse_usize(raw).ok_or_else(|| format!("无法解析 PPN `{raw}`"))?;

        match self.owned.remove(&ppn) {
            Some(tracker) => {
                let pa = tracker.start_pa();
                drop(tracker);
                let s = self.fa.borrow().stats();
                Ok(format!(
                    "已释放 PPN {}（{}）。当前持有 {} 帧，空闲 {} 帧。\n{}",
                    hex_ppn(ppn),
                    pa,
                    s.allocated,
                    s.free,
                    self.ui
                        .dim("Drop → 帧压入 recycled 栈，下一次分配会优先复用它。")
                ))
            }
            None => Err(format!(
                "PPN {} 不在实验台持有列表中。只有被 FrameTracker 持有的帧才能释放——\
                 这正是 rCore 防止误释放他人内存的方式。",
                hex_ppn(ppn)
            )),
        }
    }

    // ---------------------------------------------------------------- 页表命令

    fn cmd_map(&mut self, args: &[&str]) -> Result<String, String> {
        if args.len() < 2 {
            return Err("用法：map <va> <pa|auto> [rwxu]，例如 map 0x1000 auto rw".into());
        }
        let va_raw =
            parse_usize(args[0]).ok_or_else(|| format!("无法解析虚拟地址 `{}`", args[0]))?;
        let va = VirtAddr::new(va_raw).map_err(|e| e.to_string())?;
        let flags = match args.get(2) {
            Some(raw) => {
                parse_flags(raw).ok_or_else(|| format!("无法解析权限 `{raw}`（可用 r/w/x/u）"))?
            }
            None => PTE_R | PTE_W,
        };

        let mut auto_ppn = None;
        let pa = if args[1].eq_ignore_ascii_case("auto") {
            let tracker = FrameTracker::alloc(&self.fa).map_err(|e| e.to_string())?;
            let ppn = tracker.ppn();
            let pa = tracker.start_pa();
            self.owned.insert(ppn, tracker);
            auto_ppn = Some(ppn);
            pa
        } else {
            let raw =
                parse_usize(args[1]).ok_or_else(|| format!("无法解析物理地址 `{}`", args[1]))?;
            PhysAddr::new(raw)
        };

        match self.map_inner(va, pa, flags) {
            Ok(()) => {
                self.mappings.insert(va.raw(), pa.raw());
                let note = match auto_ppn {
                    Some(ppn) => format!("（自动分配到 PPN {}）", hex_ppn(ppn)),
                    None => String::new(),
                };
                Ok(format!(
                    "映射建立成功：VA {} → PA {}{}\n权限：{}\n{}",
                    va,
                    pa,
                    note,
                    perm_string(flags),
                    self.ui.dim(
                        "中间级页表按需分配并清零；在 rCore 里这一步需要 FrameAllocator 提供物理帧。"
                    )
                ))
            }
            Err(e) => {
                if let Some(ppn) = auto_ppn {
                    self.owned.remove(&ppn);
                }
                Err(e.to_string())
            }
        }
    }

    fn cmd_unmap(&mut self, args: &[&str]) -> Result<String, String> {
        let raw = args.first().ok_or("用法：unmap <va>")?;
        let va_raw = parse_usize(raw).ok_or_else(|| format!("无法解析虚拟地址 `{raw}`"))?;
        let va = VirtAddr::new(va_raw).map_err(|e| e.to_string())?;

        match self.pt.unmap(va, &mut self.mem) {
            Ok(ppn) => {
                self.mappings.remove(&va.raw());
                let held = self.owned.contains_key(&ppn);
                let next = if held {
                    "该物理帧仍由 FrameTracker 持有，可用 free 回收。"
                } else {
                    "该物理帧不在实验台持有列表中。"
                };
                Ok(format!(
                    "已拆除 VA {} 的映射（原物理帧 PPN {}）。{}\n{}",
                    va,
                    hex_ppn(ppn),
                    next,
                    self.ui
                        .dim("中间级页表不随 unmap 回收；rCore 的 unmap 同样只清叶子项。")
                ))
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn cmd_walk(&mut self, args: &[&str]) -> Result<String, String> {
        let raw = args.first().ok_or("用法：walk <va>")?;
        let va_raw = parse_usize(raw).ok_or_else(|| format!("无法解析虚拟地址 `{raw}`"))?;
        let va = VirtAddr::new(va_raw).map_err(|e| e.to_string())?;
        Ok(self.walk_report(va))
    }

    fn cmd_fault(&mut self, args: &[&str]) -> Result<String, String> {
        let raw = args.first().ok_or("用法：fault <va> [r|w|x]")?;
        let va_raw = parse_usize(raw).ok_or_else(|| format!("无法解析虚拟地址 `{raw}`"))?;
        let va = VirtAddr::new(va_raw).map_err(|e| e.to_string())?;
        let access = match args.get(1) {
            Some(raw) => AccessType::parse(raw)
                .ok_or_else(|| format!("无法解析访问类型 `{raw}`（r/w/x）"))?,
            None => AccessType::Read,
        };

        match self.pt.translate(va, access, &mut self.mem) {
            Ok(pa) => Ok(format!(
                "{} 访存成功（{}）：VA {} → PA {}\n{}",
                self.ui.ok("✓"),
                access,
                va,
                pa,
                self.ui.dim("PTE 的 A 位已置 1；写访问还会置 D 位。")
            )),
            Err(fault) => Ok(format!(
                "{} {}\n\n{}",
                self.ui.err("✗ 触发异常"),
                fault,
                self.fault_hint()
            )),
        }
    }

    fn fault_hint(&self) -> String {
        self.ui.dim(
            "对照 rCore Lab3：\n\
             \x20 · CPU 访存失败会跳到 __alltraps，由 trap_handler 处理；\n\
             \x20 · 内核根据 scause 区分 Load / Store / Instruction Page Fault，\n\
             \x20   再决定\"补映射\"还是\"杀掉进程\"；\n\
             \x20 · 你在上面看到的那一级 V=0，正是内核定位问题所需的信息。",
        )
    }

    fn walk_report(&mut self, va: VirtAddr) -> String {
        let steps = self.pt.walk(va, &self.mem);
        let mut rows = Vec::new();
        for step in &steps {
            let pte = PageTableEntry(step.raw);
            let next = if !step.valid {
                "—".to_string()
            } else if step.leaf {
                "叶子（4 KiB 页）".to_string()
            } else {
                pte.next_table_pa().to_string()
            };
            rows.push(vec![
                format!("L{}", step.level),
                step.index.to_string(),
                step.pte_addr.to_string(),
                hex64(step.raw),
                if step.valid { "1" } else { "0" }.to_string(),
                format!(
                    "{} {} {}",
                    if pte.readable() { "R" } else { "-" },
                    if pte.writable() { "W" } else { "-" },
                    if pte.executable() { "X" } else { "-" }
                ),
                next,
            ]);
        }

        let outcome = match self.pt.translate(va, AccessType::Read, &mut self.mem) {
            Ok(pa) => format!(
                "{} 翻译结果：VA {} → PA {}（读访问成功）",
                self.ui.ok("✓"),
                va,
                pa
            ),
            Err(fault) => format!("{} {}", self.ui.err("✗"), fault),
        };

        format!(
            "{}\n{}\n{outcome}\n{}",
            self.ui.section(&format!("Sv39 三级页表遍历：VA = {va}")),
            self.ui.table(
                &[
                    "级别",
                    "VPN 索引",
                    "PTE 地址",
                    "PTE 值",
                    "V",
                    "R W X",
                    "下一级 / 叶子"
                ],
                &rows
            ),
            self.ui.dim(
                "Sv39：VA[38:30]=VPN[2] 选 L2 项，VA[29:21]=VPN[1] 选 L1 项，\
                 VA[20:12]=VPN[0] 选 L0 项，最后 12 位是页内偏移。"
            )
        )
    }

    // ---------------------------------------------------------------- 演示与实验

    fn cmd_demo(&mut self, args: &[&str]) -> Result<String, String> {
        let name = args.first().ok_or_else(|| {
            "用法：demo <名称>。可选：move / clone / raii / leak / borrow / dangling / double-free"
                .to_string()
        })?;
        ownership::demo(name, &self.fa)
    }

    fn cmd_lab(&mut self, args: &[&str]) -> Result<String, String> {
        match args.first().copied() {
            Some("lab2") => self.run_lab2(),
            Some("lab3") => self.run_lab3(),
            Some(other) => Err(format!("未知实验 `{other}`，可选：lab2 / lab3")),
            None => Err("用法：lab <lab2|lab3>".into()),
        }
    }

    fn run_lab2(&mut self) -> Result<String, String> {
        let mut out = String::new();
        out.push_str(&self.ui.section("Lab2 · 物理帧分配器（stack allocator）"));
        out.push_str("\n\n");
        out.push_str(
            "rCore 的分配器只有两个状态：`current`（水位线）与 `recycled`（回收栈）。\n\
             先连续分配 3 帧：\n",
        );

        let a = self.alloc_one()?;
        let b = self.alloc_one()?;
        let c = self.alloc_one()?;
        out.push('\n');
        out.push_str(&self.frame_table(&[a, b, c]));

        out.push_str(&format!("\n现在释放中间那一帧 PPN {}：\n", hex_ppn(b)));
        self.free_owned(b)?;
        out.push_str(&format!("  已释放 PPN {}（压入回收栈顶）。\n", hex_ppn(b)));

        out.push_str("\n再分配 2 帧，观察复用顺序：\n");
        let d = self.alloc_one()?;
        let e = self.alloc_one()?;
        out.push('\n');
        out.push_str(&self.frame_table(&[d, e]));
        if d == b {
            out.push_str(&format!(
                "\n{} 第一帧直接复用了 PPN {} —— recycled 栈的 LIFO 行为。",
                self.ui.ok("✓"),
                hex_ppn(b)
            ));
        } else {
            out.push_str(&format!(
                "\n{} 回收栈里还有更早释放的帧，所以本次没有复用 PPN {}。",
                self.ui.warn("注意"),
                hex_ppn(b)
            ));
        }

        // 收尾：把本实验分配的帧全部还回去，展示 RAII 的效果。
        for ppn in [a, c, d, e] {
            if self.owned.contains_key(&ppn) {
                self.free_owned(ppn)?;
            }
        }

        out.push_str("\n\n收尾：释放本实验分配的全部帧。");
        out.push('\n');
        out.push_str(&self.stats_table());
        out.push_str(&format!(
            "\n{}",
            self.ui.dim(
                "要点：分配只推进水位线，释放只压回收栈；没有任何一次 malloc/free 系统调用，\
                 全部由 FrameTracker 的 Drop 驱动。"
            )
        ));
        Ok(out)
    }

    fn run_lab3(&mut self) -> Result<String, String> {
        self.reset_paging()?;

        let mut out = String::new();
        out.push_str(&self.ui.section("Lab3 · Sv39 页表与地址翻译"));
        out.push_str("\n\n");
        out.push_str("先建立三条映射（物理帧由分配器自动提供）：\n");

        let va_rw = VirtAddr::new(0x1000).unwrap();
        let va_r = VirtAddr::new(0x2000).unwrap();
        let va_rx = VirtAddr::new(0x3000).unwrap();
        let p1 = self.map_auto(va_rw, PTE_R | PTE_W | PTE_U)?;
        let p2 = self.map_auto(va_r, PTE_R | PTE_U)?;
        let p3 = self.map_auto(va_rx, PTE_R | PTE_X | PTE_U)?;

        let rows = vec![
            vec![
                va_rw.to_string(),
                p1.to_string(),
                perm_string(PTE_R | PTE_W | PTE_U),
            ],
            vec![va_r.to_string(), p2.to_string(), perm_string(PTE_R | PTE_U)],
            vec![
                va_rx.to_string(),
                p3.to_string(),
                perm_string(PTE_R | PTE_X | PTE_U),
            ],
        ];
        out.push_str(&self.ui.table(&["虚拟地址", "物理地址", "权限"], &rows));

        out.push_str("\n\n");
        out.push_str(&self.walk_report(va_rw));

        out.push_str("\n\n");
        out.push_str(&self.fault_report(va_r, AccessType::Write));
        out.push_str("\n\n");
        out.push_str(&self.fault_report(VirtAddr::new(0x4_0000).unwrap(), AccessType::Read));

        out.push_str("\n\n");
        out.push_str("最后拆除 VA 0x0000_3000 的映射，再走一次页表：\n");
        self.pt
            .unmap(va_rx, &mut self.mem)
            .map_err(|e| e.to_string())?;
        self.mappings.remove(&va_rx.raw());
        out.push_str(&self.walk_report(va_rx));

        out.push_str("\n\n");
        out.push_str(&self.stats_table());
        out.push_str(&format!(
            "\n{}\n{}\n{}",
            self.ui.dim(
                "对照 rCore：`os/src/mm/page_table.rs` 的 map/unmap 与这里同构；\
                 缺页路径是 __alltraps → trap_handler → 补映射或杀进程。"
            ),
            self.ui.dim(
                "本次实验占用的 3 个数据帧仍在持有列表中（可用 free 释放）；\
                 reset 只回收页表帧，不回收数据帧。"
            ),
            self.ui.dim(
                "reset 回收的页表帧会立刻被后续分配复用，所以数据页的物理地址\
                 可能和之前的页表帧相同 —— 这正是物理内存被重复利用的证据。"
            )
        ));
        Ok(out)
    }

    fn fault_report(&mut self, va: VirtAddr, access: AccessType) -> String {
        match self.pt.translate(va, access, &mut self.mem) {
            Ok(pa) => format!(
                "{} 访存成功（{}）：VA {} → PA {}",
                self.ui.ok("✓"),
                access,
                va,
                pa
            ),
            Err(fault) => format!(
                "{} VA {} 尝试{}：\n    {}",
                self.ui.err("✗"),
                va,
                access,
                fault
            ),
        }
    }

    fn reset_paging(&mut self) -> Result<(), String> {
        let mut allocator = self.fa.borrow_mut();
        let new_pt = PageTable::new(&mut self.mem, &mut allocator).map_err(|e| e.to_string())?;
        let old = std::mem::replace(&mut self.pt, new_pt);
        old.release(&mut self.mem, &mut allocator);
        drop(allocator);
        self.mappings.clear();
        Ok(())
    }

    fn cmd_reset(&mut self) -> Result<String, String> {
        self.reset_paging()?;
        let s = self.fa.borrow().stats();
        Ok(format!(
            "已重建空白页表。页表帧已全部归还；当前持有 {} 帧，空闲 {} 帧。\n{}",
            s.allocated,
            s.free,
            self.ui
                .dim("注意：你通过 alloc/map auto 持有的数据帧不会被 reset 释放，请显式 free。")
        ))
    }

    // ---------------------------------------------------------------- 内部工具

    fn map_inner(&mut self, va: VirtAddr, pa: PhysAddr, flags: u64) -> Result<(), MapError> {
        let mut allocator = self.fa.borrow_mut();
        self.pt.map(va, pa, flags, &mut self.mem, &mut allocator)
    }

    fn alloc_one(&mut self) -> Result<usize, String> {
        let tracker = FrameTracker::alloc(&self.fa).map_err(|e| e.to_string())?;
        let ppn = tracker.ppn();
        self.owned.insert(ppn, tracker);
        Ok(ppn)
    }

    fn free_owned(&mut self, ppn: usize) -> Result<(), String> {
        match self.owned.remove(&ppn) {
            Some(tracker) => {
                drop(tracker);
                Ok(())
            }
            None => Err(format!("PPN {} 不在持有列表中", hex_ppn(ppn))),
        }
    }

    fn map_auto(&mut self, va: VirtAddr, flags: u64) -> Result<PhysAddr, String> {
        let tracker = FrameTracker::alloc(&self.fa).map_err(|e| e.to_string())?;
        let ppn = tracker.ppn();
        let pa = tracker.start_pa();
        self.owned.insert(ppn, tracker);
        match self.map_inner(va, pa, flags) {
            Ok(()) => {
                self.mappings.insert(va.raw(), pa.raw());
                Ok(pa)
            }
            Err(e) => {
                self.owned.remove(&ppn);
                Err(e.to_string())
            }
        }
    }

    fn frame_table(&self, ppns: &[usize]) -> String {
        let rows: Vec<Vec<String>> = ppns
            .iter()
            .enumerate()
            .map(|(i, ppn)| {
                vec![
                    (i + 1).to_string(),
                    hex_ppn(*ppn),
                    PhysAddr(*ppn * PAGE_SIZE).to_string(),
                ]
            })
            .collect();
        self.ui.table(&["序号", "PPN", "物理地址"], &rows)
    }

    fn stats_table(&self) -> String {
        let s = self.fa.borrow().stats();
        let rows = vec![
            vec!["持有帧".to_string(), s.allocated.to_string()],
            vec!["空闲帧".to_string(), s.free.to_string()],
            vec!["高水位".to_string(), s.high_water.to_string()],
            vec!["回收栈".to_string(), s.recycled.to_string()],
        ];
        format!(
            "{}\n{}",
            self.ui.dim("当前分配器状态："),
            self.ui.table(&["指标", "数值"], &rows)
        )
    }

    fn clear_screen(&self) -> String {
        if self.ui.color_enabled() {
            "\x1b[2J\x1b[H".to_string()
        } else {
            "\n".repeat(2)
        }
    }
}

fn row3(a: &str, b: &str, c: &str) -> Vec<String> {
    vec![a.to_string(), b.to_string(), c.to_string()]
}

fn perm_string(flags: u64) -> String {
    let mut out = String::new();
    out.push_str(if flags & PTE_R != 0 { "R" } else { "-" });
    out.push_str(if flags & PTE_W != 0 { "W" } else { "-" });
    out.push_str(if flags & PTE_X != 0 { "X" } else { "-" });
    if flags & PTE_U != 0 {
        out.push_str(" +U");
    }
    out
}

fn parse_usize(raw: &str) -> Option<usize> {
    let cleaned = raw.trim().replace('_', "");
    if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        usize::from_str_radix(hex, 16).ok()
    } else {
        cleaned.parse::<usize>().ok()
    }
}

fn parse_flags(raw: &str) -> Option<u64> {
    let mut flags = 0;
    for ch in raw.chars() {
        match ch.to_ascii_lowercase() {
            'r' => flags |= PTE_R,
            'w' => flags |= PTE_W,
            'x' => flags |= PTE_X,
            'u' => flags |= PTE_U,
            'v' | '-' | ',' => {}
            _ => return None,
        }
    }
    if flags & (PTE_R | PTE_W | PTE_X) == 0 {
        return None;
    }
    Some(flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernel() -> Kernel {
        Kernel::new(Ui::new(false))
    }

    #[test]
    fn boot_consumes_exactly_one_frame_for_root_table() {
        let k = kernel();
        assert_eq!(k.frame_stats().allocated, 1);
        assert_eq!(k.frame_stats().high_water, 1);
    }

    #[test]
    fn alloc_and_free_round_trip() {
        let mut k = kernel();
        let out = k.exec("alloc 2").unwrap();
        assert!(out.contains("已分配 2 个物理帧"));
        assert_eq!(k.frame_stats().allocated, 3);

        let ppn = *(k.owned.keys().next().unwrap());
        let out = k.exec(&format!("free 0x{ppn:x}")).unwrap();
        assert!(out.contains("已释放"));
        assert_eq!(k.frame_stats().allocated, 2);
    }

    #[test]
    fn free_unknown_frame_is_rejected() {
        let mut k = kernel();
        let err = k.exec("free 0x80999").unwrap_err();
        assert!(err.contains("不在实验台持有列表"));
    }

    #[test]
    fn map_auto_then_walk_translates() {
        let mut k = kernel();
        let out = k.exec("map 0x1000 auto rw").unwrap();
        assert!(out.contains("映射建立成功"));
        let out = k.exec("walk 0x1000").unwrap();
        assert!(out.contains("翻译结果"));
        assert!(out.contains("L2"));
        assert!(out.contains("L0"));
    }

    #[test]
    fn fault_reports_permission_and_not_mapped() {
        let mut k = kernel();
        k.exec("map 0x1000 auto r").unwrap();
        let out = k.exec("fault 0x1000 w").unwrap();
        assert!(out.contains("权限异常"));
        let out = k.exec("fault 0x9000").unwrap();
        assert!(out.contains("缺页异常"));
    }

    #[test]
    fn walk_reports_three_levels_for_mapped_page() {
        let mut k = kernel();
        k.exec("map 0x4000_0000 auto rw").unwrap();
        let out = k.exec("walk 0x4000_0000").unwrap();
        assert!(out.contains("L2"));
        assert!(out.contains("L1"));
    }

    #[test]
    fn labs_run_and_are_repeatable() {
        let mut k = kernel();
        k.exec("lab lab2").unwrap();
        k.exec("lab lab2").unwrap();
        k.exec("lab lab3").unwrap();
        // 根页表 1 帧 + 两个中间级页表 2 帧 + lab3 自动映射的 3 个数据帧
        let stats = k.frame_stats();
        assert_eq!(stats.allocated, 6);
        // 重复运行 lab3 会重建页表并回收页表帧，但数据帧仍然保留，不会 panic。
        k.exec("lab lab3").unwrap();
        assert!(k.frame_stats().allocated >= 6);
    }

    #[test]
    fn demos_do_not_leak_except_leak_demo() {
        let mut k = kernel();
        let before = k.frame_stats().allocated;
        for name in ["move", "clone", "raii", "borrow", "dangling", "double-free"] {
            k.exec(&format!("demo {name}")).unwrap();
            assert_eq!(k.frame_stats().allocated, before, "demo {name} 泄漏了帧");
        }
        k.exec("demo leak").unwrap();
        assert_eq!(k.frame_stats().allocated, before + 1);
    }

    #[test]
    fn unknown_command_lists_help_hint() {
        let mut k = kernel();
        let err = k.exec("frobnicate").unwrap_err();
        assert!(err.contains("help"));
    }

    #[test]
    fn parse_hex_and_decimal() {
        assert_eq!(parse_usize("0x8020_0000"), Some(0x8020_0000));
        assert_eq!(parse_usize("4096"), Some(4096));
        assert_eq!(parse_usize("nope"), None);
    }

    #[test]
    fn reset_returns_page_table_frames() {
        let mut k = kernel();
        k.exec("map 0x1000 auto rw").unwrap();
        k.exec("map 0x2000 auto rw").unwrap();
        let after_maps = k.frame_stats().allocated;
        assert!(after_maps > 3);
        k.exec("reset").unwrap();
        // 只剩两个数据帧 + 新根页表
        assert_eq!(k.frame_stats().allocated, 3);
        assert!(k.mappings.is_empty());
    }
}
