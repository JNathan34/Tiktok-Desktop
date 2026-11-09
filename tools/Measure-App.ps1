param(
    [Parameter(Mandatory = $true)][int]$AppProcessId,
    [ValidateRange(2, 60)][int]$Seconds = 10
)
$ErrorActionPreference = 'Stop'

function Get-AppTree {
    $rows = Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    [void]$ids.Add($AppProcessId)
    do {
        $previousCount = $ids.Count
        foreach ($row in $rows) {
            if ($ids.Contains([int]$row.ParentProcessId)) {
                [void]$ids.Add([int]$row.ProcessId)
            }
        }
    } while ($ids.Count -ne $previousCount)
    Get-Process -Id @($ids) -ErrorAction SilentlyContinue
}

$startProcesses = @(Get-AppTree)
if ($AppProcessId -notin $startProcesses.Id) { throw 'The app process is not running.' }
$cpuStart = @{}
foreach ($process in $startProcesses) { $cpuStart[$process.Id] = $process.CPU }
$timer = [System.Diagnostics.Stopwatch]::StartNew()
Start-Sleep -Seconds $Seconds
$endProcesses = @(Get-AppTree)
$elapsed = $timer.Elapsed.TotalSeconds
if ($AppProcessId -notin $endProcesses.Id) { throw 'The app exited while sampling.' }
$cpu = 0
foreach ($process in $endProcesses) {
    $cpu += [Math]::Max(0, $process.CPU - $cpuStart[$process.Id])
}
$systemProcessors = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
[pscustomobject]@{
    Seconds = [Math]::Round($elapsed, 2)
    ProcessCount = $endProcesses.Count
    ProcessCountChanged = @(Compare-Object $startProcesses.Id $endProcesses.Id).Count -ne 0
    WorkingSetMiB = [Math]::Round(($endProcesses | Measure-Object WorkingSet64 -Sum).Sum / 1MB, 1)
    PrivateCommitMiB = [Math]::Round(($endProcesses | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB, 1)
    LogicalCorePercent = [Math]::Round(100 * $cpu / $elapsed, 2)
    SystemCpuPercent = [Math]::Round(100 * $cpu / $elapsed / $systemProcessors, 2)
} | ConvertTo-Json -Compress
