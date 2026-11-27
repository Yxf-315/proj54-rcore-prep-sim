# 提交材料说明 · proj54 SynRun

本目录是 2026 华北区域赛（功能挑战赛道）的提交材料包。

## 目录

| 文件 | 内容 | 对应评审项 |
| --- | --- | --- |
| `01-source-link.md` | GitHub 仓库地址（主）、commit、构建命令 | 源代码 |
| `02-design.docx` / `.pdf` | 设计实现文档（11 页） | 文档 30% |
| `03-usage.docx` / `.pdf` | 使用说明（6 页） | 使用文档 |
| `04-lab-guide.docx` / `.pdf` | 实验指导书（7 页） | 教学价值 10% |
| `05-slides.pptx` / `.pdf` | 汇报幻灯片（12 页） | 演示答辩 20% |
| `06-demo.mp4` | 演示视频（**待录制**，脚本见 `docs/video-script.md`） | 演示答辩 20% |
| `07-trial-record.xlsx` | 教学试用记录表（**待填真实数据**） | 教学验证 |
| `08-defense-handbook.docx` / `.pdf` | 答辩手册（8 分钟流程 + 20 问） | 答辩准备 |
| `09-license.txt` | 开源协议说明 | 规范 10% |

## 生成方式（可复现）

```bash
# 文档
python tools/md2docx.py --input docs/design.md --output submission/02-design.docx \
  --title "设计实现文档" --subtitle "proj54 · 基于 Rust 的 rCore-Tutorial 配套预习模拟器" ...
# 幻灯片
python tools/build_deck.py --output submission/05-slides.pptx
# 试用表
python tools/build_trial_xlsx.py --output submission/07-trial-record.xlsx
```

DOCX → PDF 使用赛方/本机 LibreOffice：

```bash
soffice --headless --convert-to pdf submission/02-design.docx
```

## 提交前必改

1. `01-source-link.md` 中的仓库地址与 commit hash；
2. `07-trial-record.xlsx` 中的试用数据（并把 `02-design.pdf` §13 重新导出）；
3. 文档封面日期与队伍信息；
4. 视频文件放入本目录。

## 许可

代码 BSD-3-Clause；文档、幻灯片、视频 CC BY-SA 4.0。
