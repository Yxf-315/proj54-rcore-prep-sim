# 把当前目录初始化成 git 仓库，并按四阶段做 8 次提交。
# 用法：
#   .\tools\init_repo.ps1
#   .\tools\init_repo.ps1 -Remote "https://course.educg.net/<组>/proj54-SynRun.git"
param(
    [string]$Remote = "",
    [string]$UserName = "SynRun",
    [string]$UserEmail = "synrun@example.com"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "未找到 git，请先安装 Git for Windows。"
}
if (Test-Path .git) {
    throw "当前目录已经有 .git，脚本停止。若要重来，请先删除 .git 目录。"
}

git init -b main | Out-Null
git config user.name $UserName
git config user.email $UserEmail

function Commit([string]$Message, [string[]]$Paths) {
    git add -- $Paths
    git commit -m $Message | Out-Null
    Write-Host "OK  $Message"
}

Commit "stage1: 初始化工程、协议与 .gitignore" @(
    "Cargo.toml", "Cargo.lock", ".gitignore", ".gitattributes", "LICENSE", "README.md"
)
Commit "stage2: 实现栈式帧分配器、Sv39 页表与命令层" @("src")
Commit "stage3: 配套测试样例与端到端回归" @("cases", "tests")
Commit "stage4: 设计实现文档与 rCore 源码对照" @(
    "docs/design.md", "docs/roadmap.md", "docs/submission.md"
)
Commit "stage4: 使用说明、实验指导书与冲刺计划" @(
    "docs/usage.md", "docs/lab-guide.md", "docs/plan-20d.md", "docs/plan-14d.md",
    "docs/checklist.md", "docs/video-script.md", "docs/defense-handbook.md"
)
Commit "stage4: 演示脚本、Docker 与双平台 CI" @(
    "examples", "Dockerfile", ".github", ".gitlab-ci.yml"
)
Commit "stage4: 文档、幻灯片与试用表生成脚本" @("tools")
Commit "stage4: 提交材料包与协议说明" @("submission")

if ($Remote -ne "") {
    git remote add origin $Remote
    git push -u origin main
    git tag v1.0.0
    git push origin v1.0.0
    Write-Host "已推送到 $Remote"
}

Write-Host ""
git log --oneline
