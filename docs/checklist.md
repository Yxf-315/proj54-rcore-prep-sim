# 提交清单（12 月 9 日）

> 按顺序执行，每完成一项打勾。目标：一次提交成功，不留返工。

---

## 1. 提交材料清单

| # | 文件 | 说明 | 状态 |
| --- | --- | --- | --- |
| 01 | `01-source-link.md` | GitLab 仓库地址 + commit hash + 构建命令 | ☐ |
| 02 | `02-design.pdf` / `.docx` | 设计实现文档（30%） | ☐ |
| 03 | `03-usage.pdf` / `.docx` | 使用说明 | ☐ |
| 04 | `04-lab-guide.pdf` / `.docx` | 实验指导书（教学 10%） | ☐ |
| 05 | `05-slides.pptx` / `.pdf` | 汇报幻灯片 | ☐ |
| 06 | `06-demo.mp4` | 演示视频 ≤5 分钟、≤150MB | ☐ |
| 07 | `07-trial-record.xlsx` | 试用记录（含真实数据） | ☐ |
| 08 | `08-defense-handbook.pdf` | 答辩手册（内部使用，可选提交） | ☐ |
| 09 | `09-license.txt` | BSD-3-Clause + CC BY-SA 4.0 说明 | ☐ |

## 2. 发布到希冀 GitLab

```bash
cd rcore-memlab
git init -b main
git add .
git commit -m "stage4: 提交 v1.0 材料（代码、文档、幻灯片、用例）"
git remote add origin https://course.educg.net/<组>/proj54-SynRun.git
git push -u origin main
git tag v1.0.0
git push --tags
```

- [ ] 仓库可见性符合赛方要求（一般 Public）；
- [ ] 成员已邀请（刘宇豪 Developer）；
- [ ] README 正常渲染，能一眼看到运行命令；
- [ ] commit 数 ≥ 8，信息按 `stage1`–`stage4` 组织；
- [ ] 仓库地址与 commit hash 填入 `01-source-link.md`。

## 3. 打包检查

- [ ] 所有 PDF 能正常打开，无空白页、无乱码；
- [ ] PPT 在 16:9 投影下无溢出（已在 12 页导出 PDF 验证）；
- [ ] 视频 MP4、H.264、时长 ≤5 分钟、≤150MB、有字幕；
- [ ] 文档中队伍名、学校、项目编号一致：proj54 / SynRun / 北京工商大学；
- [ ] 文档中所有占位符（`<希冀 GitLab 仓库地址>`、示例数据）已替换；
- [ ] 无真实学号、身份证号、手机号等敏感信息；
- [ ] 许可说明与仓库 LICENSE 一致（代码 BSD-3-Clause，文档 CC BY-SA 4.0）。

## 4. 平台提交

- [ ] 登录赛题页面，确认赛道为“华北区域赛 · 功能挑战”；
- [ ] 按赛方要求的文件命名/格式上传（若只收压缩包，打成 `proj54-SynRun.zip`）；
- [ ] 上传后**下载回来**抽查 1–2 个文件是否完整；
- [ ] 截图保存“提交成功”页面（含时间戳）；
- [ ] 12 月 11 日 19:00 前再登录确认一次状态。

## 5. 演示兜底

- [ ] 预编译 `memlab.exe`（Windows）与 Linux 二进制各一份；
- [ ] `docker build -t rcore-memlab .` 成功；
- [ ] 视频拷到 U 盘 + 手机 + 云盘；
- [ ] PPT 导出 PDF 备份（防字体缺失）。

## 6. 最后 10 分钟自检

```powershell
cd rcore-memlab
cargo test
cargo run -- --check cases
cargo run -- --exec examples/tour.txt
```

三条命令全绿，即可提交。
