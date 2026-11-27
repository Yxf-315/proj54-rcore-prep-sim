# 源代码地址与构建说明

## 仓库

- 平台：大赛希冀 GitLab（course.educg.net）
- 地址：`https://course.educg.net/<组>/proj54-SynRun`
- 分支：`main`
- 标签：`v1.0.0`
- 提交：`<填入 commit hash>`
- 镜像（可选）：`https://github.com/<org>/proj54-SynRun`

## 构建与运行

```bash
git clone https://course.educg.net/<组>/proj54-SynRun.git
cd proj54-SynRun

cargo test                      # 43 项测试
cargo run -- --check cases      # 8 个配套用例
cargo run                       # 交互模式
cargo run -- --exec examples/tour.txt
```

要求：Rust 1.75+，无第三方依赖，离线可构建。

## 目录速览

```
src/            memory / frame / paging / ownership / kernel / cases / repl / main
cases/          8 个 .case 测试样例（对应 rCore 实验观察点）
docs/           使用说明、设计文档、实验指导书、视频脚本、答辩手册、14 天计划
examples/       tour.txt（完整演示脚本）、demo-record.txt（录屏用）
tests/          端到端测试与用例回归
Dockerfile      一键复现
```

## 许可

源代码 BSD-3-Clause（见 `LICENSE`）；文档与视频 CC BY-SA 4.0。
