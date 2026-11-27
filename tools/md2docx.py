#!/usr/bin/env python3
"""把项目里的 Markdown 文档转换成排版好的 DOCX（供导出 PDF）。

只覆盖项目文档实际用到的语法：标题、段落、无序/有序列表、GFM 表格、
代码块、引用、水平线、**加粗**、`行内代码`。

用法：
    python md2docx.py --input docs/design.md --output submission/02-design.docx \
        --title "设计实现文档" --subtitle "proj54 · 基于 Rust 的 rCore-Tutorial 配套预习模拟器"
"""

from __future__ import annotations

import argparse
import re
from datetime import date
from pathlib import Path

from docx import Document
from docx.enum.section import WD_SECTION
from docx.enum.table import WD_TABLE_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Cm, Pt, RGBColor

BODY_FONT = "Microsoft YaHei"
CODE_FONT = "Consolas"
ACCENT = RGBColor(0x1F, 0x3A, 0x5F)
MUTED = RGBColor(0x66, 0x66, 0x66)


def set_run_font(run, name: str = BODY_FONT, size: float | None = None,
                 bold: bool | None = None, color: RGBColor | None = None) -> None:
    run.font.name = name
    rpr = run._element.get_or_add_rPr()
    rfonts = rpr.find(qn("w:rFonts"))
    if rfonts is None:
        rfonts = OxmlElement("w:rFonts")
        rpr.append(rfonts)
    rfonts.set(qn("w:eastAsia"), name)
    rfonts.set(qn("w:ascii"), name)
    rfonts.set(qn("w:hAnsi"), name)
    if size is not None:
        run.font.size = Pt(size)
    if bold is not None:
        run.bold = bold
    if color is not None:
        run.font.color.rgb = color


def style_document(doc: Document) -> None:
    section = doc.sections[0]
    section.page_width = Cm(21.0)
    section.page_height = Cm(29.7)
    section.top_margin = Cm(2.0)
    section.bottom_margin = Cm(2.0)
    section.left_margin = Cm(2.0)
    section.right_margin = Cm(2.0)

    normal = doc.styles["Normal"]
    normal.font.name = BODY_FONT
    normal.font.size = Pt(10.5)
    normal.element.rPr.rFonts.set(qn("w:eastAsia"), BODY_FONT)
    normal.paragraph_format.line_spacing = 1.45
    normal.paragraph_format.space_after = Pt(6)

    for level, size in ((1, 16), (2, 13.5), (3, 11.5)):
        style = doc.styles[f"Heading {level}"]
        style.font.name = BODY_FONT
        style.font.size = Pt(size)
        style.font.bold = True
        style.font.color.rgb = ACCENT
        style.element.rPr.rFonts.set(qn("w:eastAsia"), BODY_FONT)
        style.paragraph_format.space_before = Pt(14 if level == 1 else 10)
        style.paragraph_format.space_after = Pt(6)


def add_cover(doc: Document, title: str, subtitle: str, meta: list[str]) -> None:
    for _ in range(6):
        doc.add_paragraph()
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    set_run_font(p.add_run(title), size=30, bold=True, color=ACCENT)

    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    set_run_font(p.add_run(subtitle), size=12.5, color=MUTED)

    doc.add_paragraph()
    for line in meta:
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        set_run_font(p.add_run(line), size=11)

    doc.add_paragraph()
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    set_run_font(p.add_run(date.today().strftime("%Y 年 %m 月")), size=10, color=MUTED)

    doc.add_page_break()


INLINE = re.compile(r"(\*\*.+?\*\*|`[^`]+`)")
LINK = re.compile(r"\[([^\]]+)\]\(([^)]+)\)")


def add_inline(paragraph, text: str, size: float = 10.5) -> None:
    # 链接只保留可读文字，避免 PDF 里出现 Markdown 语法
    text = LINK.sub(r"\1", text)
    for part in INLINE.split(text):
        if not part:
            continue
        if part.startswith("**") and part.endswith("**") and len(part) > 4:
            set_run_font(paragraph.add_run(part[2:-2]), size=size, bold=True)
        elif part.startswith("`") and part.endswith("`") and len(part) > 2:
            set_run_font(paragraph.add_run(part[1:-1]), name=CODE_FONT, size=size - 0.5,
                         color=RGBColor(0xB0, 0x30, 0x30))
        else:
            set_run_font(paragraph.add_run(part), size=size)


def add_code_block(doc: Document, lines: list[str]) -> None:
    p = doc.add_paragraph()
    p.paragraph_format.left_indent = Cm(0.2)
    p.paragraph_format.space_before = Pt(4)
    p.paragraph_format.space_after = Pt(8)
    p.paragraph_format.line_spacing = 1.0
    p.paragraph_format.keep_together = True
    run = p.add_run("\n".join(lines))
    set_run_font(run, name=CODE_FONT, size=7.5)
    ppr = p._p.get_or_add_pPr()
    shd = OxmlElement("w:shd")
    shd.set(qn("w:val"), "clear")
    shd.set(qn("w:fill"), "F4F5F7")
    ppr.append(shd)


