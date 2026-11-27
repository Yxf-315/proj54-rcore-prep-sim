# 设计文档 · rcore-memlab

> 版本：v1.0（华北区域赛提交版）
> 对应赛题：proj54 · 基于 Rust 的 rCore-Tutorial 配套预习模拟器（华北区域赛 · 功能挑战）

---

## 1. 选题背景

rCore-Tutorial 把操作系统实验拆成 Lab1～Lab8，其中 **Lab2（物理内存管理）** 与
**Lab3（虚拟内存）** 是学生第一次直接操作"内核数据结构"的地方：

- `FrameAllocator` 用 `current` 与 `recycled` 两个变量管理物理帧；
- `FrameTracker` 用 `Drop` 归还帧；
- `PageTable` 用三级页表把虚拟地址翻译成物理地址；
- 缺页异常由 `trap_handler` 分派处理。

但这些都发生在 QEMU + 裸机环境里，学生只能靠 `println!` 和 GDB 观察，且一旦写错就是
三重故障（triple fault）或静默错误。课程作业也普遍停留在"填空通过测例"，缺少
**可观察、可交互、可复现** 的中间态。

另一方面，课程前半段的 Rust 教学（变量/所有权/借用、复合类型、流程控制、泛型与特征、
集合与生命周期）与 rCore 的实现语言完全一致，但两门内容往往各讲各的：
学生知道 `move` 会失效，却不知道它对应内核里"句柄只能有一个主人"。

本项目的切入点：**用 Rust 所有权作为桥梁，把 rCore Lab2/Lab3 的内存管理做成可交互实验台。**
对应 proj54 申报口径，对外声明的核心知识点是 **Sv39 分页地址翻译（Lab3）**；
Lab2 的帧分配器作为页表的物理内存来源保留，同时用于演示 Rust 所有权与 RAII。
纯用户态、零依赖、`cargo run` 即用，既能在课堂上演示，也能作为课后实验与自动评测的基础。

## 2. 目标与非目标

### 2.1 目标

1. **可观察**：每一次分配、释放、映射、翻译都能看到中间状态（水位线、回收栈、PTE、A/D 位）。
2. **可交互**：REPL 命令 + 实验脚本 + 单命令模式，教师演示与学生探索共用一套接口。
3. **可验证**：页表真实存放在内存中，翻译逐级读取；错误路径（缺页/权限）可复现。
4. **可教学**：每个模块与 rCore 源文件一一对应，文档给出对照表与思考题。
5. **可扩展**：命令层是纯状态机，便于第二阶段加入任务切换、COW、闯关模式。

### 2.2 非目标

- 不实现完整的 RISC-V 模拟器，不执行真实机器码；
- 不替代 rCore 本身的内核实验，而是其"观察窗口"与"前置训练"；
- 第一阶段不支持高半区内核地址（`0xffff_ffc0_...`）、多级大页映射与多进程地址空间
  （大页与 `unmap` 空表回收列入第二阶段挑战题）。

## 3. 需求分析

### 3.1 用户与场景

| 用户 | 场景 | 需求 |
| --- | --- | --- |
| 课程学生 | 课前预习 Lab2/Lab3 | 低门槛运行、看得懂的输出、可对照课本 |
| 课程学生 | 课后做扩展实验 | 清晰的数据结构边界、可测试的 API |
| 教师/助教 | 课堂演示 | 一条命令跑完整个流程、可脚本回放 |
| 参赛评委 | 复现与评审 | 零依赖构建、自动化测试、文档齐全 |

### 3.2 功能需求

| 编号 | 功能 | 优先级 |
| --- | --- | --- |
| F1 | 物理帧分配/释放，显示 PPN、物理地址、水位线、回收栈 | P0 |
| F2 | 建立/拆除 4 KiB 页映射，权限位可配置 | P0 |
| F3 | Sv39 三级页表逐级遍历，显示 PTE 原始值与权限 | P0 |
| F4 | 模拟访存：成功返回 PA 并置 A/D 位；失败报告缺页层级/权限原因 | P0 |
| F5 | 所有权演示（move/clone/raii/leak/borrow/dangling/double-free） | P0 |
| F6 | 实验脚本 `lab2` / `lab3` 一键演示 | P0 |
| F7 | 脚本回放与单命令模式（便于 CI 与录屏） | P1 |
| F8 | 页表整体释放（`release`），reset 不泄漏页表帧 | P1 |
| F9 | 中英混排表格对齐、颜色可关闭 | P2 |

