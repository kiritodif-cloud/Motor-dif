$ErrorActionPreference = "Stop"
$artifactDir = "C:\Users\difer\.gemini\antigravity\brain\4c43111a-e23d-4652-8bfd-0d250e748e9a"
$exePath = "c:\Users\difer\Desktop\Motor-dif\target\release\motor_dif.exe"
$targetArtifact = Join-Path $artifactDir "viewport_open_scene_editor.png"
$targetLocal = "c:\Users\difer\Desktop\Motor-dif\viewport_open_scene_editor.png"

Get-Process motor_dif -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

Write-Host "=== Starting Motor-dif Release Editor ==="
$proc = Start-Process -FilePath $exePath -ArgumentList "assets/scene.json" -PassThru

try {
    Start-Sleep -Seconds 4
    if ($proc.HasExited) {
        throw "motor_dif.exe exited unexpectedly with code $($proc.ExitCode)"
    }

    # Query performance stats
    $bodyStats = @{
        jsonrpc = "2.0"
        id = 301
        method = "tools/call"
        params = @{
            name = "get_performance_stats"
            arguments = @{}
        }
    } | ConvertTo-Json

    $statsResp = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -Body $bodyStats -ContentType "application/json"
    Write-Host "STATS: $($statsResp.result.content[0].text)"

    # Capture viewport
    $bodyCap = @{
        jsonrpc = "2.0"
        id = 302
        method = "tools/call"
        params = @{
            name = "capture_viewport"
            arguments = @{}
        }
    } | ConvertTo-Json

    $respCap = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -Body $bodyCap -ContentType "application/json"
    $base64Data = $respCap.result.content[0].data

    if ($base64Data) {
        $bytes = [System.Convert]::FromBase64String($base64Data)
        [System.IO.File]::WriteAllBytes($targetArtifact, $bytes)
        [System.IO.File]::WriteAllBytes($targetLocal, $bytes)
        Write-Host "SUCCESS: Captured viewport screenshot ($($bytes.Length) bytes)"
    }
}
finally {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Get-Process motor_dif -ErrorAction SilentlyContinue | Stop-Process -Force
    Write-Host "Port 3030 clean."
}
