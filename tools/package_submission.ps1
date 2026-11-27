# 打包提交材料为 zip（视频录制完成后运行）
# 用法：.\tools\package_submission.ps1
param(
    [string]$Out = "proj54-SynRun-submission.zip"
)

$root = Split-Path -Parent $PSScriptRoot
$submission = Join-Path $root "submission"

$video = Join-Path $submission "06-demo.mp4"
if (-not (Test-Path $video)) {
    Write-Warning "还没有 06-demo.mp4，打包后请补上视频再提交。"
}

$zip = Join-Path $root $Out
if (Test-Path $zip) { Remove-Item $zip -Force }

Compress-Archive -Path (Join-Path $submission "*") -DestinationPath $zip -Force
$size = [math]::Round((Get-Item $zip).Length / 1MB, 2)
Write-Host "已生成 $zip（$size MB）"