### 3.3 非功能需求

- **零第三方依赖**：离线环境可构建，避免评审环境网络问题；
- **确定性**：相同命令序列产生相同输出（除时间/颜色外），便于截图对照；
- **可测试**：核心逻辑不依赖 stdin/stdout，单元测试直接驱动 `Kernel::exec`；
- **跨平台**：Linux / macOS / Windows 均可运行（CI 覆盖 Ubuntu 与 Windows）；
- **性能**：命令响应 < 10 ms，64 MiB 内存初始化 < 50 ms。

## 4. 总体设计

### 4.1 分层架构

```
+-----------------------------------------------------------+
|  main.rs / repl.rs        入口、参数解析、交互循环          |
+-----------------------------+-----------------------------+
                              | 一行命令
+-----------------------------v-----------------------------+
|  kernel.rs                命令状态机、lab2/lab3 脚本        |
|                            Kernel::exec() -> String        |
+----------+----------------+----------------+--------------+
           |                |                |
+----------v-----+  +-------v------+  +------v-----------+
| frame.rs       |  | paging.rs    |  | ownership.rs     |
| FrameAllocator |  | PageTable    |  | 演示脚本         |
| FrameTracker   |  | Pte / Walk   |  | (move/Rc/RAII)   |
+----------+-----+  +-------+------+  +------------------+
           |                |
+----------v----------------v-------------------------------+
|  memory.rs   PhysAddr / VirtAddr / PhysMemory(64 MiB)     |
+-----------------------------------------------------------+
```

依赖方向严格单向：`memory` ← `frame` / `paging` ← `kernel` ← `repl` / `main`。
`ownership` 只依赖 `frame` 与 `memory`。

### 4.2 模块职责

| 模块 | 职责 | 对应 rCore |
| --- | --- | --- |
| `memory.rs` | 地址 newtype、Sv39 合法性检查、线性物理内存读写 | `os/src/mm/address.rs` |
| `frame.rs` | 栈式分配器、`FrameTracker` RAII、统计 | `os/src/mm/frame_allocator.rs` |
| `paging.rs` | PTE 位域、页表创建/映射/拆除/翻译/遍历/释放 | `os/src/mm/page_table.rs` |
| `ownership.rs` | 用真实帧演示所有权语义 | `FrameTracker` 的使用方式 |
| `cases.rs` | 配套测试样例（`.case`）的解析与判定 | 无（教学层） |
| `kernel.rs` | 命令解析、状态维护、实验脚本、输出格式化 | 无（教学层） |
| `ui.rs` | ANSI 颜色、CJK 宽度正确的表格 | 无 |
| `repl.rs` | 交互循环、横幅、提示符 | 无 |

### 4.3 数据流

```
用户输入 "map 0x1000 auto rw"
   │
   ▼
Kernel::exec  ──split_whitespace──► ["map", "0x1000", "auto", "rw"]
   │
   ├─ parse: VirtAddr::new(0x1000) / flags = R|W
   ├─ FrameTracker::alloc(&fa) ──► 取得一帧，登记到 owned
   ├─ PageTable::map(va, pa, flags, &mut mem, &mut fa)
   │     ├─ L2 PTE 无效 → 分配 L1 帧、清零、写链接 PTE
   │     ├─ L1 PTE 无效 → 分配 L0 帧、清零、写链接 PTE
   │     └─ L0 写叶子 PTE（PPN + R|W|V）
   └─ 记录 mappings[va] = pa，返回格式化文本
```

## 5. 数据结构设计

