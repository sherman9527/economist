<#
.SYNOPSIS
    Daily runner script for The Economist A-Share / US-Stock Deep Impact Analysis.
    Strictly keeps all data, logs, checkpoints, and temp files under D:\code\economist.

.PARAMETER Action
    Analyze     - Run at 08:00 AM daily: Fetch Economist articles + Checkpointed Gemini 4 Argon (983k high) analysis + HTML Archive
    Open        - Run at 09:00 AM daily: Ensure today's report is ready, start local ACK server, and open today's report + all historical un-ACKed reports in browser
    Both        - Run Analyze then Open immediately
    Backfill3D  - Analyze and generate 3-day archived reports and open in browser
#>
param(
    [ValidateSet("Analyze", "Open", "Both", "Backfill3D")]
    [string]$Action = "Both",

    [int]$Days = 3,

    [string]$ReportDate = (Get-Date -Format "yyyy-MM-dd"),

    [int]$MaxCore = 22,

    [int]$BatchSize = 6
)

$ErrorActionPreference = "Stop"

$BaseDir = "D:\code\economist"
$ScriptsDir = Join-Path $BaseDir "scripts"
$LogsDir = Join-Path $BaseDir "logs"
$TempDir = Join-Path $BaseDir ".temp"

foreach ($d in @($BaseDir, $ScriptsDir, $LogsDir, $TempDir)) {
    if (-not (Test-Path $d)) {
        New-Item -ItemType Directory -Path $d -Force | Out-Null
    }
}

# Zero C: drive footprint & clean Python environment
$env:TEMP = $TempDir
$env:TMP = $TempDir
$env:TMPDIR = $TempDir
$env:PYTHONIOENCODING = "utf-8"
$env:PYTHONHOME = ""
$env:PYTHONPATH = ""

Set-Location $BaseDir

$ModeMap = @{
    "Analyze"    = "analyze"
    "Open"       = "open"
    "Both"       = "both"
    "Backfill3D" = "backfill-3d"
}
$PyMode = $ModeMap[$Action]

$LogFile = Join-Path $LogsDir "runner_$ReportDate.log"
$Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
Add-Content -Path $LogFile -Value "[$Timestamp] Starting run_daily.ps1 -Action $Action -Days $Days -ReportDate $ReportDate" -Encoding UTF8

$MaxRunnerAttempts = 3
for ($attempt = 1; $attempt -le $MaxRunnerAttempts; $attempt++) {
    try {
        & python -E -X utf8 "$ScriptsDir\main.py" --mode $PyMode --date $ReportDate --days $Days --max-core $MaxCore --batch-size $BatchSize
        if ($LASTEXITCODE -eq 0) {
            $DoneTs = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            Add-Content -Path $LogFile -Value "[$DoneTs] Completed run_daily.ps1 ($Action) successfully on attempt $attempt." -Encoding UTF8
            exit 0
        }
        throw "python main.py exited with code $LASTEXITCODE"
    }
    catch {
        $ErrTs = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
        Add-Content -Path $LogFile -Value "[$ErrTs] Attempt $attempt/$MaxRunnerAttempts failed: $_" -Encoding UTF8
        if ($attempt -lt $MaxRunnerAttempts) {
            Start-Sleep -Seconds (5 * $attempt)
        }
        else {
            throw
        }
    }
}
