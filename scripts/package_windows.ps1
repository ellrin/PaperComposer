$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

cargo build --release

$Dist = Join-Path $Root "dist\windows"
New-Item -ItemType Directory -Force -Path $Dist | Out-Null
Copy-Item "target\release\paper-composer.exe" (Join-Path $Dist "Paper Composer.exe") -Force
Copy-Item "assets\papericon.png" (Join-Path $Dist "papericon.png") -Force

Write-Host "Created: $Dist\Paper Composer.exe"
