# 源代码地址与构建说明

## 仓库

- 主仓库（GitHub）：`https://github.com/<你的GitHub用户名>/proj54-rcore-prep-sim`
- 分支：`main`
- 标签：`v1.0.0`
- 提交：`<填入 commit hash>`
- 希冀镜像：后续在希冀 GitLab 通过“导入 GitHub 仓库”生成，地址待补

> 老师要求先在 GitHub 上维护，评审前再导入/镜像到希冀 GitLab；
> 两边保持同一份 `main` 分支即可。

## 构建与运行

```bash
git clone https://github.com/<你的GitHub用户名>/proj54-rcore-prep-sim.git
cd proj54-rcore-prep-sim

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
