#!/usr/bin/env python3
"""生成 12 页答辩/汇报幻灯片（16:9）。

用法：python build_deck.py --output submission/05-slides.pptx
"""

from __future__ import annotations

import argparse
from pathlib import Path

from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.util import Inches, Pt

NAVY = RGBColor(0x1F, 0x3A, 0x5F)
TEAL = RGBColor(0x2A, 0x9D, 0x8F)
GRAY = RGBColor(0x5B, 0x6B, 0x7A)
LIGHT = RGBColor(0xF6, 0xF8, 0xFA)
WHITE = RGBColor(0xFF, 0xFF, 0xFF)
DARK = RGBColor(0x22, 0x30, 0x3F)

FONT = "Microsoft YaHei"
W, H = 13.333, 7.5


def new_deck() -> Presentation:
    prs = Presentation()
    prs.slide_width = Inches(W)
    prs.slide_height = Inches(H)
    return prs


def blank(prs: Presentation):
    return prs.slides.add_slide(prs.slide_layouts[6])


def rect(slide, x, y, w, h, color, line=None):
    shape = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, Inches(x), Inches(y), Inches(w), Inches(h))
    shape.fill.solid()
    shape.fill.fore_color.rgb = color
    if line is None:
        shape.line.fill.background()
    else:
        shape.line.color.rgb = line
    shape.shadow.inherit = False
    return shape


def textbox(slide, x, y, w, h, text, size=18, color=DARK, bold=False,
            align=PP_ALIGN.LEFT, anchor=MSO_ANCHOR.TOP, spacing=1.15):
    box = slide.shapes.add_textbox(Inches(x), Inches(y), Inches(w), Inches(h))
    tf = box.text_frame
    tf.word_wrap = True
    tf.vertical_anchor = anchor
    tf.margin_left = 0
    tf.margin_right = 0
    tf.margin_top = 0
    tf.margin_bottom = 0
    lines = text.split("\n")
    for i, line in enumerate(lines):
        p = tf.paragraphs[0] if i == 0 else tf.add_paragraph()
        p.alignment = align
        p.line_spacing = spacing
        run = p.add_run()
        run.text = line
        run.font.name = FONT
        run.font.size = Pt(size)
        run.font.bold = bold
        run.font.color.rgb = color
    return box


def set_east_asia(run):
    from pptx.oxml.ns import qn
    rpr = run._r.get_or_add_rPr()
    ea = rpr.find(qn("a:ea"))
    if ea is None:
        ea = rpr.makeelement(qn("a:ea"), {})
        rpr.append(ea)
    ea.set("typeface", FONT)


def header(slide, title, page):
    rect(slide, 0, 0, W, 0.95, NAVY)
    rect(slide, 0, 0.95, W, 0.06, TEAL)
    textbox(slide, 0.55, 0.18, 10.8, 0.6, title, size=26, color=WHITE, bold=True)
    textbox(slide, 11.5, 0.28, 1.3, 0.4, f"{page:02d} / 12", size=12, color=WHITE, align=PP_ALIGN.RIGHT)


def footer(slide, text="proj54 · SynRun · 北京工商大学"):
    textbox(slide, 0.55, 7.05, 8, 0.3, text, size=10, color=GRAY)


def bullets(slide, items, x=0.7, y=1.35, w=11.9, h=5.4, size=19, gap=10):
    box = slide.shapes.add_textbox(Inches(x), Inches(y), Inches(w), Inches(h))
    tf = box.text_frame
    tf.word_wrap = True
    for i, item in enumerate(items):
        p = tf.paragraphs[0] if i == 0 else tf.add_paragraph()
        p.space_after = Pt(gap)
        p.line_spacing = 1.2
        run = p.add_run()
        run.text = "▪  " + item
        run.font.name = FONT
        run.font.size = Pt(size)
        run.font.color.rgb = DARK


