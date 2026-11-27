# 使用说明 · rcore-memlab

> 项目：**proj54 · 基于 Rust 的 rCore-Tutorial 配套预习模拟器**
> 队伍：SynRun（北京工商大学）
> 核心知识点：**Sv39 分页地址翻译（rCore-Tutorial Lab3）**；物理帧分配器（Lab2）作为页表的物理内存支撑

---

## 1. 这个工具解决什么问题

初学者直接上手 rCore 的 Lab3 时，通常卡在三件事：

1. 页表项（PTE）里到底存了什么，`VPN[2]/VPN[1]/VPN[0]` 怎么从虚拟地址里切出来；
2. 地址翻译走哪几个物理地址，为什么缺页会发生在"第 2 级"而不是"第 0 级"；
3. Rust 的所有权/借用和内核资源管理到底有什么关系。

本工具在用户态复刻 rCore 的帧分配器与三级页表：输入一个虚拟地址，就能看到逐级遍历的
PTE 原始值、权限位和最终物理地址；输入一次访存，就能看到缺页/权限异常发生在哪一级。
**先在模拟器上把原理跑通，再进 QEMU 做实验**，降低学习断层。

## 2. 环境要求

| 项目 | 要求 |
| --- | --- |
| 操作系统 | Windows 10/11、Linux、macOS |
| Rust | 1.75 及以上（`cargo --version` 可查看） |
| 第三方依赖 | **无**（离线可构建） |
| 终端 | 支持 ANSI 颜色即可；不支持时加 `--no-color` |

## 3. 安装

```bash
git clone https://github.com/<你的 GitHub 用户名>/proj54-rcore-prep-sim.git
cd proj54-rcore-prep-sim
cargo build --release
```

产物在 `target/release/memlab`（Windows 为 `memlab.exe`），可以直接拷贝到没有 Rust 环境的机器运行。

Docker 方式（评审现场兜底）：

```bash
docker build -t rcore-memlab .
docker run --rm -it rcore-memlab
```

## 4. 四种运行方式

```bash
memlab                              # 交互模式，输入 help 查看命令
memlab -c "lab lab3"                # 单条命令，适合脚本/录屏
memlab --exec examples/tour.txt     # 回放脚本，输出可直接贴进实验报告
memlab --check cases                # 运行配套测试样例并判定
```

`--exec` 与 `--check` 的退出码：0 表示全部成功，非 0 表示有命令报错或用例失败，便于 CI 使用。

## 5. 命令与参数

| 命令 | 参数 | 作用 | 示例 |
| --- | --- | --- | --- |
| `alloc` | `[n]`，1–64，默认 1 | 分配 n 个物理帧 | `alloc 3` |
| `free` | `<ppn>`，支持 `0x` 与 `_` | 释放指定物理帧 | `free 0x8_0002` |
| `map` | `<va> <pa\|auto> [rwxu]` | 建立 4 KiB 页映射 | `map 0x1000 auto rw` |
| `unmap` | `<va>` | 拆除映射 | `unmap 0x1000` |
| `walk` | `<va>` | 逐级遍历并翻译 | `walk 0x1000` |
| `fault` | `<va> [r\|w\|x]` | 模拟一次访存 | `fault 0x2000 w` |
| `demo` | `move\|clone\|raii\|leak\|borrow\|dangling\|double-free` | 所有权演示 | `demo raii` |
| `lab` | `lab2\|lab3` | 实验脚本 | `lab lab3` |
| `status` | — | 物理内存/分配器/页表总览 | `status` |
| `frames` | — | 分配器统计 | `frames` |
| `reset` | — | 重建空白页表 | `reset` |
| `help` | `[命令]` | 帮助 | `help map` |
| `quit` | — | 退出 | `quit` |

权限位写法：`r` 可读、`w` 可写、`x` 可执行、`u` 用户态可访问，例如 `rw`、`rxu`。

## 6. 五分钟上手

### 6.1 物理帧分配（Lab2）

```text
memlab> alloc 3
已分配 3 个物理帧（FrameTracker 持有，Drop 时自动归还）：
| 序号 | PPN      | 物理地址    |
| 1    | 0x8_0001 | 0x8000_1000 |
| 2    | 0x8_0002 | 0x8000_2000 |
| 3    | 0x8_0003 | 0x8000_3000 |
```

要点：PPN × 4096 = 物理地址；分配只推进水位线，释放进入回收栈（LIFO）。

### 6.2 建立映射并遍历（Lab3）

