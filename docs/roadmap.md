# 路线图

> 三个阶段对应大赛的初赛 → 决赛 → 赛后开源维护。
> 每个阶段都以"可运行 + 可测试 + 可演示"为完成标准。

## 第一阶段：内存管理实验台（已完成，v0.1.0）

**目标**：把 rCore Lab2/Lab3 的核心机制做成可交互、可测试的参考实现。

- [x] `PhysAddr` / `VirtAddr` / `PhysMemory`（64 MiB，帧读写、清零、越界检查）
- [x] 栈式帧分配器：水位线 + 回收栈 + 峰值统计
- [x] `FrameTracker` RAII 句柄与 `Drop` 归还
- [x] Sv39 页表：建表、`map`、`unmap`、`translate`、`walk`、`release`
- [x] PTE 位域与 A/D 位、缺页层级、权限异常
- [x] 命令层：`alloc/free/map/walk/fault/status/frames/demo/lab`
- [x] 7 个所有权演示（move/clone/raii/leak/borrow/dangling/double-free）
- [x] `lab2` / `lab3` 实验脚本与 `examples/tour.txt` 回放
- [x] 43 项测试（37 单元 + 1 用例回归 + 5 端到端）
- [x] 实验指导书、设计文档、提交要点
- [x] CI（Ubuntu + Windows）、Dockerfile、BSD-3-Clause License

**验收**：`cargo test` 全绿；`cargo run -- --exec examples/tour.txt` 全程无错误；
指导书中的四个实验可在 2 学时内完成前三个。

## 第二阶段：进程与并发（初赛冲刺）

**目标**：从"内存"扩展到"任务"，覆盖 Lab4/Lab5 的核心难点。

| 任务 | 说明 | 验收 | 预估 |
| --- | --- | --- | --- |
| 任务切换可视化 | 用 `TaskControlBlock` 持有 `FrameTracker`，模拟 `fork` 时的地址空间复制；展示 TCB 的所有权树 | `task fork` 后父子页表独立；`task switch` 打印上下文切换 | 12 h |
| COW 演示 | `Rc<FrameTracker>` 共享页，写时复制；引用计数可视化 | `cow` 命令：写前计数 2，写后各自 1 | 8 h |
| `unmap` 回收空页表 | 叶子拆除后递归释放空表，上级 PTE 清零 | `map` + `unmap` 后帧数回到映射前 | 6 h |
| 大页映射 | 2 MiB 页（L1 叶子项） | `walk` 只走两级，翻译正确 | 6 h |
| 闯关模式 | 出题 → 学生输入 → 自动判分 → 进度保存 | `challenge` 命令，5 关，可重玩 | 10 h |
| 教师配置 | 通过 `lab.toml` 配置内存大小、题目、初始状态 | 无代码改动切换实验参数 | 4 h |

## 第三阶段：与真实内核联动（决赛）

**目标**：从"模拟器"升级为"课程实验平台"。

| 任务 | 说明 | 验收 |
| --- | --- | --- |
| 对照模式 | 同一操作在 rCore 与模拟器中并排展示（读取 rCore 日志/页表 dump） | `compare <op>` 输出左右两栏 |
| 地址空间实验 | 多进程独立 satp、内核页表映射 | `task` 支持多进程切换 |
| WebAssembly 版本 | 浏览器中运行，无需安装 Rust | `wasm-pack build` 产出可交互页面 |
| 在线评测 | 每个实验的自动判分与报告导出 | `cargo test --features grader` 生成 JSON |
| 课程集成 | 与学校 OS 课程实验大纲对齐，提供教师手册 | `docs/teacher-guide.md` |

## 里程碑

| 时间（相对） | 里程碑 | 交付 |
| --- | --- | --- |
| T-8 周 | M1 内存实验台 | v0.1.0（当前） |
| T-6 周 | M2 进程与并发 | v0.2.0 + 演示视频初版 |
| T-4 周 | M3 文档与视频定稿 | 初赛材料包 |
| T-2 周 | M4 冻结与答辩排练 | 决赛材料包 |
| T-0 | 答辩 | 开源仓库 + 最终文档 |

## 风险与对策

| 风险 | 影响 | 对策 |
| --- | --- | --- |
| 功能铺得太开，每个都浅 | 评审认为不完整 | 每阶段只做一个主线，其余列为挑战题 |
| 与 rCore 版本不一致 | 对照表失效 | 固定 rCore-Tutorial-Book v3 版本，文档注明 commit |
| 评委环境无 Rust | 无法复现 | Dockerfile + 预编译二进制 + 录屏 |
| 中文输出在 CI 乱码 | 测试失败 | 测试只断言 ASCII 关键词，中文仅用于展示 |
