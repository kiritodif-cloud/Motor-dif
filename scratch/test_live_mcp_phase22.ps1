$ErrorActionPreference = "Stop"

# 1. Kill any existing motor_dif process
Get-Process -Name "motor_dif" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# 2. Launch engine
$proc = Start-Process -FilePath ".\target\release\motor_dif.exe" -ArgumentList "--scene assets/showcase_v10.json" -PassThru
Write-Host "Started motor_dif with PID $($proc.Id)"

# 3. Wait for MCP port 3030
$connected = $false
for ($i = 0; $i -lt 20; $i++) {
    Start-Sleep -Milliseconds 500
    try {
        $ping = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body '{"jsonrpc":"2.0","id":0,"method":"ping"}' -TimeoutSec 2
        if ($ping) {
            $connected = $true
            Write-Host "Connected to MCP server!"
            break
        }
    } catch {
        # Keep waiting
    }
}

if (-not $connected) {
    Stop-Process -Id $proc.Id -Force
    throw "Could not connect to MCP server on 127.0.0.1:3030"
}

try {
    # 4. Test get_engine_docs
    $bodyDocs = '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_engine_docs","arguments":{"topic":"mcp_workflow"}}}'
    $respDocs = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyDocs
    $textDocs = $respDocs.result.content[0].text
    Write-Host "`n=== 1. Test get_engine_docs ==="
    Write-Host "Contains Working Dir:" ($textDocs.Contains("Working Dir:"))
    Write-Host "Contains load_scene:" ($textDocs.Contains("load_scene"))
    Write-Host "Contains set_play_mode:" ($textDocs.Contains("set_play_mode"))

    # 5. Test load_scene with bare name "showcase_v20_vfx.json" (normalization test)
    $bodyLoad = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"load_scene","arguments":{"path":"showcase_v20_vfx.json"}}}'
    $respLoad = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyLoad
    Write-Host "`n=== 2. Test load_scene ==="
    Write-Host "load_scene response:" ($respLoad | ConvertTo-Json -Depth 4)
    $loadData = $respLoad.result.data
    if (-not $loadData.success -or $loadData.scene -ne "assets/showcase_v20_vfx.json") {
        throw "load_scene failed or path not normalized: $loadData"
    }

    # 6. Test set_play_mode playing: true
    $bodyPlay = '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"set_play_mode","arguments":{"playing":true}}}'
    $respPlay = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyPlay
    Write-Host "`n=== 3. Test set_play_mode playing: true ==="
    Write-Host "set_play_mode response:" ($respPlay | ConvertTo-Json -Depth 4)
    if (-not $respPlay.result.data.success -or -not $respPlay.result.data.playing) {
        throw "set_play_mode playing: true failed"
    }

    # 7. Test set_play_mode playing: false
    $bodyStop = '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"set_play_mode","arguments":{"playing":false}}}'
    $respStop = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyStop
    Write-Host "`n=== 4. Test set_play_mode playing: false ==="
    Write-Host "set_play_mode stop response:" ($respStop | ConvertTo-Json -Depth 4)
    if (-not $respStop.result.data.success -or $respStop.result.data.playing) {
        throw "set_play_mode playing: false failed"
    }

    # 8. Test load_scene error handling for non-existent file
    $bodyBad = '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"load_scene","arguments":{"path":"non_existent_scene_xyz.json"}}}'
    $respBad = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyBad
    Write-Host "`n=== 5. Test load_scene error handling ==="
    Write-Host "Error response:" ($respBad | ConvertTo-Json -Depth 4)
    if (-not $respBad.error) {
        throw "Expected error for non existent scene, got success"
    }

    Write-Host "`nALL LIVE MCP TESTS PASSED SUCCESSFULLY!"
} finally {
    Stop-Process -Id $proc.Id -Force
    Start-Sleep -Milliseconds 500
    Write-Host "Engine process stopped cleanly and port 3030 is free."
}