def _display_len(text: str) -> int:
    return sum(2 if ord(ch) > 0x2E7F else 1 for ch in text)


def add_table(doc: Document, rows: list[list[str]]) -> None:
    cols = max(len(r) for r in rows)
    table = doc.add_table(rows=len(rows), cols=cols)
    table.style = "Table Grid"
    table.alignment = WD_TABLE_ALIGNMENT.CENTER
    table.autofit = True

    # 按内容长度分配列宽（总宽约 16 cm），避免出现一列很宽、一列很挤。
    weights = []
    for j in range(cols):
        longest = max(_display_len(r[j]) if j < len(r) else 0 for r in rows)
        weights.append(max(longest, 4))
    total = sum(weights)
    widths = [max(1.7, 16.2 * w / total) for w in weights]

    for i, row in enumerate(rows):
        for j in range(cols):
            text = row[j] if j < len(row) else ""
            cell = table.cell(i, j)
            cell.width = Cm(widths[j])
            cell.text = ""
            p = cell.paragraphs[0]
            p.paragraph_format.space_after = Pt(0)
            add_inline(p, text, size=9)
            if i == 0:
                for run in p.runs:
                    run.bold = True
        # 表头跨页重复，行内容不跨页断开
        tr_pr = table.rows[i]._tr.get_or_add_trPr()
        cant_split = OxmlElement("w:cantSplit")
        tr_pr.append(cant_split)
        if i == 0:
            header = OxmlElement("w:tblHeader")
            header.set(qn("w:val"), "true")
            tr_pr.append(header)

    doc.add_paragraph().paragraph_format.space_after = Pt(2)


def split_row(line: str) -> list[str]:
    """按未转义的 `|` 切分表格行，`\\|` 视为单元格内的字面竖线。"""
    s = line.strip()
    if s.startswith("|"):
        s = s[1:]
    if s.endswith("|") and not s.endswith("\\|"):
        s = s[:-1]
    cells: list[str] = []
    current: list[str] = []
    escaped = False
    for ch in s:
        if escaped:
            current.append(ch)
            escaped = False
        elif ch == "\\":
            escaped = True
        elif ch == "|":
            cells.append("".join(current))
            current = []
        else:
            current.append(ch)
    cells.append("".join(current))
    return [cell.strip() for cell in cells]


def convert(md_text: str, doc: Document) -> None:
    lines = md_text.splitlines()
    i = 0
    seen_h1 = False
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        if stripped.startswith("```"):
            i += 1
            block = []
            while i < len(lines) and not lines[i].strip().startswith("```"):
                block.append(lines[i])
                i += 1
            add_code_block(doc, block)
            i += 1
            continue

        if stripped.startswith("|") and i + 1 < len(lines) and re.match(
            r"^\|[\s:\-|]+\|$", lines[i + 1].strip()
        ):
            rows = []
            rows.append(split_row(stripped))
            i += 2
            while i < len(lines) and lines[i].strip().startswith("|"):
                rows.append(split_row(lines[i]))
                i += 1
            add_table(doc, rows)
            continue

        if stripped.startswith("#"):
            level = len(stripped) - len(stripped.lstrip("#"))
            text = stripped[level:].strip()
            if level == 1:
                if seen_h1:
                    doc.add_page_break()
                seen_h1 = True
            doc.add_heading(text, level=min(level, 3))
            i += 1
            continue

        if stripped in ("---", "***", "___"):
            i += 1
            continue

        if stripped.startswith(">"):
            p = doc.add_paragraph()
            p.paragraph_format.left_indent = Cm(0.4)
            add_inline(p, stripped.lstrip("> ").strip(), size=9.5)
            for run in p.runs:
                run.font.color.rgb = MUTED
            i += 1
            continue

        if stripped.startswith(("- ", "* ")):
            p = doc.add_paragraph(style="List Bullet")
            add_inline(p, stripped[2:].strip())
            i += 1
            continue

        if re.match(r"^\d+\.\s", stripped):
            # 不依赖 Word 的自动编号（跨列表会连续编号），保留原文序号
            p = doc.add_paragraph()
            p.paragraph_format.left_indent = Cm(0.7)
            p.paragraph_format.first_line_indent = Cm(-0.45)
            add_inline(p, stripped)
            i += 1
            continue

        if stripped:
            p = doc.add_paragraph()
            add_inline(p, stripped)
        i += 1


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--title", required=True)
    parser.add_argument("--subtitle", required=True)
    parser.add_argument("--meta", nargs="*", default=[])
    args = parser.parse_args()

    text = Path(args.input).read_text(encoding="utf-8")
    doc = Document()
    style_document(doc)
    add_cover(doc, args.title, args.subtitle, args.meta)
    convert(text, doc)

    # 文档属性
    doc.core_properties.title = args.title
    doc.core_properties.author = "SynRun（曹卓雅、刘宇豪）"

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    doc.save(out)
    print(f"saved {out}")


if __name__ == "__main__":
    main()