### 5.1 `FrameAllocator`

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `start_ppn` / `end_ppn` | `usize` | 可用帧区间（左闭右开） |
| `frontier` | `usize` | 从未使用过的第一帧，只增不减 |
| `recycled` | `Vec<usize>` | 已释放帧的栈（LIFO） |
| `allocated` | `usize` | 当前持有帧数 |
| `peak_allocated` | `usize` | 历史峰值 |

不变量：`allocated == (frontier - start_ppn) - recycled.len()`。
测试 `high_water_never_shrinks` 与 `drop_returns_frame_and_reuse_is_lifo` 守住这条性质。

### 5.2 `FrameTracker`

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `ppn` | `usize` | 拥有的帧号 |
| `allocator` | `Rc<RefCell<FrameAllocator>>` | 归还目标 |

`Drop` 调用私有的 `dealloc`。没有 `Clone`、没有 `Copy`，因此同一帧不可能被两个
`FrameTracker` 持有——这正是"双重释放"在 safe Rust 中写不出来的原因。

### 5.3 `PageTableEntry`

64 位，布局与 RISC-V Sv39 一致：

```
 63        54 53      10 9    8 7 6 5 4 3 2 1 0
+------------+----------+------+-+-+-+-+-+-+-+-+
|  reserved  |   PPN    | RSW  |D|A|G|U|X|W|R|V|
+------------+----------+------+-+-+-+-+-+-+-+-+
```

`new(ppn, flags)` 做 `(ppn << 10) | flags`；`ppn()` 做 `(raw >> 10) & ((1<<44)-1)`。

### 5.4 `PageTable` 与 `WalkStep`

- `PageTable` 只保存 `root: PhysFrame`，所有中间表都在 `PhysMemory` 中；
- `walk()` 返回 `Vec<WalkStep>`，每条记录 `level/index/pte_addr/raw/valid/leaf`，
  供 UI 渲染成表格；
- `translate()` 额外检查 R/W/X 并置 A/D 位。

### 5.5 命令层状态

| 字段 | 含义 |
| --- | --- |
| `mem: PhysMemory` | 64 MiB 模拟物理内存 |
| `fa: Rc<RefCell<FrameAllocator>>` | 共享分配器 |
| `pt: PageTable` | 当前根页表 |
| `owned: BTreeMap<usize, FrameTracker>` | 实验台持有的帧（可按 PPN 释放） |
| `mappings: BTreeMap<usize, usize>` | VA → PA，用于 `status` 展示 |

`owned` 用 `BTreeMap` 而不是 `Vec`，保证按 PPN 精确释放且输出顺序稳定。

## 6. 关键流程

### 6.1 分配

```
alloc:
  recycled.pop() 有值? ──是──► 复用该 PPN
        │否
  frontier < end? ──是──► 取 frontier，frontier += 1
        │否
      OutOfMemory
  allocated += 1；peak = max(peak, allocated)
```

### 6.2 释放（Drop）

```
FrameTracker::drop:
  allocator.borrow_mut().dealloc(ppn)
    ├─ debug_assert 区间合法、allocated > 0
    ├─ recycled.push(ppn)
    └─ allocated -= 1
```

### 6.3 映射

```
map(va, pa, flags):
  校验对齐、PA 在内存内
  table = root
  for level in [2,1]:
     pte = read(table + index(level)*8)
     if pte.valid && pte.leaf -> AlreadyMapped
     if !pte.valid:
         child = fa.alloc(); zero(child); write link PTE
         table = child
     else: table = pte.next_table_pa()
  pte = read(table + index(0)*8)
  if pte.valid && pte.leaf -> AlreadyMapped
  write leaf PTE(pa.ppn, flags | V)
```

### 6.4 翻译与缺页

```
translate(va, access):
  table = root
  for level in [2,1,0]:
     pte = read(...)
     if !pte.valid        -> PageFault::NotMapped { level }
     if pte.leaf:
         if !permits(access) -> PageFault::PermissionDenied { level }
         set A (write 时也 set D)
         return pa
     table = pte.next_table_pa()
  -> PageFault::Corrupt
```

"在第几级无效"是内核调试缺页时最关键的信息，也是本实验台输出的核心字段。

### 6.5 页表整体释放

