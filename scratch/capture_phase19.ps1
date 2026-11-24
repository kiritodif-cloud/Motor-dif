$ErrorActionPreference = "Stop"
$artifactDir = "C:\Users\difer\.gemini\antigravity\brain\4c43111a-e23d-4652-8bfd-0d250e748e9a"
$exePath = "c:\Users\difer\Desktop\Motor-dif\target\release\motor_dif.exe"
$targetArtifact = Join-Path $artifactDir "viewport_fase19_rig.png"
$targetLocal = "c:\Users\difer\Desktop\Motor-dif\viewport_fase19_rig.png"

# Make sure port 3030 and previous instances are clean
Get-Process motor_dif -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

Write-Host "=== Starting Motor-dif with showcase_v19_rig.json ==="
$proc = Start-Process -FilePath $exePath -ArgumentList "assets/showcase_v19_rig.json" -PassThru

try {
    Write-Host "Waiting 5 seconds for Procedural Rig engine, locomotion & scripts..."
    Start-Sleep -Seconds 5

    if ($proc.HasExited) {
        throw "motor_dif.exe exited unexpectedly with code $($proc.ExitCode)"
    }

    # Query performance stats
    $bodyStats = @{
        jsonrpc = "2.0"
        id = 191
        method = "tools/call"
        params = @{
            name = "get_performance_stats"
            arguments = @{}
        }
    } | ConvertTo-Json

    try {
        $statsResp = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -Body $bodyStats -ContentType "application/json"
        Write-Host "STATS: $($statsResp.result.content[0].text)"
    } catch {
        Write-Host "Warning: Could not fetch stats: $_"
    }

    $bodyCap = @{
        jsonrpc = "2.0"
        id = 192
        method = "tools/call"
        params = @{
            name = "capture_viewport"
            arguments = @{}
        }
    } | ConvertTo-Json

    Write-Host "Requesting viewport capture via MCP (http://127.0.0.1:3030/mcp)..."
    $respCap = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -Body $bodyCap -ContentType "application/json"
    $base64Data = $respCap.result.content[0].data

    if (-not $base64Data) {
        throw "Failed to receive base64 viewport data: $($respCap | ConvertTo-Json)"
    }

    $bytes = [System.Convert]::FromBase64String($base64Data)
    [System.IO.File]::WriteAllBytes($targetArtifact, $bytes)
    [System.IO.File]::WriteAllBytes($targetLocal, $bytes)
    Write-Host "SUCCESS: Saved viewport_fase19_rig.png ($($bytes.Length) bytes) to $targetArtifact and $targetLocal"
}
finally {
    Write-Host "Terminating motor_dif process..."
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Get-Process motor_dif -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Host "Port 3030 clean and verified."
}
