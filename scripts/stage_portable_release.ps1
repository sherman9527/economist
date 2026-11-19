# Stage Self-Contained Portable x64 Folder (./portable-release/)
# Requirement #7 & #8: Portable x64 standalone directory, zero installer, zero C-drive writes, zero absolute paths.

param(
    [switch]$RebuildRelease
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $RepoRoot

$env:TEMP = Join-Path $RepoRoot ".temp"
$env:TMP = $env:TEMP

if ($RebuildRelease) {
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "Frontend Vite build failed." }
    . (Join-Path $PSScriptRoot "env_msvc.ps1")
    Invoke-WithMsvcX64 "cargo build --release --manifest-path src-tauri/Cargo.toml"
}

$ReleaseExe = Join-Path $RepoRoot ".build\cargo-target\x86_64-pc-windows-msvc\release\economist-quant-terminal.exe"
if (-not (Test-Path $ReleaseExe)) {
    throw "Release binary not found at relative path .build\cargo-target\x86_64-pc-windows-msvc\release\economist-quant-terminal.exe"
}

$StageDir = Join-Path $RepoRoot "portable-release"
foreach ($sub in @(
    "",
    ".github\agents",
    "scripts\renderer_pkg",
    "config",
    "plugins\strategies",
    "plugins\factors",
    "data\runs",
    "reports\assets",
    "logs\agents",
    "telemetry",
    "webview_data",
    ".temp"
)) {
    $target = if ($sub) { Join-Path $StageDir $sub } else { $StageDir }
    if (-not (Test-Path $target)) {
        New-Item -ItemType Directory -Force -Path $target | Out-Null
    }
}

Copy-Item -Force $ReleaseExe (Join-Path $StageDir "economist-quant-terminal.exe")
Copy-Item -Force (Join-Path $RepoRoot "config\app.toml") (Join-Path $StageDir "config\app.toml")
foreach ($pluginSub in @("strategies", "factors")) {
    Get-ChildItem -Path (Join-Path $RepoRoot "plugins\$pluginSub") -Filter "*.json" -File | ForEach-Object {
        $dstPlugin = Join-Path $StageDir "plugins\$pluginSub\$($_.Name)"
        if (-not (Test-Path $dstPlugin)) {
            Copy-Item -Force $_.FullName $dstPlugin
        }
    }
}
if (Test-Path (Join-Path $RepoRoot "memo.md")) {
    Copy-Item -Force (Join-Path $RepoRoot "memo.md") (Join-Path $StageDir "memo.md")
}
if (Test-Path (Join-Path $RepoRoot ".github\agents")) {
    Copy-Item -Force (Join-Path $RepoRoot ".github\agents\*.agent.md") (Join-Path $StageDir ".github\agents\")
}
Get-ChildItem -Path (Join-Path $RepoRoot "scripts") -Filter "*.py" -File |
    Where-Object { $_.Name -notlike "test_*.py" } |
    ForEach-Object { Copy-Item -Force $_.FullName (Join-Path $StageDir "scripts\$($_.Name)") }
Get-ChildItem -Path (Join-Path $RepoRoot "scripts") -Filter "*.ps1" -File |
    ForEach-Object { Copy-Item -Force $_.FullName (Join-Path $StageDir "scripts\$($_.Name)") }
if (Test-Path (Join-Path $RepoRoot "scripts\renderer_pkg")) {
    Copy-Item -Force (Join-Path $RepoRoot "scripts\renderer_pkg\*.py") (Join-Path $StageDir "scripts\renderer_pkg\")
}

$DstSqliteDb = Join-Path $StageDir "data\economist_quant.db"
if (Test-Path $DstSqliteDb) {
    python -E -c "import sqlite3, pathlib; p = pathlib.Path('portable-release/data/economist_quant.db'); conn = sqlite3.connect(str(p)); conn.execute('PRAGMA wal_checkpoint(TRUNCATE);'); conn.close()" 2>$null
} else {
    $SrcSqliteDb = Join-Path $RepoRoot "data\economist_quant.db"
    if (Test-Path $SrcSqliteDb) {
        python -E -c "import sqlite3, pathlib; p = pathlib.Path('data/economist_quant.db'); conn = sqlite3.connect(str(p)); conn.execute('PRAGMA wal_checkpoint(TRUNCATE);'); conn.close()" 2>$null
        Copy-Item -Force $SrcSqliteDb $DstSqliteDb
    }
}
foreach ($dataFile in @("ack_state.json", "backtest_ledger.json")) {
    $srcFile = Join-Path $RepoRoot "data\$dataFile"
    $dstFile = Join-Path $StageDir "data\$dataFile"
    if ((Test-Path $srcFile) -and (-not (Test-Path $dstFile))) {
        Copy-Item -Force $srcFile $dstFile
    }
}
if (Test-Path (Join-Path $RepoRoot "data\runs")) {
    Copy-Item -Force -Recurse (Join-Path $RepoRoot "data\runs\*") (Join-Path $StageDir "data\runs\")
}
if (Test-Path (Join-Path $RepoRoot "reports")) {
    Copy-Item -Force -Recurse (Join-Path $RepoRoot "reports\*") (Join-Path $StageDir "reports\")
}

& powershell -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "clean_build_cap.ps1")
$stagedExe = Get-Item (Join-Path $StageDir "economist-quant-terminal.exe")
Write-Host ("[StagePortable] Ready at .\portable-release\economist-quant-terminal.exe ({0:N2} MB)" -f ($stagedExe.Length / 1MB))
