# 把本地仓库推送到 GitHub
# 用法：.\tools\push_github.ps1 -User 你的GitHub用户名
param(
    [Parameter(Mandatory = $true)][string]$User,
    [string]$Repo = "proj54-rcore-prep-sim"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not (Test-Path .git)) {
    throw "当前目录还没有 .git，请先运行 tools\init_repo.ps1"
}

$url = "https://github.com/$User/$Repo.git"
$existing = git remote get-url origin 2>$null
if ($LASTEXITCODE -eq 0 -and $existing) {
    git remote set-url origin $url
    Write-Host "已更新 origin -> $url"
} else {
    git remote add origin $url
    Write-Host "已添加 origin -> $url"
}

git push -u origin main
git push origin --tags
Write-Host "完成：https://github.com/$User/$Repo"