`release()` 递归遍历有效非叶子项，先释放子树再释放本表，归还全部页表帧。
`reset` 用它保证反复运行实验不会泄漏页表帧（数据帧仍需显式 `free`）。

## 7. 与 rCore 源码对照

| 本项目 | rCore-Tutorial | 差异 |
| --- | --- | --- |
| `PhysAddr` / `VirtAddr` | `os/src/mm/address.rs` | 增加 Sv39 上界检查 |
| `FrameAllocator::alloc/dealloc` | `os/src/mm/frame_allocator.rs` | 用 `RefCell` 代替 `UPSafeCell` |
| `FrameTracker::drop` | 同名 | 用 `Rc` 代替 `Arc` |
| `PageTable::new/map/unmap` | `os/src/mm/page_table.rs` | 页表放在模拟内存中 |
| `PageTable::translate` | `find_pte` + 硬件 MMU | 显式置 A/D，返回错误类型 |
| `PTE_*` 常量 | `os/src/mm/page_table.rs` | 完全一致 |
| 缺页处理提示 | `os/src/trap/mod.rs` | 仅文字对照，不执行 |

## 8. 正确性分析

1. **位域正确**：`index(level) = (va >> (12 + 9*level)) & 0x1ff`，与 Sv39 规范一致；
   `satp = (8 << 60) | root_ppn`，MODE=8 表示 Sv39。
2. **PTE 正确**：`(ppn << 10) | flags`，PPN 取 44 位，保留位不被污染。
3. **边界**：`PhysMemory` 越界读写返回 `None/false`；`VirtAddr::new` 拒绝 `≥ 2^38`；
   `map` 拒绝未对齐与非内存内物理地址；`translate` 拒绝非叶子链末尾。
4. **一致性**：`unmap` 清叶子项并返回 PPN；`release` 归还所有页表帧，
   单元测试 `release_returns_all_table_frames` 验证"只剩数据帧"。
5. **不变量**：分配器 `allocated` 与 `recycled` 的关系由多个测试守住；
   `demos_do_not_leak_except_leak_demo` 验证演示脚本不会偷偷泄漏。

## 9. 测试方案

| 层次 | 位置 | 覆盖 |
| --- | --- | --- |
| 单元测试 | `src/*.rs` 内 `#[cfg(test)]` | 地址拆分、内存边界、分配/复用/耗尽、映射/翻译/缺页/权限、A/D 位、release |
| 集成测试 | `tests/cli.rs` | 参数解析、脚本回放、`--check`、退出码、help/version |
| 用例回归 | `cases/*.case` + `tests/cases_bundle.rs` | 8 个观察点：分配、RAII、遍历、权限、缺页、move、LIFO、全流程 |
| 手工验收 | `examples/tour.txt` | 完整演示流程、实验脚本可重复运行 |

