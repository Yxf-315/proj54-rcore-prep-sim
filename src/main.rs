//! `memlab` 命令行入口。
//!
//! 四种用法：
//! - 交互模式：`memlab`
//! - 单条命令：`memlab -c "lab lab3"`
//! - 脚本回放：`memlab --exec examples/tour.txt`（可把输出直接贴进实验报告）
//! - 用例自检：`memlab --check cases`（配套测试样例，自动判定）

use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::process::ExitCode;

use rcore_memlab::cases;
use rcore_memlab::kernel::Kernel;
use rcore_memlab::repl;
use rcore_memlab::ui::Ui;

fn main() -> ExitCode {
    let mut no_color = false;
    let mut script_path: Option<String> = None;
    let mut one_cmd: Option<String> = None;
    let mut check_dir: Option<String> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            "-V" | "--version" => {
                println!("{} {}", rcore_memlab::NAME, rcore_memlab::VERSION);
                return ExitCode::SUCCESS;
            }
            "--no-color" => no_color = true,
            "--exec" => {
                let Some(path) = args.next() else {
                    eprintln!("错误：--exec 后面需要脚本路径");
                    return ExitCode::FAILURE;
                };
                script_path = Some(path);
            }
            "--check" => {
                let Some(dir) = args.next() else {
                    eprintln!("错误：--check 后面需要用例目录（例如 --check cases）");
                    return ExitCode::FAILURE;
                };
                check_dir = Some(dir);
            }
            "-c" | "--cmd" => {
                let Some(cmd) = args.next() else {
                    eprintln!("错误：-c 后面需要一条命令");
                    return ExitCode::FAILURE;
                };
                one_cmd = Some(cmd);
            }
            other if !other.starts_with('-') => script_path = Some(other.to_string()),
            other => {
                eprintln!("错误：未知参数 `{other}`。用 --help 查看用法。");
                return ExitCode::FAILURE;
            }
        }
    }

    // 输出被重定向或显式 --no-color 时关闭颜色，保证日志干净。
    let color = !no_color && io::stdout().is_terminal();
    let ui = Ui::new(color);

    // 用例模式不需要预先创建内核：每个用例都会在干净的实验台上运行。
    if let Some(dir) = check_dir {
        return run_checks(&dir, ui);
    }

    let mut kernel = Kernel::new(ui);

    if let Some(path) = script_path {
        return run_script(&mut kernel, &path, ui);
    }

    if let Some(cmd) = one_cmd {
        return match kernel.exec(&cmd) {
            Ok(text) => {
                if !text.is_empty() {
                    println!("{text}");
                }
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{} {message}", ui.err("错误："));
                ExitCode::FAILURE
            }
        };
    }

    repl::run(&mut kernel);
    ExitCode::SUCCESS
}

fn run_checks(dir: &str, ui: Ui) -> ExitCode {
    match cases::run_dir(Path::new(dir), ui) {
        Ok(results) => {
            let failed = results.iter().filter(|outcome| !outcome.passed).count();
            for outcome in &results {
                if outcome.passed {
                    println!("{} {}", ui.ok("通过"), outcome.name);
                } else {
                    println!("{} {}", ui.err("失败"), outcome.name);
                    for want in &outcome.missing {
                        println!("    缺少期望片段：{want}");
                    }
                }
            }
            println!();
            println!(
                "共 {} 个用例：{} 通过，{} 失败",
                results.len(),
                results.len() - failed,
                failed
            );
            if failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!("{} {err}", ui.err("错误："));
            ExitCode::FAILURE
        }
    }
}

fn run_script(kernel: &mut Kernel, path: &str, ui: Ui) -> ExitCode {
    let script = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("错误：无法读取脚本 {path}：{err}");
            return ExitCode::FAILURE;
        }
    };

    let mut failed = false;
    for line in script.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        println!("{}", ui.dim(&format!("> {line}")));
        match kernel.exec(line) {
            Ok(text) => {
                if !text.is_empty() {
                    println!("{text}");
                }
            }
            Err(message) => {
                eprintln!("{} {message}", ui.err("错误："));
                failed = true;
            }
        }
        if kernel.quit_requested() {
            break;
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_help() {
    println!(
        "rcore-memlab {} —— rCore 内存实验室（Sv39 地址翻译预习模拟器）\n\
         \n\
         用法:\n\
         \x20 memlab                       进入交互模式\n\
         \x20 memlab -c \"lab lab3\"        执行一条命令后退出\n\
         \x20 memlab --exec <脚本>         回放脚本（每行一条命令，# 开头为注释）\n\
         \x20 memlab --check <目录>        运行配套测试样例（*.case）并判定\n\
         \n\
         选项:\n\
         \x20 -h, --help      显示本帮助\n\
         \x20 -V, --version   显示版本\n\
         \x20     --no-color  关闭 ANSI 颜色\n\
         \n\
         示例:\n\
         \x20 memlab --exec examples/tour.txt\n\
         \x20 memlab --check cases\n\
         \x20 memlab -c \"walk 0x1000\"\n",
        rcore_memlab::VERSION
    );
}
