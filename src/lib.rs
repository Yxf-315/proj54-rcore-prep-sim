//! # rCore 内存实验室（rcore-memlab）
//!
//! 一个零依赖的 Rust 教学实验台：在用户态复刻 rCore-Tutorial 的
//! **物理帧分配器**（Lab2）与 **Sv39 三级页表**（Lab3），
//! 并用手册式交互把 Rust 的**所有权 / 借用 / RAII** 与内核资源管理一一对应。
//!
//! 设计目标：
//! 1. 代码可读：每个模块都能与 rCore 的源码文件对上号；
//! 2. 行为可验证：分配、翻译、缺页都是真实计算出来的，不是打印好的台词；
//! 3. 结果可复现：`Kernel::exec` 是纯状态机接口，便于脚本与自动化测试。
//!
//! 模块地图：
//! - [`memory`]：物理地址 / 虚拟地址 / 物理内存（对应 `os/src/mm/address.rs`）
//! - [`frame`]  ：栈式物理帧分配器与 RAII 句柄（对应 `os/src/mm/frame_allocator.rs`）
//! - [`paging`] ：Sv39 页表与地址翻译（对应 `os/src/mm/page_table.rs`）
//! - [`ownership`]：所有权演示脚本
//! - [`kernel`] ：命令状态机（把上面三者组装成可交互实验）
//! - [`ui`]     ：终端表格与配色
//! - [`repl`]   ：交互式命令行

pub mod cases;
pub mod frame;
pub mod kernel;
pub mod memory;
pub mod ownership;
pub mod paging;
pub mod repl;
pub mod ui;

/// 版本号，供 `--version` 与启动横幅使用。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 项目名。
pub const NAME: &str = env!("CARGO_PKG_NAME");
