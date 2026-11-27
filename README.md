# rcore-memlab · rCore 内存实验室

> 面向 rCore-Tutorial **Sv39 分页地址翻译（Lab3）** 的配套预习模拟器：先用它把原理跑通、把过程看清，再进内核做实验。
>
> 赛题：**proj54 · 基于 Rust 的 rCore-Tutorial 配套预习模拟器**（华北区域赛 · 功能挑战赛道）
>
> 核心知识点：**地址翻译**；物理帧分配器（Lab2）作为页表的物理内存支撑，同时用于讲清 Rust 所有权。

`rcore-memlab` 是一个**零依赖**的 Rust 命令行程序。它把 rCore 内核里"看不见"的内存管理搬进用户态：
物理帧是真的从一块 64 MiB 模拟内存里分配的，页表是真的按 Sv39 规则逐级读内存翻译的，
缺页与权限异常也是真的算出来的——不是预先写好的台词。

```
$ cargo run
  rcore-memlab  ·  rCore 内存实验室
  用 Rust 所有权讲透 rCore Lab2/Lab3 的物理帧与 Sv39 页表

  输入 help 查看命令，或直接运行 lab lab2 / lab lab3。

memlab> lab lab3
== Lab3 · Sv39 页表与地址翻译
...
+------+----------+-------------+-----------------------+---+-------+------------------+
| 级别 | VPN 索引 | PTE 地址    | PTE 值                | V | R W X | 下一级 / 叶子    |
+------+----------+-------------+-----------------------+---+-------+------------------+
| L2   | 0        | 0x8000_c000 | 0x0000_0000_2000_1801 | 1 | - - - | 0x8000_6000      |
| L1   | 0        | 0x8000_6000 | 0x0000_0000_2000_1c01 | 1 | - - - | 0x8000_7000      |
| L0   | 1        | 0x8000_7008 | 0x0000_0000_2000_0017 | 1 | R W - | 叶子（4 KiB 页） |
+------+----------+-------------+-----------------------+---+-------+------------------+
✓ 翻译结果：VA 0x0000_1000 → PA 0x8000_0000（读访问成功）
```

---

## 为什么做这个

操作系统课程里最抽象的两块是**物理内存管理**和**地址翻译**：

- 学生在 rCore 里写完 `FrameAllocator::alloc`，看到的只是一行日志；
- 写完 `PageTable::map`，页表长什么样、翻译走了哪几级、缺页发生在第几级，全在 QEMU 里"黑箱"；
- Rust 的所有权、借用、RAII 又常被当成语法糖，学生不知道它们**为什么**是内核的第一道安全防线。

`rcore-memlab` 把这三件事接在一起：

| 你在本实验台看到的行为 | 对应 rCore 源码 | 对应课程知识点 |
| --- | --- | --- |
| 帧分配推进水位线、释放进入回收栈 | `os/src/mm/frame_allocator.rs` | 物理内存管理、连续分配 |
| `FrameTracker` 离开作用域自动归还 | `FrameTracker::drop` | RAII、所有权、生命周期 |
| `drop(a)` 后 `a` 不可再用 | `!Clone` 的 `FrameTracker` | move 语义、E0382 |
| `Rc<FrameTracker>` 引用计数 | `Arc<FrameTracker>`（COW 场景） | 共享所有权、引用计数 |
| 逐级打印 PTE、定位 V=0 的层级 | `os/src/mm/page_table.rs` | 多级页表、Sv39 |
| 读写/执行权限检查 | PTE 的 R/W/X 位 | 内存保护、缺页异常 |

## 与课程/训练营的衔接

项目选题直接来自课程与 Rustlings 训练营的内容，实验指导书里的前置练习与它们一一对应：