```text
memlab> map 0x1000 auto rw
映射建立成功：VA 0x0000_1000 → PA 0x8000_5000（自动分配到 PPN 0x8_0005）
memlab> walk 0x1000
| 级别 | VPN 索引 | PTE 地址    | PTE 值                | V | R W X | 下一级 / 叶子    |
| L2   | 0        | 0x8000_0000 | 0x0000_0000_2000_1801 | 1 | - - - | 0x8000_6000      |
| L1   | 0        | 0x8000_6000 | 0x0000_0000_2000_1c01 | 1 | - - - | 0x8000_7000      |
| L0   | 1        | 0x8000_7008 | 0x0000_0000_2000_1407 | 1 | R W - | 叶子（4 KiB 页） |
✓ 翻译结果：VA 0x0000_1000 → PA 0x8000_5000（读访问成功）
```

手工验证：`0x1000 >> 12 = 1`，所以 `VPN[0] = 1`，`VPN[1] = VPN[2] = 0`。

### 6.3 缺页与权限

```text
memlab> map 0x2000 auto r
memlab> fault 0x2000 w
✗ 触发异常 权限异常：VA=0x0000_2000 尝试写，但 L0 的 PTE 权限位不允许
memlab> fault 0x9000
✗ 触发异常 缺页异常：VA=0x0009_0000 在第 2 级页表（L2）处 PTE 无效（V=0）
```

### 6.4 所有权

```text
memlab> demo raii        # Drop 自动归还物理帧
memlab> demo double-free # 为什么安全 Rust 写不出双重释放
```

## 7. 配套测试样例

用例放在 `cases/*.case`，由 `memlab --check cases` 自动执行。格式：

```text
name: 用例名称
cmd:  map 0x1000 auto rw
cmd:  walk 0x1000
want: 翻译结果
want: L0
```

- `#` 开头为注释；
- 每个用例在**全新**的实验台上运行（互不影响）；
- 所有 `want` 都是输出子串，全部命中即通过。

自带 8 个用例，对应 rCore 实验的观察点：

| 文件 | 对应知识点 | 观察点 |
| --- | --- | --- |
| `01_frame_alloc.case` | Lab2 分配器 | PPN 递增、PPN × 4096 = PA |
| `02_raii.case` | `FrameTracker::drop` | 作用域结束自动回收 |
| `03_walk_basic.case` | Lab3 页表遍历 | 三级 PTE 与翻译结果 |
| `04_fault_perm.case` | 权限异常 | 只读页写访问在 L0 被拒 |
| `05_fault_unmapped.case` | 缺页 | 缺页定位到 L2 |
| `06_ownership_move.case` | Rust move | E0382 与自动归还 |
| `07_lab2_lifo.case` | 回收栈 | 释放的帧按 LIFO 复用 |
| `08_lab3_sv39.case` | Lab3 全流程 | 映射/遍历/缺页/unmap |

新增用例：复制一个 `.case`，改 `name/cmd/want`，再运行 `memlab --check cases` 验证。

## 8. 与 rCore 源码的对应关系

| 本工具 | rCore-Tutorial | 说明 |
| --- | --- | --- |
| `src/memory.rs` 的 `PhysAddr`/`VirtAddr` | `os/src/mm/address.rs` | 地址 newtype，本工具增加 Sv39 上界检查 |
| `src/frame.rs` 的 `FrameAllocator::alloc/dealloc` | `os/src/mm/frame_allocator.rs` | 水位线 `current` + 回收栈 |
| `src/frame.rs` 的 `FrameTracker::drop` | 同名 | 独占所有权，`!Clone` |
| `src/paging.rs` 的 `PageTable::map/unmap` | `os/src/mm/page_table.rs` | 页表帧从分配器申请，逐级写 PTE |
| `src/paging.rs` 的 `translate` | `find_pte` + 硬件 MMU | 逐级读内存，置 A/D 位 |
| `fault` 命令的提示 | `os/src/trap/mod.rs` | 对应 `trap_handler` 的缺页分派 |
| `demo` 命令 | Rustlings `move_semantics`/`smart_pointers`/`lifetimes` | 所有权与内核资源生命周期 |

## 9. 常见问题

**Q：`cargo run` 提示找不到 `cargo`？**
A：安装 Rust（<https://rustup.rs>），或直接用 `target/release/memlab`。

**Q：输出里有乱码/颜色符号？**
A：加 `--no-color`；Windows 旧版控制台建议用 Windows Terminal。

**Q：`map` 报地址未对齐？**
A：4 KiB 页要求 VA、PA 都是 4096 的倍数。

**Q：`free` 报"不在持有列表中"？**
A：只有 `alloc` / `map auto` 分配给实验台的帧才能释放，页表内部帧由页表管理。

**Q：`--check` 失败怎么看原因？**
A：输出会列出"缺少期望片段"，对照 `cases/*.case` 修改命令或期望即可。

## 10. 许可

源代码 BSD-3-Clause；文档、幻灯片与演示视频 CC BY-SA 4.0。
