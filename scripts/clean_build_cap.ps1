<#
.SYNOPSIS
    Enforces Requirement #7: Periodic build artifact cleanup keeping build footprint strictly < 5 GB.
    Soft watermark: 4.0 GB | Hard ceiling: 5.0 GB. Zero hardcoded absolute paths.
#>
param(
    [long]$SoftWatermarkBytes = 4294967296, # 4.0 GB
    [long]$HardCapBytes       = 5368709120  # 5.0 GB
)

$ErrorActionPreference = "SilentlyContinue"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

$TargetDirs = @(
    (Join-Path $RepoRoot ".build\cargo-target"),
    (Join-Path $RepoRoot "src-tauri\target"),
    (Join-Path $RepoRoot "dist"),
    (Join-Path $RepoRoot "node_modules\.vite"),
    (Join-Path $RepoRoot ".temp")
)

function Get-BuildFootprintBytes {
    $sum = [long]0
    foreach ($d in $TargetDirs) {
        if (Test-Path $d) {
            $measured = (Get-ChildItem -Path $d -Recurse -File -Force | Measure-Object -Property Length -Sum).Sum
            if ($measured) { $sum += [long]$measured }
        }
    }
    return $sum
}

$before = Get-BuildFootprintBytes
$beforeGb = [math]::Round($before / 1GB, 3)

if ($before -gt $SoftWatermarkBytes) {
    Write-Host "[CleanBuildCap] Footprint ${beforeGb} GB exceeds 4.0 GB soft watermark. Running Tier-1 cleanup..."
    # Tier 1: purge .temp (>1h), vite cache, incremental cache, and stale duplicate crate hashes
    $tempPath = Join-Path $RepoRoot ".temp"
    if (Test-Path $tempPath) {
        Get-ChildItem -Path $tempPath -Force | Where-Object { $_.LastWriteTimeUtc -lt (Get-Date).ToUniversalTime().AddHours(-1) } | Remove-Item -Recurse -Force
    }
    $viteCache = Join-Path $RepoRoot "node_modules\.vite"
    if (Test-Path $viteCache) { Remove-Item -Path $viteCache -Recurse -Force }

    foreach ($cargoTgt in @((Join-Path $RepoRoot ".build\cargo-target"), (Join-Path $RepoRoot "src-tauri\target"))) {
        if (Test-Path $cargoTgt) {
            Get-ChildItem -Path $cargoTgt -Filter "incremental" -Directory -Recurse | Remove-Item -Recurse -Force
            Get-ChildItem -Path $cargoTgt -Include "*.pdb" -File -Recurse | Remove-Item -Force
        }
    }
}

$mid = Get-BuildFootprintBytes
if ($mid -ge ($HardCapBytes - 314572800)) {
    Write-Host "[CleanBuildCap] Footprint still near 5.0 GB cap. Running Tier-2 debug artifact purge..."
    foreach ($dbgDir in @((Join-Path $RepoRoot ".build\cargo-target\debug"), (Join-Path $RepoRoot "src-tauri\target\debug"))) {
        if (Test-Path $dbgDir) { Remove-Item -Path $dbgDir -Recurse -Force }
    }
}

$after = Get-BuildFootprintBytes
$afterGb = [math]::Round($after / 1GB, 3)
Write-Host "[CleanBuildCap] Build artifacts: ${afterGb} GB / 5.000 GB cap (before: ${beforeGb} GB)."
if ($after -ge $HardCapBytes) {
    Write-Error "[CleanBuildCap] VIOLATION: Build footprint ${afterGb} GB >= 5.0 GB!"
    exit 1
}
exit 0
