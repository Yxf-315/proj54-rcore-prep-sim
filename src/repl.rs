//! 交互式命令行（REPL）。

use std::io::{self, BufRead, Write};

use crate::kernel::Kernel;

/// 进入交互模式，直到用户输入 `quit` 或输入流结束（Ctrl-D / Ctrl-Z）。
pub fn run(kernel: &mut Kernel) {
    print_banner(kernel);

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        print!("{}", kernel.ui().title("memlab> "));
        let _ = io::stdout().flush();

        let Some(line) = lines.next() else {
            println!();
            break;
        };
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                println!();
                break;
            }
        };

        match kernel.exec(&line) {
            Ok(text) => {
                if !text.is_empty() {
                    println!("{text}");
                }
            }
            Err(message) => println!("{} {message}", kernel.ui().err("错误：")),
        }

        if kernel.quit_requested() {
            break;
        }
    }
}

fn print_banner(kernel: &Kernel) {
    let ui = kernel.ui();
    println!();
    println!("{}", ui.title("  rcore-memlab  ·  rCore 内存实验室"));
    println!(
        "  {}",
        ui.dim("用 Rust 所有权讲透 rCore Lab2/Lab3 的物理帧与 Sv39 页表")
    );
    println!();
    println!(
        "  输入 {} 查看命令，或直接运行 {} / {}。",
        ui.bold("help"),
        ui.bold("lab lab2"),
        ui.bold("lab lab3")
    );
    println!();
}
