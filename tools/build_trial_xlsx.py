#!/usr/bin/env python3
"""生成教学试用记录表（xlsx），供 5 人试用时填写并汇总。

用法：python build_trial_xlsx.py --output submission/07-trial-record.xlsx
"""

from __future__ import annotations

import argparse
from pathlib import Path

from openpyxl import Workbook
from openpyxl.styles import Alignment, Border, Font, PatternFill, Side
from openpyxl.utils import get_column_letter
from openpyxl.worksheet.datavalidation import DataValidation

NAVY = "1F3A5F"
TEAL = "2A9D8F"
LIGHT = "F6F8FA"
WHITE = "FFFFFF"

TITLE_FONT = Font(name="微软雅黑", size=16, bold=True, color=NAVY)
BODY_FONT = Font(name="微软雅黑", size=10.5)
HEADER_FONT = Font(name="微软雅黑", size=10.5, bold=True, color=WHITE)
HEADER_FILL = PatternFill("solid", fgColor=NAVY)
THIN = Side(style="thin", color="C9D2DC")
BORDER = Border(left=THIN, right=THIN, top=THIN, bottom=THIN)


def style_header(ws, row: int, cols: int) -> None:
    for c in range(1, cols + 1):
        cell = ws.cell(row=row, column=c)
        cell.font = HEADER_FONT
        cell.fill = HEADER_FILL
        cell.alignment = Alignment(horizontal="center", vertical="center", wrap_text=True)
        cell.border = BORDER


def set_widths(ws, widths: dict[str, float]) -> None:
    for col, width in widths.items():
        ws.column_dimensions[col].width = width