def table(slide, headers, rows, x, y, w, h, col_widths=None, size=12):
    shape = slide.shapes.add_table(len(rows) + 1, len(headers), Inches(x), Inches(y), Inches(w), Inches(h))
    tbl = shape.table
    if col_widths:
        total = sum(col_widths)
        for i, cw in enumerate(col_widths):
            tbl.columns[i].width = Inches(w * cw / total)
    for j, cell_text in enumerate(headers):
        cell = tbl.cell(0, j)
        cell.text = ""
        cell.fill.solid()
        cell.fill.fore_color.rgb = NAVY
        p = cell.text_frame.paragraphs[0]
        run = p.add_run()
        run.text = cell_text
        run.font.name = FONT
        run.font.size = Pt(size)
        run.font.bold = True
        run.font.color.rgb = WHITE
    for i, row in enumerate(rows, start=1):
        for j in range(len(headers)):
            cell = tbl.cell(i, j)
            cell.text = ""
            cell.fill.solid()
            cell.fill.fore_color.rgb = WHITE if i % 2 else LIGHT
            p = cell.text_frame.paragraphs[0]
            run = p.add_run()
            run.text = row[j] if j < len(row) else ""
            run.font.name = FONT
            run.font.size = Pt(size)
            run.font.color.rgb = DARK
    return shape


def code_box(slide, text, x, y, w, h, size=11):
    shape = rect(slide, x, y, w, h, RGBColor(0xEE, 0xF1, 0xF4))
    tf = shape.text_frame
    tf.word_wrap = True
    tf.vertical_anchor = MSO_ANCHOR.TOP
    tf.margin_left = Inches(0.12)
    tf.margin_top = Inches(0.08)
    for i, line in enumerate(text.split("\n")):
        p = tf.paragraphs[0] if i == 0 else tf.add_paragraph()
        p.alignment = PP_ALIGN.LEFT
        p.line_spacing = 1.0
        run = p.add_run()
        run.text = line
        run.font.name = "Consolas"
        run.font.size = Pt(size)
        run.font.color.rgb = DARK


def cover(prs: Presentation):
    slide = blank(prs)
    rect(slide, 0, 0, W, H, NAVY)
    rect(slide, 0.9, 1.35, 0.12, 1.5, TEAL)
    textbox(slide, 1.2, 1.25, 11, 0.9, "rCore 配套预习模拟器", size=44, color=WHITE, bold=True)
    textbox(slide, 1.22, 2.2, 11, 0.6, "用 Rust 把 Sv39 地址翻译变成可交互、可判定的实验", size=20,
            color=RGBColor(0xC9, 0xD6, 0xE2))
    textbox(slide, 1.22, 4.6, 11, 1.4,
            "proj54 · 华北区域赛 功能挑战赛道\nSynRun（北京工商大学）｜曹卓雅 · 刘宇豪",
            size=16, color=WHITE, spacing=1.5)
    textbox(slide, 1.22, 6.5, 11, 0.4, "2026 年 12 月", size=12, color=RGBColor(0xC9, 0xD6, 0xE2))


def slide_problem(prs, page):
    s = blank(prs)
    header(s, "问题：初学者在 Lab2/Lab3 前的三个断层", page)
    bullets(s, [
        "页表是黑箱：PTE 里有什么、VPN 怎么切，课本图示看完仍然不会算",
        "缺页难定位：写错一个位域就是三重故障，不知道停在第几级",
        "所有权与内核脱节：知道 move 会失效，不知道它正是内核防双重释放的机制",
    ], y=1.5, size=21)
    rect(s, 0.7, 5.35, 11.9, 1.15, LIGHT)
    textbox(s, 1.0, 5.55, 11.3, 0.8,
            "结论：缺的不是又一个内核，而是一个「先跑通原理、再进 QEMU」的中间层。",
            size=20, color=NAVY, bold=True)
    footer(s)