| 课程章节 | 用到的 Rust 能力 | 在本项目中的落点 |
| --- | --- | --- |
| 第 2 章 变量、所有权和借用 | move / borrow / `&mut` | `demo move`、`demo borrow`、页表的 `&mut self` |
| 第 3 章 复合类型 | struct / enum / 元组 | `PhysFrame`、`Pte`、`WalkStep` |
| 第 4 章 流程控制与模式匹配 | `match` / `Option` | 命令解析、`translate` 的分级匹配 |
| 第 5 章 方法、泛型和特征 | `impl` / `Drop` / trait | `FrameTracker::drop`、错误类型 |
| 第 6 章 集合类型、生命周期 | `Vec` / `BTreeMap` / 生命周期 | 回收栈、映射表、`walk` 的借用 |
| Rustlings `move_semantics` | 所有权转移 | `lab lab2` 的帧生命周期 |
| Rustlings `smart_pointers` | `Rc` / `RefCell` | 共享分配器 `Rc<RefCell<FrameAllocator>>` |
| Rustlings `lifetimes` | 借用检查 | `demo dangling` |

> 训练营的 Rustlings 作业在 CNB 平台完成（`rustlings watch`，110 道题），
> 本项目建议在完成 `move_semantics`、`smart_pointers`、`lifetimes` 三章后开始。

## 快速开始

需要 Rust 1.75+（`cargo` 在 PATH 中），**不需要任何第三方 crate，也不需要联网**。

```bash
git clone <希冀 GitLab 仓库地址> proj54-SynRun
cd proj54-SynRun
cargo run                                # 交互模式
cargo run -- --exec examples/tour.txt    # 回放完整演示脚本
cargo run -- --check cases               # 运行配套测试样例
cargo test                               # 43 个测试
```

> 仓库同时提供 Dockerfile 与预编译二进制，评审现场断网也能演示。

### 常用命令

| 命令 | 作用 | 示例 |
| --- | --- | --- |
| `lab lab2` | 物理帧分配器实验脚本 | `lab lab2` |
| `lab lab3` | Sv39 页表与缺页实验脚本 | `lab lab3` |
| `alloc [n]` / `free <ppn>` | 手动分配 / 释放物理帧 | `alloc 3` |
| `map <va> <pa\|auto> [rwxu]` | 建立 4 KiB 页映射 | `map 0x1000 auto rw` |
| `walk <va>` | 逐级遍历页表并翻译 | `walk 0x1000` |
| `fault <va> [r\|w\|x]` | 模拟访存、观察缺页/权限异常 | `fault 0x2000 w` |
| `demo <名称>` | 所有权演示 | `demo raii` |
| `status` / `frames` | 系统总览 / 分配器统计 | `status` |

`help` 查看全部命令，`help map` 查看单个命令细节。
`memlab --check cases` 会自动运行 `cases/*.case` 里的配套测试样例并给出通过/失败。

## 一次完整的实验（lab3 摘录）

```text
memlab> map 0x1000 auto rw
映射建立成功：VA 0x0000_1000 → PA 0x8000_5000（自动分配到 PPN 0x8_0005）
权限：RW-
memlab> walk 0x1000
| L2 | 0 | 0x8000_0000 | 0x0000_0000_2000_1801 | 1 | - - - | 0x8000_6000 |
| L1 | 0 | 0x8000_6000 | 0x0000_0000_2000_1c01 | 1 | - - - | 0x8000_7000 |
| L0 | 1 | 0x8000_7008 | 0x0000_0000_2000_1407 | 1 | R W - | 叶子（4 KiB 页） |
✓ 翻译结果：VA 0x0000_1000 → PA 0x8000_5000（读访问成功）
memlab> fault 0x2000 w
✗ 触发异常 权限异常：VA=0x0000_2000 尝试写，但 L0 的 PTE 权限位不允许
```

手工验证：`0x1000 >> 12 = 1`，所以 `VPN[0] = 1`、`VPN[1] = VPN[2] = 0`；
三级索引分别落在 `0x8000_0000`、`0x8000_6000`、`0x8000_7008` 三个物理地址上，
与 rCore 中 `PageTable::find_pte` 的走法完全一致。

## 目录结构