def build(path: Path) -> None:
    wb = Workbook()

    # ---------------- 使用说明 ----------------
    ws = wb.active
    ws.title = "使用说明"
    ws.sheet_view.showGridLines = False
    ws["A1"] = "rCore 配套预习模拟器 · 教学试用记录表"
    ws["A1"].font = TITLE_FONT
    ws.merge_cells("A1:F1")
    lines = [
        "",
        "用途：记录 5 名同学按《实验指导书》试用本工具的完成情况、耗时与卡点，作为设计文档“教学验证”章节的数据来源。",
        "",
        "填写步骤：",
        "1. 每位同学一行，填入“试用记录”表；编号用 S1、S2……；姓名可只写姓氏或化名，注意隐私。",
        "2. “完成任务”填写实际完成的实验（实验一/二/三/四）；“是否完成”填“是”或“否”。",
        "3. “主要卡点”写现象（如“VPN 计算”）；“原话摘录”尽量记录同学的原话，答辩时最有说服力。",
        "4. “汇总”表自动计算完成率与平均耗时；公式结果需要刷新时，用 office 工具重新计算或打开后按 F9。",
        "5. “问题清单”按优先级排序，试用后逐条关闭；未关闭的写进“局限与后续”。",
        "",
        "注意：提交前务必替换掉本页的示例说明，并确认没有填写真实姓名、学号等敏感信息。",
    ]
    for i, text in enumerate(lines, start=2):
        cell = ws.cell(row=i, column=1, value=text)
        cell.font = BODY_FONT
        cell.alignment = Alignment(wrap_text=True, vertical="top")
        ws.merge_cells(start_row=i, start_column=1, end_row=i, end_column=6)
    set_widths(ws, {"A": 18, "B": 18, "C": 18, "D": 18, "E": 18, "F": 18})

    # ---------------- 试用记录 ----------------
    ws = wb.create_sheet("试用记录")
    headers = ["编号", "日期", "年级", "编程基础", "完成任务", "总耗时(分钟)",
               "求助次数", "主要卡点", "原话摘录", "改进措施", "是否完成"]
    ws.append(headers)
    style_header(ws, 1, len(headers))
    for i in range(2, 12):
        ws.append([f"S{i - 1}", "", "", "", "实验一", "", "", "", "", "", "否"])
        for c in range(1, len(headers) + 1):
            ws.cell(row=i, column=c).font = BODY_FONT
            ws.cell(row=i, column=c).border = BORDER
            ws.cell(row=i, column=c).alignment = Alignment(vertical="center", wrap_text=True)
        ws.row_dimensions[i].height = 26
    set_widths(ws, {"A": 7, "B": 11, "C": 9, "D": 13, "E": 14, "F": 12,
                    "G": 10, "H": 22, "I": 30, "J": 24, "K": 10})
    ws.freeze_panes = "A2"

    dv_task = DataValidation(type="list", formula1='"实验一,实验二,实验三,实验四,全部"', allow_blank=True)
    dv_done = DataValidation(type="list", formula1='"是,否"', allow_blank=True)
    ws.add_data_validation(dv_task)
    ws.add_data_validation(dv_done)
    dv_task.add("E2:E11")
    dv_done.add("K2:K11")

    # ---------------- 汇总 ----------------
    ws = wb.create_sheet("汇总")
    ws.sheet_view.showGridLines = False
    ws["A1"] = "试用数据汇总（自动计算）"
    ws["A1"].font = TITLE_FONT
    ws.merge_cells("A1:C1")

    rows = [
        ("试用人数", "=COUNTA('试用记录'!B2:B11)", "人"),
        ("完成人数", "=COUNTIF('试用记录'!K2:K11,\"是\")", "人"),
        ("完成率", "=IFERROR(B3/B2,0)", "百分比"),
        ("平均耗时", "=IFERROR(AVERAGE('试用记录'!F2:F11),0)", "分钟"),
        ("求助总次数", "=SUM('试用记录'!G2:G11)", "次"),
        ("平均求助次数", "=IFERROR(B6/B2,0)", "次/人"),
    ]
    for i, (label, formula, unit) in enumerate(rows, start=2):
        ws.cell(row=i, column=1, value=label).font = BODY_FONT
        cell = ws.cell(row=i, column=2, value=formula)
        cell.font = Font(name="微软雅黑", size=12, bold=True, color=TEAL)
        ws.cell(row=i, column=3, value=unit).font = BODY_FONT
        for c in range(1, 4):
            ws.cell(row=i, column=c).border = BORDER
    ws["B4"].number_format = "0.0%"
    ws["B5"].number_format = "0.0"
    ws["B7"].number_format = "0.0"

    ws["A10"] = "主要结论（试用后手写）"
    ws["A10"].font = Font(name="微软雅黑", size=11, bold=True, color=NAVY)
    for i, prompt in enumerate([
        "1. 完成率与平均耗时：",
        "2. 卡点 Top3：",
        "3. 已做的改进：",
        "4. 是否达到“降低 Lab3 入门门槛”的预期：",
    ], start=11):
        ws.cell(row=i, column=1, value=prompt).font = BODY_FONT
        ws.merge_cells(start_row=i, start_column=1, end_row=i, end_column=6)
        ws.row_dimensions[i].height = 24
    set_widths(ws, {"A": 24, "B": 14, "C": 10, "D": 10, "E": 10, "F": 10})

    # ---------------- 问题清单 ----------------
    ws = wb.create_sheet("问题清单")
    headers = ["编号", "问题描述", "来源(编号)", "优先级", "状态", "负责人", "计划解决日期"]
    ws.append(headers)
    style_header(ws, 1, len(headers))
    samples = [
        ["P1", "VPN 索引怎么从地址里切出来不清楚", "S1", "高", "待处理", "", ""],
        ["P2", "PPN 与物理地址换算混淆", "S2", "高", "待处理", "", ""],
        ["P3", "缺页层级判断缺少提示", "S3", "中", "待处理", "", ""],
    ]
    for row in samples:
        ws.append(row)
    for i in range(2, 9):
        for c in range(1, len(headers) + 1):
            ws.cell(row=i, column=c).font = BODY_FONT
            ws.cell(row=i, column=c).border = BORDER
            ws.cell(row=i, column=c).alignment = Alignment(vertical="center", wrap_text=True)
    ws.freeze_panes = "A2"
    set_widths(ws, {"A": 7, "B": 38, "C": 10, "D": 9, "E": 10, "F": 10, "G": 14})

    wb.properties.title = "rCore 配套预习模拟器 · 教学试用记录"
    wb.properties.creator = "SynRun（曹卓雅、刘宇豪）"
    path.parent.mkdir(parents=True, exist_ok=True)
    wb.save(path)
    print(f"saved {path}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="submission/07-trial-record.xlsx")
    args = parser.parse_args()
    build(Path(args.output))