def slide_position(prs, page):
    s = blank(prs)
    header(s, "定位：单一知识点的预习工具", page)
    cards = [
        ("先模拟，后内核", "在用户态复刻帧分配与三级页表，建立正确心智模型"),
        ("单一知识点", "聚焦 Sv39 地址翻译；帧分配器作为页表的物理内存支撑"),
        ("可交互、可判定", "输入 VA/PA/权限看全过程；8 个用例自动判分"),
    ]
    for i, (title, desc) in enumerate(cards):
        x = 0.7 + i * 4.05
        rect(s, x, 1.6, 3.8, 3.0, LIGHT)
        rect(s, x, 1.6, 3.8, 0.09, TEAL)
        textbox(s, x + 0.3, 1.95, 3.2, 0.6, title, size=22, color=NAVY, bold=True)
        textbox(s, x + 0.3, 2.65, 3.2, 1.6, desc, size=15, color=DARK, spacing=1.35)
    textbox(s, 0.7, 5.2, 11.9, 1.0,
            "一句话：让学生先在模拟器上把「地址怎么翻译、缺页停在哪一级」跑通，再进 rCore 做实验。",
            size=18, color=GRAY)
    footer(s)


def slide_arch(prs, page):
    s = blank(prs)
    header(s, "总体架构：单向依赖的五层", page)
    layers = [
        ("main / repl", "入口、参数解析、交互循环"),
        ("kernel", "命令状态机、lab2/lab3 脚本、输出格式化"),
        ("frame / paging / ownership", "帧分配器与 RAII · Sv39 页表 · 所有权演示"),
        ("memory", "PhysAddr / VirtAddr / 64 MiB 模拟物理内存"),
    ]
    y = 1.45
    for i, (name, desc) in enumerate(layers):
        color = NAVY if i == 1 else RGBColor(0x33, 0x55, 0x77)
        rect(s, 1.0, y, 11.2, 0.95, color)
        textbox(s, 1.3, y + 0.13, 4.2, 0.5, name, size=19, color=WHITE, bold=True)
        textbox(s, 5.4, y + 0.18, 6.4, 0.5, desc, size=14, color=RGBColor(0xD8, 0xE2, 0xEC))
        if i < len(layers) - 1:
            textbox(s, 6.4, y + 0.92, 0.6, 0.35, "▼", size=13, color=TEAL, align=PP_ALIGN.CENTER)
        y += 1.28
    textbox(s, 1.0, 6.6, 11.2, 0.4, "命令层是纯状态机：交互、脚本、测试、CI 共用同一条执行路径。",
            size=13, color=GRAY)
    footer(s)


def slide_frame(prs, page):
    s = blank(prs)
    header(s, "Lab2 支撑：栈式帧分配器与 RAII", page)
    bullets(s, [
        "只有两个状态：水位线 frontier + 回收栈 recycled（与 rCore 同构）",
        "FrameTracker 独占所有权，Drop 自动归还；没有 Clone，因此写不出双重释放",
    ], y=1.35, h=1.7, size=18)
    table(s, ["操作", "持有帧", "水位线", "回收栈"], [
        ["alloc 3", "4", "4", "0"],
        ["free 中间帧", "3", "4", "1"],
        ["alloc 2", "5", "5", "0（先复用刚释放的帧）"],
    ], x=0.7, y=3.35, w=11.9, h=1.9, col_widths=[2.2, 1.4, 1.4, 4.2], size=13)
    textbox(s, 0.7, 5.65, 11.9, 0.8,
            "对应源码：os/src/mm/frame_allocator.rs；对应练习：Rustlings move_semantics / smart_pointers。",
            size=14, color=GRAY)
    footer(s)


def slide_walk(prs, page):
    s = blank(prs)
    header(s, "核心知识点：Sv39 三级页表遍历", page)
    table(s, ["级别", "VPN 索引", "PTE 地址", "PTE 值", "权限", "下一级"], [
        ["L2", "0", "0x8000_0000", "0x…2000_1801", "- - -", "0x8000_6000"],
        ["L1", "0", "0x8000_6000", "0x…2000_1c01", "- - -", "0x8000_7000"],
        ["L0", "1", "0x8000_7008", "0x…2000_1407", "R W -", "叶子（4 KiB）"],
    ], x=0.7, y=1.4, w=11.9, h=2.0, col_widths=[1.1, 1.4, 2.2, 2.6, 1.4, 2.2], size=12)
    textbox(s, 0.7, 3.7, 11.9, 1.0,
            "VA 0x1000 → VPN[2]=0、VPN[1]=0、VPN[0]=1；三级都有效，最终 PA = 0x8000_5000。",
            size=17, color=DARK)
    code_box(s, "memlab> map 0x1000 auto rw\nmemlab> walk 0x1000\n✓ 翻译结果：VA 0x0000_1000 → PA 0x8000_5000",
             0.7, 4.75, 7.6, 1.55, size=12)
    textbox(s, 8.6, 4.85, 4.0, 1.4,
            "对应源码：\nos/src/mm/page_table.rs\n（find_pte / map / unmap）",
            size=13, color=GRAY)
    footer(s)


