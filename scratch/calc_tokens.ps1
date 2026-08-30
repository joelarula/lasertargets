$files = Get-ChildItem -Path . -Recurse -File | Where-Object {
    $_.FullName -notmatch '\\(\.git|target|dist|out|temp|scratch|\.vscode|\.idea|assets)\\' -and
    $_.Name -ne 'Cargo.lock' -and
    $_.Extension -match '^\.(rs|toml|md|json|sh|dockerfile|yml|yaml)$'
}

$moduleStats = @{}
$totalBytes = 0
$totalLines = 0
$totalFiles = 0

foreach ($f in $files) {
    $len = $f.Length
    $lines = (Get-Content -LiteralPath $f.FullName | Measure-Object -Line).Lines
    $rel = $f.FullName.Substring((Get-Location).Path.Length + 1)
    
    $mod = 'root & docs'
    if ($rel -like 'server\*') { $mod = 'server' }
    elseif ($rel -like 'common\*') { $mod = 'common' }
    elseif ($rel -like 'laserlogic\*') { $mod = 'laserlogic' }
    elseif ($rel -like 'terminal\*') { $mod = 'terminal' }
    elseif ($rel -like 'minigames\hunter\*') { $mod = 'minigames/hunter' }
    elseif ($rel -like 'minigames\snake\*') { $mod = 'minigames/snake' }
    elseif ($rel -like 'gamepad\*') { $mod = 'gamepad' }
    elseif ($rel -like 'shape-editor\*') { $mod = 'shape-editor' }
    elseif ($rel -like 'dac-test\*') { $mod = 'dac-test' }
    elseif ($rel -like 'deploy\*' -or $rel -like 'docker\*' -or $rel -like 'scripts\*') { $mod = 'infra/scripts' }
    elseif ($rel -like '.agents\*') { $mod = '.agents (rules/skills)' }

    if (-not $moduleStats.ContainsKey($mod)) {
        $moduleStats[$mod] = @{ Files = 0; Lines = 0; Bytes = 0 }
    }
    $moduleStats[$mod].Files += 1
    $moduleStats[$mod].Lines += $lines
    $moduleStats[$mod].Bytes += $len

    $totalFiles += 1
    $totalLines += $lines
    $totalBytes += $len
}

$results = foreach ($k in ($moduleStats.Keys | Sort-Object)) {
    $stat = $moduleStats[$k]
    $estTokens = [math]::Round($stat.Bytes / 3.7)
    [PSCustomObject]@{
        Module     = $k
        Files      = $stat.Files
        Lines      = $stat.Lines
        Size_KB    = [math]::Round($stat.Bytes / 1024, 1)
        Est_Tokens = $estTokens
    }
}

$results | Format-Table -AutoSize
$totalEstTokens = [math]::Round($totalBytes / 3.7)
Write-Output ("`n=======================================================")
Write-Output ("TOTAL ACTIVE SOURCE CODE: {0} files, {1:N0} lines, {2:N1} KB, ~{3:N0} tokens" -f $totalFiles, $totalLines, ($totalBytes / 1024), $totalEstTokens)
Write-Output ("=======================================================")