```bash
cargo test                              # 43 项测试
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 10. 创新点

1. **把"页表"真的放进物理内存**：翻译不是查表模拟，而是逐级 `read_u64`，
   与硬件 MMU 走同一条路径，错误无法被"美化"。
2. **所有权与内核资源的显式映射**：每个演示都分配真实帧，并注明对应 rCore 的哪个设计
   （`!Clone` 的 `FrameTracker`、`Arc` 共享、`&mut self` 页表）。
3. **可脚本化的教学层**：`Kernel::exec` 纯状态机，使课堂演示、自动评测、CI 共用一条路径。
4. **零依赖 + 中英混排对齐**：评审环境离线可构建，中文表格不会错位。

## 11. 局限与后续

- 不支持高半区内核地址与多地址空间（第二阶段：ASID、内核页表映射）；
- 不支持 2 MiB / 1 GiB 大页映射（已列入挑战题 B）；
- `unmap` 不回收空页表（已列入挑战题 A）；
- 尚未与真实 rCore 仓库联动（第三阶段：对照模式）。

第二阶段与第三阶段任务见 [roadmap.md](roadmap.md)。

## 12. 与类似工具/做法的对比

| 对比对象 | 能做什么 | 局限 | 本项目的差异 |
| --- | --- | --- | --- |
| rCore 原生实验（QEMU + GDB） | 真实内核、真实硬件行为、权威 | 门槛高、反馈慢、写错易三重故障，看不到页表中间态 | 先在模拟器建立心智模型，再进内核；两者 PTE 位域与三级走法一致 |
| Rustlings | 语法与所有权练习，即时反馈 | 与操作系统无关，学生不知道所有权"为什么" | 把 move / Rc / Drop 映射到帧与页表的真实生命周期 |
| 教材与课件图示 | 直观、系统 | 静态、不可交互、无法验证 | 输入 VA/PA/权限，真实计算并给出 PTE 原始值 |
| 网页版页表可视化工具 | 图形化、易传播 | 与 rCore 代码脱节、依赖网络、难以离线评审 | 与 rCore 源码逐文件对照，零依赖、离线可跑、可脚本回放 |
| 直接读源码 | 最准确 | 初学者容易迷失在细节里 | 提供"可操作的中间层"，并给出源码对照表作为下一步 |

## 13. 教学试用

### 13.1 试用设计

- **对象**：已完成 Rust 基础语法、尚未开始 rCore Lab3 的本科生 5 人；
- **任务**：按 `docs/lab-guide.md` 完成实验一至实验三（约 60–90 分钟）；
- **观察**：完成率、各实验耗时、求助次数、卡点原话；
- **问卷**：5 级量表（是否有帮助、是否愿意推荐、难度是否合适）+ 两道开放题。

### 13.2 试用记录表（模板）

| 编号 | 年级/基础 | 完成任务 | 总耗时 | 主要卡点 | 原话摘录 | 改进措施 |
| --- | --- | --- | --- | --- | --- | --- |
| S1 | 大三/学过 Rust | 实验一、二 | 55 min | VPN 索引计算 | "不知道怎么从地址里切出 9 位" | 在 `walk` 输出加二进制提示 |
| S2 | 大三/无 Rust | 实验一 | 40 min | `free` 的 PPN 从哪来 | "PPN 和物理地址对不上" | 帮助里补 PPN×4096 公式 |
| S3 | 大二/学过 C | 实验一、二、三 | 75 min | 缺页层级判断 | "为什么不是 L0" | 增加"逐级 V 位"讲解卡 |

> 上表为格式示例，提交前替换为真实试用数据；评审时同时保留问卷截图与录屏。

### 13.3 试用结论（试用后填写）

- 完成率：____ / 5；
- 平均耗时：____ 分钟；
- 主要卡点 Top3：____、____、____；
- 已改进：____；
- 结论：是否达到"降低 Lab3 入门门槛"的预期。

## 14. 复现与提交

```bash
git clone <希冀 GitLab 仓库地址> proj54-SynRun
cd proj54-SynRun
cargo test                     # 43 项测试
cargo run -- --check cases     # 8 个配套用例
cargo run -- --exec examples/tour.txt
```

- 运行环境：Rust 1.75+，无第三方依赖；亦可用 `docker build -t rcore-memlab .`；
- 提交物：源代码、设计实现文档、使用说明、实验指导书、幻灯片、演示视频、试用数据；
- 托管：大赛希冀 GitLab（持续提交，提交信息按四阶段组织）；
- 许可：代码 BSD-3-Clause，文档与视频 CC BY-SA 4.0。

## 15. 术语表

| 术语 | 含义 |
| --- | --- |
| PPN | 物理页帧号，物理地址除以 4096 |
| PTE | 页表项，64 位，含 PPN 与 V/R/W/X/U/A/D 等标志 |
| VPN | 虚拟页号，Sv39 中拆成 VPN[2]/VPN[1]/VPN[0] 三个 9 位索引 |
| satp | RISC-V 寄存器，保存页表根 PPN 与分页模式（Sv39 = 8） |
| RAII | 资源获取即初始化，用对象生命周期管理资源释放 |
| 缺页 | 页表遍历中某一级 PTE 无效（V=0） |
| 权限异常 | PTE 有效但 R/W/X 不允许本次访问 |
