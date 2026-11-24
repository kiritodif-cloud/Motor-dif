$ErrorActionPreference = "Stop"
$artifactDir = "C:\Users\difer\.gemini\antigravity\brain\4c43111a-e23d-4652-8bfd-0d250e748e9a"
$exePath = "c:\Users\difer\Desktop\Motor-dif\target\release\motor_dif.exe"

# Make sure port 3030 is clean
Get-Process motor_dif -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

Write-Host "=== PART 1: Capturing Cosmos.io 3D Play Mode ==="
$targetCosmos = Join-Path $artifactDir "viewport_cosmos_final.png"
$targetCosmosLocal = "c:\Users\difer\Desktop\Motor-dif\viewport_cosmos_final.png"

$proc1 = Start-Process -FilePath $exePath -ArgumentList "--play","assets/cosmos_scene.json" -PassThru
try {
    Write-Host "Waiting 5 seconds for cosmos gameplay simulation & HUD..."
    Start-Sleep -Seconds 5

    if ($proc1.HasExited) {
        throw "motor_dif.exe exited unexpectedly with code $($proc1.ExitCode)"
    }

    $bodyCap = @{
        jsonrpc = "2.0"
        id = 1
        method = "tools/call"
        params = @{
            name = "capture_viewport"
            arguments = @{}
        }
    } | ConvertTo-Json

    $respCap = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -Body $bodyCap -ContentType "application/json"
    $base64Data = $respCap.result.content[0].data

    if (-not $base64Data) {
        throw "Failed to receive base64 viewport data: $($respCap | ConvertTo-Json)"
    }

    $bytes = [System.Convert]::FromBase64String($base64Data)
    [System.IO.File]::WriteAllBytes($targetCosmos, $bytes)
    [System.IO.File]::WriteAllBytes($targetCosmosLocal, $bytes)
    Write-Host "Saved cosmos viewport capture ($($bytes.Length) bytes) to $targetCosmos"
}
finally {
    Stop-Process -Id $proc1.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

Write-Host "=== PART 2: Capturing Asset Forge Modeler ==="
$targetForge = Join-Path $artifactDir "viewport_fase13_forge.png"
$targetForgeLocal = "c:\Users\difer\Desktop\Motor-dif\viewport_fase13_forge.png"

$proc2 = Start-Process -FilePath $exePath -PassThru
try {
    Write-Host "Waiting 4 seconds for editor initialization..."
    Start-Sleep -Seconds 4

    if ($proc2.HasExited) {
        throw "motor_dif.exe editor exited unexpectedly with code $($proc2.ExitCode)"
    }

    # Switch to Modeler workspace using forge_create_model with spaceship recipe
    $recipeObj = @{
        name = "spaceship"
        normalize_pivot = $false
        parts = @(
            @{
                name = "fuselage"
                primitive = "box"
                position = @(0.0, 0.0, 0.0)
                rotation_deg = @(0.0, 0.0, 0.0)
                scale = @(0.8, 0.4, 2.2)
                color = @(0.2, 0.6, 0.9, 1.0)
                modifiers = @()
            },
            @{
                name = "cockpit"
                primitive = "sphere"
                position = @(0.0, 0.25, -0.4)
                rotation_deg = @(0.0, 0.0, 0.0)
                scale = @(0.5, 0.35, 0.8)
                color = @(0.1, 0.9, 1.0, 1.0)
                modifiers = @()
            },
            @{
                name = "left_wing"
                primitive = "cylinder"
                position = @(-1.2, -0.05, 0.2)
                rotation_deg = @(0.0, 0.0, 90.0)
                scale = @(0.15, 1.4, 0.6)
                color = @(0.15, 0.45, 0.75, 1.0)
                modifiers = @()
            },
            @{
                name = "right_wing"
                primitive = "cylinder"
                position = @(1.2, -0.05, 0.2)
                rotation_deg = @(0.0, 0.0, -90.0)
                scale = @(0.15, 1.4, 0.6)
                color = @(0.15, 0.45, 0.75, 1.0)
                modifiers = @()
            }
        )
    }

    $bodyForge = @{
        jsonrpc = "2.0"
        id = 2
        method = "tools/call"
        params = @{
            name = "forge_create_model"
            arguments = @{
                recipe = $recipeObj
            }
        }
    } | ConvertTo-Json -Depth 6

    Write-Host "Sending forge_create_model..."
    $respForge = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -Body $bodyForge -ContentType "application/json"
    Write-Host "Forge model loaded: $($respForge.result.content[0].text)"

    Start-Sleep -Seconds 1

    # Call forge_capture_turnaround
    $bodyTurn = @{
        jsonrpc = "2.0"
        id = 3
        method = "tools/call"
        params = @{
            name = "forge_capture_turnaround"
            arguments = @{
                angle = "iso"
            }
        }
    } | ConvertTo-Json

    Write-Host "Calling forge_capture_turnaround..."
    $respTurn = Invoke-RestMethod -Uri "http://127.0.0.1:3030/mcp" -Method Post -Body $bodyTurn -ContentType "application/json"
    $base64Turn = $respTurn.result.content[0].data

    if (-not $base64Turn) {
        throw "Failed to receive base64 turnaround data: $($respTurn | ConvertTo-Json)"
    }

    $bytesTurn = [System.Convert]::FromBase64String($base64Turn)
    [System.IO.File]::WriteAllBytes($targetForge, $bytesTurn)
    [System.IO.File]::WriteAllBytes($targetForgeLocal, $bytesTurn)
    Write-Host "Saved forge viewport turnaround ($($bytesTurn.Length) bytes) to $targetForge"
}
finally {
    Stop-Process -Id $proc2.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    Write-Host "All processes cleaned up."
}
