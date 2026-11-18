# Dynamic MSVC x64 + Portable Local Rust Toolchain Environment Loader
# Strictly ZERO hardcoded absolute paths (Requirement #2 & ./rule.md Pitfall #03).

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

$TempDir = Join-Path $RepoRoot ".temp"
$CargoHome = Join-Path $RepoRoot ".toolchain\cargo"
$RustupHome = Join-Path $RepoRoot ".toolchain\rustup"
$CargoTarget = Join-Path $RepoRoot ".build\cargo-target"

foreach ($dir in @($TempDir, $CargoHome, $RustupHome, $CargoTarget)) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
    }
}

$env:TEMP = $TempDir
$env:TMP = $TempDir
$env:CARGO_HOME = $CargoHome
$env:RUSTUP_HOME = $RustupHome
$env:CARGO_TARGET_DIR = $CargoTarget
$env:RUSTUP_DIST_SERVER = "https://rsproxy.cn"
$env:RUSTUP_UPDATE_ROOT = "https://rsproxy.cn/rustup"
$env:PYTHONHOME = ""
$env:PYTHONPATH = ""
$env:PYTHONIOENCODING = "utf-8"

$CargoBin = Join-Path $CargoHome "bin"
if (($env:PATH -split ";") -notcontains $CargoBin) {
    $env:PATH = "$CargoBin;$env:PATH"
}

# Dynamically discover Visual Studio installation with x86/x64 VC++ tools via vswhere
$ProgFilesX86 = [Environment]::GetFolderPath("ProgramFilesX86")
$VsWhere = Join-Path $ProgFilesX86 "Microsoft Visual Studio\Installer\vswhere.exe"
$VcVars64 = $null

if (Test-Path $VsWhere) {
    $VsInstall = & $VsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1
    if ($VsInstall) {
        $Candidate = Join-Path $VsInstall "VC\Auxiliary\Build\vcvars64.bat"
        if (Test-Path $Candidate) {
            $VcVars64 = $Candidate
        }
    }
}

if (-not $VcVars64) {
    # Fallback relative probe across mounted non-system drives without hardcoding drive letters
    foreach ($drv in (Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Name -ne "C" })) {
        foreach ($rel in @("program\vs2022\VC\Auxiliary\Build\vcvars64.bat", "VisualStudio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat")) {
            $probe = Join-Path $drv.Root $rel
            if (Test-Path $probe) {
                $VcVars64 = $probe
                break
            }
        }
        if ($VcVars64) { break }
    }
}

function Invoke-WithMsvcX64 {
    param([Parameter(Mandatory = $true)][string]$CommandLine)
    if (-not $VcVars64) {
        throw "MSVC vcvars64.bat not found via vswhere or drive probe."
    }
    $wrapped = "call `"$VcVars64`" >nul && cd /d `"$RepoRoot`" && $CommandLine"
    & $env:ComSpec /c $wrapped
    if ($LASTEXITCODE -ne 0) {
        throw "MSVC command failed with exit code ${LASTEXITCODE}: $CommandLine"
    }
}
