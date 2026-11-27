//! 校验仓库自带的 `.case` 用例全部通过。
//!
//! 这些用例是申报书里的"配套测试样例"：每个用例对应 rCore 实验的一个观察点，
//! 任何改动导致输出语义变化，这里都会第一时间失败。

use std::path::Path;

use rcore_memlab::cases;
use rcore_memlab::ui::Ui;

#[test]
fn bundled_cases_all_pass() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("cases");
    let results = cases::run_dir(&dir, Ui::new(false)).expect("无法运行用例目录");

    assert!(results.len() >= 8, "用例数量不足：{}", results.len());
    for outcome in &results {
        assert!(
            outcome.passed,
            "用例 `{}` 失败，缺少期望片段：{:?}",
            outcome.name, outcome.missing
        );
    }
}
