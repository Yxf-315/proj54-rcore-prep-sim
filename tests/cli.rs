//! 端到端测试：直接调用编译出来的 `memlab` 二进制。
//!
//! 单元测试覆盖 `Kernel::exec` 的状态变化，这里再验证参数解析、
//! 脚本回放与退出码，保证"用户实际执行的命令"也是对的。

use std::process::Command;

fn memlab() -> Command {
    Command::new(env!("CARGO_BIN_EXE_memlab"))
}

#[test]
fn status_prints_memory_layout() {
    let out = memlab()
        .args(["-c", "status", "--no-color"])
        .output()
        .expect("无法启动 memlab");

    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("物理内存"));
    assert!(stdout.contains("0x8000_0000"));
    assert!(stdout.contains("satp"));
}

#[test]
fn tour_script_replays_successfully() {
    let out = memlab()
        .args(["--exec", "examples/tour.txt", "--no-color"])
        .output()
        .expect("无法启动 memlab");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "脚本回放失败：{stderr}");

    let stdout = String::from_utf8_lossy(&out.stdout);
    for expected in [
        "Lab2 · 物理帧分配器",
        "Lab3 · Sv39 页表与地址翻译",
        "翻译结果",
        "权限异常",
        "缺页异常",
        "recycled 栈的 LIFO 行为",
    ] {
        assert!(stdout.contains(expected), "输出里缺少：{expected}");
    }
}

#[test]
fn unknown_command_exits_with_failure() {
    let out = memlab()
        .args(["-c", "definitely-not-a-command", "--no-color"])
        .output()
        .expect("无法启动 memlab");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("未知命令"));
}

#[test]
fn check_mode_passes_bundled_cases() {
    let out = memlab()
        .args(["--check", "cases", "--no-color"])
        .output()
        .expect("无法启动 memlab");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "用例模式失败：{stdout}");
    assert!(stdout.contains("通过"));
    assert!(stdout.contains("0 失败"));
}

#[test]
fn help_and_version_work() {
    let out = memlab().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("rcore-memlab"));

    let out = memlab().arg("--help").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("--exec"));
}
