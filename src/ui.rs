//! 终端渲染：表格、分隔线、配色。
//!
//! 表格宽度按**显示宽度**计算（中日韩字符占两列），这样中英混排也不会错位。
//! 颜色默认开启，`--no-color` 或输出重定向时会自动关闭。

/// 终端 UI 配置。
#[derive(Clone, Copy)]
pub struct Ui {
    color: bool,
}

impl Ui {
    pub const fn new(color: bool) -> Self {
        Self { color }
    }

    pub const fn color_enabled(self) -> bool {
        self.color
    }

    fn paint(self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn title(self, text: &str) -> String {
        self.paint("1;36", text)
    }

    pub fn ok(self, text: &str) -> String {
        self.paint("1;32", text)
    }

    pub fn err(self, text: &str) -> String {
        self.paint("1;31", text)
    }

    pub fn warn(self, text: &str) -> String {
        self.paint("1;33", text)
    }

    pub fn dim(self, text: &str) -> String {
        self.paint("2", text)
    }

    pub fn bold(self, text: &str) -> String {
        self.paint("1", text)
    }

    /// 一级标题。
    pub fn section(self, text: &str) -> String {
        format!("{} {}", self.title("=="), self.title(text))
    }

    /// 渲染一张 ASCII 表格。
    pub fn table(self, headers: &[&str], rows: &[Vec<String>]) -> String {
        let cols = headers.len();
        let mut widths: Vec<usize> = headers.iter().map(|h| display_width(h)).collect();
        for row in rows {
            for (i, cell) in row.iter().enumerate().take(cols) {
                widths[i] = widths[i].max(display_width(cell));
            }
        }

        let mut out = String::new();
        out.push_str(&border(&widths));
        out.push('\n');

        out.push('|');
        for (i, header) in headers.iter().enumerate() {
            out.push(' ');
            out.push_str(&pad(header, widths[i]));
            out.push_str(" |");
        }
        out.push('\n');
        out.push_str(&border(&widths));
        out.push('\n');

        for row in rows {
            out.push('|');
            for (i, width) in widths.iter().enumerate().take(cols) {
                let cell = row.get(i).map(String::as_str).unwrap_or("");
                out.push(' ');
                out.push_str(&pad(cell, *width));
                out.push_str(" |");
            }
            out.push('\n');
        }
        out.push_str(&border(&widths));
        out
    }

    /// 两列键值表。
    pub fn kv(self, rows: &[(&str, String)]) -> String {
        let rows: Vec<Vec<String>> = rows
            .iter()
            .map(|(k, v)| vec![k.to_string(), v.clone()])
            .collect();
        self.table(&["项", "值"], &rows)
    }
}

fn border(widths: &[usize]) -> String {
    let mut s = String::from("+");
    for w in widths {
        s.push_str(&"-".repeat(w + 2));
        s.push('+');
    }
    s
}

/// 按显示宽度右补空格。
fn pad(text: &str, width: usize) -> String {
    let mut out = String::from(text);
    let w = display_width(text);
    if w < width {
        out.push_str(&" ".repeat(width - w));
    }
    out
}

/// 字符串在等宽终端中的显示宽度（CJK 字符按 2 列计）。
pub fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

fn char_width(ch: char) -> usize {
    let c = ch as u32;
    let wide = matches!(c,
        0x1100..=0x115F
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE6F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1F64F
        | 0x1F900..=0x1F9FF
    );
    if wide {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_counts_two_columns() {
        assert_eq!(display_width("帧"), 2);
        assert_eq!(display_width("PPN"), 3);
        assert_eq!(display_width("帧 PPN"), 6);
    }

    #[test]
    fn table_is_aligned() {
        let ui = Ui::new(false);
        let out = ui.table(&["级别", "V"], &[vec!["L2".into(), "1".into()]]);
        let lines: Vec<&str> = out.lines().collect();
        // 比较显示宽度而不是字符数：CJK 字符占两列。
        assert_eq!(display_width(lines[0]), display_width(lines[1]));
        assert_eq!(display_width(lines[1]), display_width(lines[3]));
    }

    #[test]
    fn color_can_be_disabled() {
        let ui = Ui::new(false);
        assert_eq!(ui.ok("x"), "x");
        assert_eq!(Ui::new(true).ok("x"), "\x1b[1;32mx\x1b[0m");
    }
}
