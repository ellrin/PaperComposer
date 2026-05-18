$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$AppName = "Paper Composer"
$DistRoot = Join-Path $Root "dist\windows"
$PackageDir = Join-Path $DistRoot $AppName
$ZipPath = Join-Path $DistRoot "PaperComposer-windows.zip"

function Require-Command {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [Parameter(Mandatory = $true)]
        [string]$InstallHint
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name was not found. $InstallHint"
    }
}

function Invoke-CargoBuild {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Args
    )

    & cargo @Args
    if ($LASTEXITCODE -ne 0) {
        throw @"
cargo failed while running: cargo $($Args -join " ")

If the error says a dependency requires a newer rustc, update Rust with:
  rustup update stable

Then re-run:
  .\scripts\package_windows.ps1
"@
    }
}

Set-Location $Root
Require-Command -Name "cargo" -InstallHint "Install Rust from https://rustup.rs/, then open a new PowerShell window."

Write-Host "Building release binaries..."
Invoke-CargoBuild -Args @("build", "--release", "--bin", "paper-composer", "--bin", "drive_bridge")

Write-Host "Creating Windows package..."
if (Test-Path $PackageDir) {
    Remove-Item -Recurse -Force $PackageDir
}
New-Item -ItemType Directory -Force -Path $PackageDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $PackageDir "web") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $PackageDir "assets") | Out-Null

Copy-Item (Join-Path $Root "target\release\paper-composer.exe") (Join-Path $PackageDir "Paper Composer.exe") -Force
Copy-Item (Join-Path $Root "target\release\drive_bridge.exe") (Join-Path $PackageDir "drive_bridge.exe") -Force
Copy-Item (Join-Path $Root "web\*") (Join-Path $PackageDir "web") -Recurse -Force
Copy-Item (Join-Path $Root "assets\*") (Join-Path $PackageDir "assets") -Recurse -Force

$InstallPath = Join-Path $PackageDir "install.ps1"
@'
$ErrorActionPreference = "Stop"

$SourceDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$AppName = "Paper Composer"
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\Paper Composer"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Paper Composer"
$DesktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Paper Composer.lnk"
$StartMenuShortcut = Join-Path $StartMenuDir "Paper Composer.lnk"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $StartMenuDir | Out-Null

Copy-Item (Join-Path $SourceDir "*") $InstallDir -Recurse -Force

$Launcher = Join-Path $InstallDir "Paper Composer.exe"
$Icon = Join-Path $InstallDir "assets\papericon.ico"

$Shell = New-Object -ComObject WScript.Shell
foreach ($ShortcutPath in @($DesktopShortcut, $StartMenuShortcut)) {
    $Shortcut = $Shell.CreateShortcut($ShortcutPath)
    $Shortcut.TargetPath = $Launcher
    $Shortcut.WorkingDirectory = $InstallDir
    if (Test-Path $Icon) {
        $Shortcut.IconLocation = $Icon
    }
    $Shortcut.Save()
}

Write-Host "Installed $AppName to: $InstallDir"
Write-Host "Shortcuts created:"
Write-Host "  $DesktopShortcut"
Write-Host "  $StartMenuShortcut"
'@ | Set-Content -Encoding UTF8 $InstallPath

$UninstallPath = Join-Path $PackageDir "uninstall.ps1"
@'
$ErrorActionPreference = "Stop"

$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\Paper Composer"
$StartMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Paper Composer"
$DesktopShortcut = Join-Path ([Environment]::GetFolderPath("Desktop")) "Paper Composer.lnk"

Get-Process -Name "drive_bridge" -ErrorAction SilentlyContinue | Stop-Process -Force

if (Test-Path $DesktopShortcut) {
    Remove-Item $DesktopShortcut -Force
}
if (Test-Path $StartMenuDir) {
    Remove-Item $StartMenuDir -Recurse -Force
}
if (Test-Path $InstallDir) {
    Remove-Item $InstallDir -Recurse -Force
}

Write-Host "Paper Composer was removed."
'@ | Set-Content -Encoding UTF8 $UninstallPath

if (Test-Path $ZipPath) {
    Remove-Item -Force $ZipPath
}
Compress-Archive -Path (Join-Path $PackageDir "*") -DestinationPath $ZipPath -Force

Write-Host ""
Write-Host "Created package:"
Write-Host "  $PackageDir"
Write-Host "  $ZipPath"
Write-Host ""
Write-Host "To install on this Windows user account:"
Write-Host "  powershell -ExecutionPolicy Bypass -File `"$InstallPath`""
