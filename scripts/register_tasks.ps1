<#
.SYNOPSIS
    Registers the two Windows Scheduled Tasks for The Economist Daily Analysis:
      1. EconomistDaily_0800_Analyze : Runs daily at 08:00 AM (fetches + deep Gemini 4 Argon analysis + checkpoints + YYYY/MM/DD archive)
      2. EconomistDaily_0900_Open    : Runs daily at 09:00 AM (opens today's report + all previous un-ACKed reports in browser)
#>

$ErrorActionPreference = "Stop"

$RunScript = "D:\code\economist\scripts\run_daily.ps1"
$WorkDir   = "D:\code\economist"

$TaskAnalyzeName = "EconomistDaily_0800_Analyze"
$TaskOpenName    = "EconomistDaily_0900_Open"

$Settings = New-ScheduledTaskSettingsSet `
    -AllowStartIfOnBatteries `
    -DontStopIfGoingOnBatteries `
    -StartWhenAvailable `
    -ExecutionTimeLimit (New-TimeSpan -Hours 2) `
    -MultipleInstances IgnoreNew

# 1. 08:00 AM Task - Fetch & Deep Analysis
$ActionAnalyze = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$RunScript`" -Action Analyze -Days 2" `
    -WorkingDirectory $WorkDir

$TriggerAnalyze = New-ScheduledTaskTrigger -Daily -At "08:00AM"

Register-ScheduledTask `
    -TaskName $TaskAnalyzeName `
    -Action $ActionAnalyze `
    -Trigger $TriggerAnalyze `
    -Settings $Settings `
    -Description "Daily 08:00 AM: Fetch latest Economist articles and run checkpointed Copilot (gemini-4-argon 983k high) A-share/US-stock impact analysis into D:\code\economist\reports\YYYY\MM\DD" `
    -Force | Out-Null

# 2. 09:00 AM Task - Open Today's Report + All Un-ACKed Reports in Browser
$ActionOpen = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$RunScript`" -Action Open -Days 2" `
    -WorkingDirectory $WorkDir

$TriggerOpen = New-ScheduledTaskTrigger -Daily -At "09:00AM"

Register-ScheduledTask `
    -TaskName $TaskOpenName `
    -Action $ActionOpen `
    -Trigger $TriggerOpen `
    -Settings $Settings `
    -Description "Daily 09:00 AM: Start local ACK server and open today's Economist report plus any historical un-ACKed reports in browser" `
    -Force | Out-Null

Write-Output "=== Registered Scheduled Tasks Successfully ==="
Get-ScheduledTask -TaskName "EconomistDaily_*" | Select-Object TaskName, State, Description | Format-Table -AutoSize
Get-ScheduledTask -TaskName "EconomistDaily_*" | Get-ScheduledTaskInfo | Select-Object TaskName, NextRunTime, LastRunTime | Format-Table -AutoSize