```
rcore-memlab/
├── src/
│   ├── memory.rs      # PhysAddr / VirtAddr / PhysMemory（对应 address.rs）
│   ├── frame.rs       # 栈式帧分配器 + FrameTracker（对应 frame_allocator.rs）
│   ├── paging.rs      # Sv39 页表、map/unmap/translate/walk（对应 page_table.rs）
│   ├── ownership.rs   # 7 个所有权演示脚本
│   ├── kernel.rs      # 命令状态机 + lab2/lab3 实验脚本（可测试）
│   ├── cases.rs       # 配套测试样例的解析与判定
│   ├── ui.rs          # CJK 宽度正确的表格渲染
│   ├── repl.rs        # 交互式命令行
│   └── main.rs        # 入口：交互 / -c / --exec / --check
├── cases/             # 8 个 .case 测试样例（对应 rCore 实验观察点）
├── examples/tour.txt  # 完整演示脚本（可回放）
├── docs/
│   ├── usage.md       # 使用说明（交付物之一）
│   ├── lab-guide.md   # 实验指导书
│   ├── design.md      # 设计文档
│   ├── submission.md  # 提交要点与时间线
│   ├── plan-20d.md    # 华北区域赛 20 天冲刺计划
│   └── roadmap.md     # 三阶段路线图
├── tests/             # 端到端测试 + 用例回归
├── tools/             # 文档/幻灯片/试用表的生成脚本
├── submission/        # 提交材料包（PDF / PPTX / XLSX）
├── .github/workflows/ci.yml
├── .gitlab-ci.yml     # 希冀 GitLab 流水线
└── Dockerfile
```

## 设计要点

1. **页表放在物理内存里。** `PageTable` 只持有根页帧号，每一级 PTE 都通过 `PhysMemory::read_u64`
   读取。因此"地址翻译"不可作弊：写错一个位域，结果立刻错。
2. **分配器与 rCore 同构。** 只有 `frontier` 与 `recycled` 两个状态；`FrameTracker` 的 `Drop`
   负责归还，`!Clone` 保证独占。
3. **命令层是纯状态机。** `Kernel::exec(&mut self, line) -> Result<String, String>`，
   交互模式、脚本回放、测试共用一条路径。
4. **零依赖。** 不引入 `clap`/`crossterm`，`cargo build` 在离线环境也能通过；
   ANSI 颜色在重定向时自动关闭。

## 测试

```bash
cargo test              # 32 个单元测试 + 端到端测试
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

覆盖点：分配/回收/LIFO 复用/耗尽、RAII 泄漏检测、三级遍历、未映射缺页、权限异常、
A/D 位置位、重复映射拒绝、`release` 递归回收、命令解析、实验脚本可重复运行。

## 路线图

- **第一阶段（本仓库当前状态）**：Lab2 帧分配器 + Lab3 Sv39 页表 + 所有权演示 + 测试 + 文档。
- **第二阶段**：`lab4` 任务切换可视化（TCB 所有权）、COW 演示、`unmap` 回收空页表、
  交互式闯关模式（题目 + 自动判分）。
- **第三阶段**：与真实 rCore 仓库联动的"对照模式"（同一操作在内核与模拟器中并排展示）、
  WebAssembly 版本、教师端实验配置。

详见 [docs/roadmap.md](docs/roadmap.md)。

## 相关文档

- [14 天冲刺计划](docs/plan-14d.md) —— 当前执行版（11/27–12/11）
- [提交清单](docs/checklist.md) —— 12 月 9 日提交前逐项核对
- [演示视频脚本](docs/video-script.md) —— 4 分钟分镜、旁白、录屏命令
- [答辩手册](docs/defense-handbook.md) —— 8 分钟流程、20 个高频问题
- [使用说明](docs/usage.md) —— 安装、命令参数、5 分钟上手、用例格式、与 rCore 源码对照
- [实验指导书](docs/lab-guide.md) —— 学生怎么用、四个实验、思考题
- [设计文档](docs/design.md) —— 需求、架构、数据结构、与 rCore 源码对照
- [大赛提交要点](docs/submission.md) —— 材料清单、评分维度、视频脚本、答辩准备
- [路线图](docs/roadmap.md)

## 许可

源代码采用 **BSD-3-Clause**（见 [LICENSE](LICENSE)），符合大赛"GPL / Apache / BSD / 木兰"的开源协议要求；
文档、幻灯片与演示视频采用 **CC BY-SA 4.0**。

## 致谢

- [rCore-Tutorial-Book](https://rcore-os.cn/rCore-Tutorial-Book-v3/)：本项目的知识来源与对照对象
- [Rustlings](https://github.com/rust-lang/rustlings)：所有权练习的设计参考
- 课程课件《变量、所有权和借用》《方法、泛型和特征》《集合类型、生命周期》等
