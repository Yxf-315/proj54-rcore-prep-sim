//! 配套测试样例：用 `.case` 文件描述"输入命令 + 期望输出片段"。
//!
//! 这是申报书里"设计配套测试样例，对照 rCore 实验思路"的落地：
//! 每个用例对应 rCore 实验的一个观察点（帧分配、页表遍历、缺页、权限、所有权），
//! 由 `memlab --check cases` 自动执行并判分。
//!
//! 文件格式（UTF-8，`#` 开头为注释）：
//!
//! ```text
//! name: 用例名称
//! cmd:  map 0x1000 auto rw
//! cmd:  walk 0x1000
//! want: 翻译结果
//! want: L0
//! ```
//!
//! 判分规则：在一个**全新的** `Kernel` 上依次执行 `cmd`，把输出拼起来，
//! 每条 `want` 都必须是其中的子串；全部命中才算通过。

use std::fs;
use std::path::Path;

use crate::kernel::Kernel;
use crate::ui::Ui;

/// 一个测试用例。
#[derive(Debug, Clone)]
pub struct Case {
    pub name: String,
    pub commands: Vec<String>,
    pub wants: Vec<String>,
}

/// 用例执行结果。
#[derive(Debug, Clone)]
pub struct CaseOutcome {
    pub name: String,
    pub passed: bool,
    pub missing: Vec<String>,
}

impl Case {
    /// 解析用例文本。
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut name: Option<String> = None;
        let mut commands = Vec::new();
        let mut wants = Vec::new();

        for (idx, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                return Err(format!("第 {} 行缺少 `:`：{line}", idx + 1));
            };
            let value = value.trim();
            if value.is_empty() {
                return Err(format!("第 {} 行的值为空：{line}", idx + 1));
            }
            match key.trim() {
                "name" => name = Some(value.to_string()),
                "cmd" | "command" => commands.push(value.to_string()),
                "want" | "expect" => wants.push(value.to_string()),
                other => return Err(format!("第 {} 行未知字段 `{other}`", idx + 1)),
            }
        }

        let name = name.ok_or_else(|| "缺少 `name:`".to_string())?;
        if commands.is_empty() {
            return Err(format!("{name}：至少需要一条 `cmd:`"));
        }
        if wants.is_empty() {
            return Err(format!("{name}：至少需要一条 `want:`"));
        }
        Ok(Self {
            name,
            commands,
            wants,
        })
    }

    /// 在全新的实验台上运行本用例。
    pub fn run(&self, ui: Ui) -> CaseOutcome {
        let mut kernel = Kernel::new(ui);
        let mut output = String::new();
        for command in &self.commands {
            match kernel.exec(command) {
                Ok(text) => {
                    output.push_str(&text);
                    output.push('\n');
                }
                Err(err) => {
                    output.push_str("错误：");
                    output.push_str(&err);
                    output.push('\n');
                }
            }
        }
        let missing: Vec<String> = self
            .wants
            .iter()
            .filter(|want| !output.contains(want.as_str()))
            .cloned()
            .collect();
        CaseOutcome {
            name: self.name.clone(),
            passed: missing.is_empty(),
            missing,
        }
    }
}

/// 运行目录下全部 `*.case`（按文件名排序，保证输出稳定）。
pub fn run_dir(dir: &Path, ui: Ui) -> Result<Vec<CaseOutcome>, String> {
    let entries =
        fs::read_dir(dir).map_err(|err| format!("无法读取用例目录 {}：{err}", dir.display()))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("读取目录项失败：{err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("case") {
            files.push(path);
        }
    }
    files.sort();

    if files.is_empty() {
        return Err(format!("{} 下没有找到任何 .case 文件", dir.display()));
    }

    let mut outcomes = Vec::new();
    for path in files {
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("无法读取 {}：{err}", path.display()))?;
        let case = Case::parse(&text).map_err(|err| format!("{}：{err}", path.display()))?;
        outcomes.push(case.run(ui));
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_case() {
        let case = Case::parse("name: 测试\ncmd: lab lab2\nwant: LIFO\n").unwrap();
        assert_eq!(case.name, "测试");
        assert_eq!(case.commands, vec!["lab lab2"]);
        assert_eq!(case.wants, vec!["LIFO"]);
    }

    #[test]
    fn rejects_unknown_field() {
        assert!(Case::parse("name: x\nbogus: y\ncmd: status\n").is_err());
    }

    #[test]
    fn rejects_empty_commands() {
        assert!(Case::parse("name: x\nwant: y\n").is_err());
    }

    #[test]
    fn case_passes_on_expected_output() {
        let case = Case::parse("name: 状态\ncmd: status\nwant: satp\n").unwrap();
        let outcome = case.run(Ui::new(false));
        assert!(outcome.passed, "缺少 {:?}", outcome.missing);
    }

    #[test]
    fn case_fails_on_missing_output() {
        let case = Case::parse("name: 反例\ncmd: status\nwant: 这段文字不会出现\n").unwrap();
        let outcome = case.run(Ui::new(false));
        assert!(!outcome.passed);
        assert_eq!(outcome.missing.len(), 1);
    }
}
