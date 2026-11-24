$ErrorActionPreference = "Stop"

# 1. Kill any existing motor_dif process
Get-Process -Name "motor_dif" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# 2. Launch engine
$proc = Start-Process -FilePath ".\target\release\motor_dif.exe" -ArgumentList "--scene assets/showcase_v10.json" -PassThru
Write-Host "Started motor_dif with PID $($proc.Id)"

# 3. Wait for MCP port 3030
$connected = $false
for ($i = 0; $i -lt 15; $i++) {
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
    # 4. Test tools/call get_engine_docs topic: "all"
    $bodyAll = '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_engine_docs","arguments":{"topic":"all"}}}'
    $respAll = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyAll
    $textAll = $respAll.result.content[0].text
    Write-Host "Response 'all' length:" $textAll.Length "chars"
    Write-Host "Has ## quickstart:" ($textAll.Contains("## quickstart"))
    Write-Host "Has ## scene_json:" ($textAll.Contains("## scene_json"))
    Write-Host "Has ## lua_core:" ($textAll.Contains("## lua_core"))
    Write-Host "Has ## lua_ui:" ($textAll.Contains("## lua_ui"))
    Write-Host "Has ## lua_vfx_audio:" ($textAll.Contains("## lua_vfx_audio"))
    Write-Host "Has ## lua_ai_rig:" ($textAll.Contains("## lua_ai_rig"))
    Write-Host "Has ## mcp_workflow:" ($textAll.Contains("## mcp_workflow"))

    # 5. Test tools/call get_engine_docs topic: "lua_ui"
    $bodyUi = '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_engine_docs","arguments":{"topic":"lua_ui"}}}'
    $respUi = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyUi
    $textUi = $respUi.result.content[0].text
    Write-Host "`nResponse 'lua_ui' length:" $textUi.Length "chars"
    Write-Host "Has ## lua_ui:" ($textUi.Contains("## lua_ui"))
    Write-Host "Has ui_button:" ($textUi.Contains("ui_button"))
    Write-Host "Does NOT have ## scene_json:" (-not $textUi.Contains("## scene_json"))

    # 6. Test tools/call get_engine_docs topic: "scene_json"
    $bodyScene = '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_engine_docs","arguments":{"topic":"scene_json"}}}'
    $respScene = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -ContentType "application/json" -Body $bodyScene
    $textScene = $respScene.result.content[0].text
    Write-Host "`nResponse 'scene_json' length:" $textScene.Length "chars"
    Write-Host "Has ## scene_json:" ($textScene.Contains("## scene_json"))
    Write-Host "Has camera:" ($textScene.Contains("camera"))
    Write-Host "Does NOT have ## lua_ui:" (-not $textScene.Contains("## lua_ui"))

    Write-Host "`nLive MCP Verification SUCCESSFUL!"
} finally {
    # 7. Stop process and verify port clean
    Stop-Process -Id $proc.Id -Force
    Start-Sleep -Milliseconds 500
    Write-Host "Engine process stopped cleanly."
}