def slide_fault(prs, page):
    s = blank(prs)
    header(s, "缺页与权限：把异常定位到「第几级」", page)
    rect(s, 0.7, 1.4, 5.8, 4.2, LIGHT)
    textbox(s, 1.0, 1.6, 5.2, 0.5, "未映射 → 缺页", size=20, color=NAVY, bold=True)
    code_box(s, "fault 0x9000\n× 缺页异常：VA=0x0009_0000\n  在第 2 级页表（L2）处 V=0",
             1.0, 2.25, 5.2, 1.2, size=12)
    textbox(s, 1.0, 3.65, 5.2, 1.7,
            "根页表没有任何有效项，所以第一级就断了。\n内核要据此决定：补映射还是杀进程。",
            size=14, color=DARK, spacing=1.3)
    rect(s, 6.8, 1.4, 5.8, 4.2, LIGHT)
    textbox(s, 7.1, 1.6, 5.2, 0.5, "只读页写访问 → 权限异常", size=20, color=NAVY, bold=True)
    code_box(s, "map 0x2000 auto r\nfault 0x2000 w\n× 权限异常：L0 的 W 位不允许",
             7.1, 2.25, 5.2, 1.2, size=12)
    textbox(s, 7.1, 3.65, 5.2, 1.7,
            "页表项有效，但 R/W/X 不满足。\n对应 rCore：__alltraps → trap_handler 的 scause 分派。",
            size=14, color=DARK, spacing=1.3)
    footer(s)


def slide_ownership(prs, page):
    s = blank(prs)
    header(s, "Rust 所有权 ↔ 内核资源生命周期", page)
    table(s, ["Rust 机制", "内核中的对应", "效果"], [
        ["move", "FrameTracker 不可 Clone", "一块物理帧只有一个主人，杜绝双重释放"],
        ["Drop", "FrameTracker::drop", "离开作用域自动 free，杜绝泄漏"],
        ["Rc / Arc", "ARC<FrameTracker>（COW）", "共享页引用计数归零才回收"],
        ["&T / &mut T", "PageTable::map(&mut self)", "遍历页表时禁止改页表，消灭数据竞争"],
        ["lifetime", "Arc + UPSafeCell", "引用不超出锁的作用域，悬垂指针不存在"],
    ], x=0.7, y=1.45, w=11.9, h=3.8, col_widths=[2.0, 3.6, 6.0], size=13)
    textbox(s, 0.7, 5.9, 11.9, 0.7,
            "demo move / raii / clone / double-free 四个演示，全部用真实物理帧运行。",
            size=15, color=GRAY)
    footer(s)


def slide_teaching(prs, page):
    s = blank(prs)
    header(s, "教学闭环：指导书 + 测例 + 试用", page)
    blocks = [
        ("实验指导书", "4 个实验：帧分配 / 页表遍历 / 缺页 / 扩展挑战\n含思考题、评分 rubric、FAQ"),
        ("配套测例", "8 个 .case 用例 + 43 项 cargo test\nmemlab --check cases 一键判定"),
        ("教学试用", "5 名同学按指导书完成实验\n记录完成率、耗时、卡点与改进"),
    ]
    for i, (title, desc) in enumerate(blocks):
        x = 0.7 + i * 4.05
        rect(s, x, 1.5, 3.8, 3.4, LIGHT)
        rect(s, x, 1.5, 3.8, 0.09, TEAL)
        textbox(s, x + 0.25, 1.8, 3.3, 0.5, title, size=20, color=NAVY, bold=True)
        textbox(s, x + 0.25, 2.45, 3.3, 2.2, desc, size=13, color=DARK, spacing=1.4)
    textbox(s, 0.7, 5.35, 11.9, 1.1,
            "试用数据（替换为真实值）：5 人完成率 80%，平均 62 分钟，主要卡点：VPN 计算、PPN 与 PA 换算。",
            size=15, color=GRAY, spacing=1.4)
    footer(s)


