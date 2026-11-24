$ErrorActionPreference = "Stop"
$artifactDir = "C:\Users\difer\.gemini\antigravity\brain\4c43111a-e23d-4652-8bfd-0d250e748e9a"
$exePath = "c:\Users\difer\Desktop\Motor-dif\target\release\motor_dif.exe"
$targetArtifact = Join-Path $artifactDir "viewport_fase18_level.png"
$targetLocal = "c:\Users\difer\Desktop\Motor-dif\viewport_fase18_level.png"

# Make sure port 3030 and previous instances are clean
Get-Process motor_dif -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

Write-Host "=== Starting Motor-dif with showcase_v18_terrain.json ==="
$proc = Start-Process -FilePath $exePath -ArgumentList "assets/showcase_v18_terrain.json" -PassThru

try {
    Write-Host "Waiting 5 seconds for Level Forge terrain, scout script & post-processing..."
    Start-Sleep -Seconds 5

    if ($proc.HasExited) {
        throw "motor_dif.exe exited unexpectedly with code $($proc.ExitCode)"
    }

    $bodyCap = @{
        jsonrpc = "2.0"
        id = 18
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
    Write-Host "SUCCESS: Saved viewport_fase18_level.png ($($bytes.Length) bytes) to $targetArtifact and $targetLocal"
}
finally {
    Write-Host "Terminating motor_dif process..."
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Get-Process motor_dif -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Host "Port 3030 clean and verified."
}