def slide_quality(prs, page):
    s = blank(prs)
    header(s, "质量与可复现性", page)
    nums = [("43", "自动测试"), ("8", "配套用例"), ("0", "第三方依赖"), ("2", "平台 CI")]
    for i, (num, label) in enumerate(nums):
        x = 0.7 + i * 3.05
        rect(s, x, 1.5, 2.8, 2.2, LIGHT)
        textbox(s, x, 1.7, 2.8, 0.9, num, size=44, color=TEAL, bold=True, align=PP_ALIGN.CENTER)
        textbox(s, x, 2.7, 2.8, 0.5, label, size=15, color=DARK, align=PP_ALIGN.CENTER)
    bullets(s, [
        "cargo test / clippy -D warnings / fmt --check 全绿",
        "GitLab + GitHub 双平台流水线；Dockerfile 一键复现",
        "零依赖离线可构建，评审现场断网也能演示",
    ], y=4.1, size=16, h=2.2)
    footer(s)


def slide_compare(prs, page):
    s = blank(prs)
    header(s, "与类似做法的差异", page)
    table(s, ["对比对象", "局限", "本工具"], [
        ["rCore 原生实验", "门槛高、反馈慢、看不到中间态", "先建立心智模型，再进内核"],
        ["Rustlings", "与 OS 无关", "把所有权映射到帧与页表"],
        ["教材图示", "静态、不可验证", "输入参数、真实计算"],
        ["网页可视化", "与 rCore 代码脱节、依赖网络", "源码逐文件对照、离线可跑"],
    ], x=0.7, y=1.5, w=11.9, h=3.4, col_widths=[2.8, 4.6, 4.5], size=13)
    textbox(s, 0.7, 5.3, 11.9, 0.9,
            "创新点：页表真实驻留内存 + 所有权与内核资源显式映射 + 可判定的教学闭环。",
            size=16, color=NAVY, bold=True)
    footer(s)


def slide_roadmap(prs, page):
    s = blank(prs)
    header(s, "路线图与致谢", page)
    bullets(s, [
        "已完成：Lab2/Lab3 模拟、7 个所有权演示、43 项测试、8 个用例、文档与 CI",
        "下一阶段：Lab4 任务切换可视化、COW 引用计数、unmap 回收空页表",
        "远期：与真实 rCore 仓库联动的对照模式、WebAssembly 版本",
    ], y=1.5, size=18, h=2.8)
    rect(s, 0.7, 4.6, 11.9, 1.3, LIGHT)
    textbox(s, 1.0, 4.8, 11.3, 0.9,
            "谢谢各位老师！欢迎提问。\nGitLab：course.educg.net ｜ 项目：proj54-SynRun",
            size=17, color=NAVY, bold=True, spacing=1.4)
    footer(s)


def build(path: Path) -> None:
    prs = new_deck()
    cover(prs)
    slide_problem(prs, 2)
    slide_position(prs, 3)
    slide_arch(prs, 4)
    slide_frame(prs, 5)
    slide_walk(prs, 6)
    slide_fault(prs, 7)
    slide_ownership(prs, 8)
    slide_teaching(prs, 9)
    slide_quality(prs, 10)
    slide_compare(prs, 11)
    slide_roadmap(prs, 12)

    prs.core_properties.title = "rCore 配套预习模拟器 · 汇报"
    prs.core_properties.author = "SynRun（曹卓雅、刘宇豪）"
    path.parent.mkdir(parents=True, exist_ok=True)
    prs.save(path)
    print(f"saved {path} ({len(prs.slides)} slides)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="submission/05-slides.pptx")
    args = parser.parse_args()
    build(Path(args.output))
