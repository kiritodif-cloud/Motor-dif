$world = [ordered]@{
    camera = [ordered]@{
        mode = "3d"
        position = @(0.0, 3.6, 4.0)
        target = @(0.0, 3.2, 0.0)
        ortho_size = 10.0
    }
    post_process = [ordered]@{
        bloom_enabled = $true
        bloom_intensity = 0.30
        bloom_threshold = 0.80
        vignette_intensity = 0.25
        exposure = 1.05
    }
    environment = [ordered]@{
        sky_mode = "gradient"
        zenith_color = @(0.35, 0.65, 0.95)
        horizon_color = @(0.75, 0.88, 1.0)
        fog_color = @(0.75, 0.88, 1.0)
        fog_density = 0.003
    }
    entities = [System.Collections.ArrayList]::new()
}

function Add-Block($id, $x, $y, $z, $tex, $color=@(1.0,1.0,1.0,1.0), $emissive=0.0, $hasLight=$false, $vfx=$null, $collider=$true) {
    $e = [ordered]@{
        id = $id
        mesh = "cube"
        position = @([float]$x, [float]$y, [float]$z)
        rotation = @(0.0, 0.0, 0.0)
        scale = @(1.0, 1.0, 1.0)
        texture = $tex
        color = $color
        emissive = [float]$emissive
    }
    if ($collider) {
        $e["physics"] = [ordered]@{
            body = "fixed"
            collider = "cuboid"
            mass = 0.0
            restitution = 0.0
            is_sensor = $false
        }
    }
    if ($hasLight) {
        $e["point_light"] = [ordered]@{
            color = @(1.0, 0.75, 0.35)
            intensity = 3.5
            radius = 12.0
        }
    }
    if ($vfx) {
        $e["vfx_emitter"] = [ordered]@{
            preset_or_recipe = $vfx
            enabled = $true
            looping = $true
            offset = @(0.0, 0.6, 0.0)
        }
    }
    [void]$world.entities.Add($e)
}

# 1. Player Entity
$player = [ordered]@{
    id = "player_steve"
    mesh = "cube"
    position = @(0.0, 3.5, 0.0)
    rotation = @(0.0, 0.0, 0.0)
    scale = @(0.6, 1.8, 0.6)
    texture = "assets/textures/steve_face.png"
    color = @(1.0, 1.0, 1.0, 1.0)
    emissive = 0.05
    script = "assets/scripts/minecraft_game.lua"
    physics = [ordered]@{
        body = "dynamic"
        collider = "cuboid"
        mass = 1.0
        restitution = 0.0
        is_sensor = $false
    }
}
[void]$world.entities.Add($player)

# 2. Celestial Bodies: Square Minecraft Sun & Moon
$sun = [ordered]@{
    id = "celestial_sun"
    mesh = "cube"
    position = @(12.0, 40.0, -25.0)
    rotation = @(0.0, 0.0, 0.0)
    scale = @(6.0, 6.0, 0.5)
    color = @(1.0, 0.98, 0.85, 1.0)
    emissive = 0.85
    point_light = [ordered]@{
        color = @(1.0, 0.95, 0.85)
        intensity = 4.0
        radius = 80.0
    }
}
[void]$world.entities.Add($sun)

$moon = [ordered]@{
    id = "celestial_moon"
    mesh = "cube"
    position = @(-12.0, -40.0, 25.0)
    rotation = @(0.0, 0.0, 0.0)
    scale = @(5.0, 5.0, 0.5)
    color = @(0.85, 0.90, 1.0, 1.0)
    emissive = 0.70
}
[void]$world.entities.Add($moon)

# 3. Voxel Grid Terrain: 20x20 area from X=-9..10, Z=-9..10
$occupied = @{}
function Key($x, $y, $z) { return "$x,$y,$z" }

# Lake area: X in -7..-3, Z in -7..-3
$isLake = {
    param($x, $z)
    return ($x -ge -7 -and $x -le -3 -and $z -ge -7 -and $z -le -3)
}
$isBeach = {
    param($x, $z)
    return ($x -ge -8 -and $x -le -2 -and $z -ge -8 -and $z -le -2 -and -not (& $isLake $x $z))
}
# Cabin area: X in 3..7, Z in 2..6
$isCabin = {
    param($x, $z)
    return ($x -ge 3 -and $x -le 7 -and $z -ge 2 -and $z -le 6)
}

# Terrain generation
for ($x = -9; $x -le 10; $x++) {
    for ($z = -9; $z -le 10; $z++) {
        # Bedrock at Y=0
        $k0 = Key $x 0 $z
        $occupied[$k0] = $true
        Add-Block "bedrock_${x}_0_${z}" $x 0 $z "assets/textures/mc_bedrock.png"

        # Stone at Y=1
        $k1 = Key $x 1 $z
        $occupied[$k1] = $true
        # Natural ores scattered in stone
        $oreRnd = (($x * 73 + $z * 179 + 53) % 100)
        $texY1 = "assets/textures/mc_stone.png"
        $emissY1 = 0.0
        if ($oreRnd -lt 6) {
            $texY1 = "assets/textures/mc_diamond_ore.png"
            $emissY1 = 0.15
        } elseif ($oreRnd -lt 16) {
            $texY1 = "assets/textures/mc_gold_ore.png"
            $emissY1 = 0.10
        } elseif ($oreRnd -lt 32) {
            $texY1 = "assets/textures/mc_iron_ore.png"
        } elseif ($oreRnd -lt 50) {
            $texY1 = "assets/textures/mc_coal_ore.png"
        }
        Add-Block "stone_${x}_1_${z}" $x 1 $z $texY1 @(1,1,1,1) $emissY1

        # Surface layers
        if (& $isLake $x $z) {
            # Lake water at Y=2
            $k2 = Key $x 2 $z
            $occupied[$k2] = $true
            Add-Block "water_${x}_2_${z}" $x 2 $z "assets/textures/mc_water.png" @(0.3, 0.7, 1.0, 0.85) 0.08 $false $null $false
        } elseif (& $isBeach $x $z) {
            # Sand beach at Y=2
            $k2 = Key $x 2 $z
            $occupied[$k2] = $true
            Add-Block "sand_${x}_2_${z}" $x 2 $z "assets/textures/mc_sand.png"
        } elseif (& $isCabin $x $z) {
            # Cabin foundation at Y=2 (Cobblestone)
            $k2 = Key $x 2 $z
            $occupied[$k2] = $true
            Add-Block "cobb_${x}_2_${z}" $x 2 $z "assets/textures/mc_cobblestone.png"
        } else {
            # Dirt at Y=2
            $k2 = Key $x 2 $z
            $occupied[$k2] = $true
            Add-Block "dirt_${x}_2_${z}" $x 2 $z "assets/textures/mc_dirt.png"

            # Rolling hill calculation for Y=3 and Y=4
            $hillFactor = [Math]::Sin($x * 0.45) * [Math]::Cos($z * 0.45)
            $isHigh = ($hillFactor -gt 0.25 -and ($x -lt 2 -or $z -lt 1))
            $isPeak = ($hillFactor -gt 0.65 -and ($x -lt 2 -or $z -lt 1))

            if ($isPeak) {
                # Dirt at 3, Grass at 4
                $k3 = Key $x 3 $z
                $occupied[$k3] = $true
                Add-Block "dirt_${x}_3_${z}" $x 3 $z "assets/textures/mc_dirt.png"

                $k4 = Key $x 4 $z
                $occupied[$k4] = $true
                Add-Block "grass_${x}_4_${z}" $x 4 $z "assets/textures/mc_grass.png"
            } elseif ($isHigh) {
                # Grass at 3
                $k3 = Key $x 3 $z
                $occupied[$k3] = $true
                Add-Block "grass_${x}_3_${z}" $x 3 $z "assets/textures/mc_grass.png"
            } else {
                # Grass at 2.8 -> standard surface at Y=3 for clearing
                $k3 = Key $x 3 $z
                $occupied[$k3] = $true
                Add-Block "grass_${x}_3_${z}" $x 3 $z "assets/textures/mc_grass.png"
            }
        }
    }
}

# 4. Cottage / Cabin Architecture at X=3..7, Z=2..6
# Walls (Oak Planks & Cobblestone corners) at Y=3, 4, 5
for ($cx = 3; $cx -le 7; $cx++) {
    for ($cz = 2; $cz -le 6; $cz++) {
        $isPerim = ($cx -eq 3 -or $cx -eq 7 -or $cz -eq 2 -or $cz -eq 6)
        $isCorner = (($cx -eq 3 -or $cx -eq 7) -and ($cz -eq 2 -or $cz -eq 6))
        $isDoorway = ($cx -eq 5 -and $cz -eq 2)
        $isWindow = (($cx -eq 3 -or $cx -eq 7) -and $cz -eq 4) -or (($cz -eq 6) -and ($cx -eq 4 -or $cx -eq 6))

        for ($cy = 3; $cy -le 5; $cy++) {
            if ($isDoorway -and ($cy -eq 3 -or $cy -eq 4)) {
                # Open doorway
                continue
            }

            if ($isPerim) {
                $tex = "assets/textures/mc_oak_planks.png"
                if ($isCorner) {
                    $tex = "assets/textures/mc_oak_log.png"
                } elseif ($isWindow -and $cy -eq 4) {
                    $tex = "assets/textures/mc_glass.png"
                }
                Add-Block "house_${cx}_${cy}_${cz}" $cx $cy $cz $tex
            }
        }
        # Roof at Y=6 (Wood Planks)
        Add-Block "roof_${cx}_6_${cz}" $cx 6 $cz "assets/textures/mc_oak_planks.png"
    }
}

# Cabin Interior Furniture
# Crafting Table at X=4, Y=3, Z=5
Add-Block "crafting_table_1" 4 3 5 "assets/textures/mc_crafting_table.png"
# Bookshelf at X=6, Y=3, Z=5
Add-Block "bookshelf_1" 6 3 5 "assets/textures/mc_bookshelf.png"
# Furnace/Fireplace at X=3, Y=3, Z=3
Add-Block "furnace_1" 3 3 3 "assets/textures/mc_cobblestone.png" @(1,1,1,1) 0.1
# Cabin Torch at X=5, Y=4.2, Z=3 with fire VFX and PointLight
Add-Block "cabin_torch" 5 4.2 3 "assets/textures/mc_torch.png" @(1,1,1,1) 0.5 $true "fire" $false
# Porch Torch at X=5, Y=4.2, Z=1.5
Add-Block "porch_torch" 5 4.2 1.5 "assets/textures/mc_torch.png" @(1,1,1,1) 0.5 $true "fire" $false

# 5. Natural Minecraft Trees (Oak Trunks & Leaf Canopies)
$treeBases = @(
    @{ x = -3; z = 4; h = 4 },
    @{ x = -6; z = 1; h = 5 },
    @{ x = 0;  z = -6; h = 4 },
    @{ x = -5; z = 7; h = 5 },
    @{ x = 8;  z = -4; h = 4 }
)

$tIdx = 1
foreach ($t in $treeBases) {
    $tx = $t.x
    $tz = $t.z
    $th = $t.h
    $baseY = 4

    # Trunk (Oak Log)
    for ($y = $baseY; $y -lt ($baseY + $th); $y++) {
        Add-Block "tree_${tIdx}_log_${y}" $tx $y $tz "assets/textures/mc_oak_log.png"
    }

    # Canopy Leaves: 3x3x2 cube
    $topY = $baseY + $th
    for ($ly = ($topY - 2); $ly -le $topY; $ly++) {
        $radius = if ($ly -eq $topY) { 1 } else { 2 }
        for ($lx = ($tx - $radius); $lx -le ($tx + $radius); $lx++) {
            for ($lz = ($tz - $radius); $lz -le ($tz + $radius); $lz++) {
                # Don't overwrite trunk
                if ($lx -eq $tx -and $lz -eq $tz -and $ly -lt $topY) { continue }
                # Cut corners for authentic round leaf crown
                if ([Math]::Abs($lx - $tx) -eq $radius -and [Math]::Abs($lz - $tz) -eq $radius -and $ly -eq $topY) { continue }
                Add-Block "tree_${tIdx}_leaf_${lx}_${ly}_${lz}" $lx $ly $lz "assets/textures/mc_oak_leaves.png"
            }
        }
    }
    $tIdx++
}

# 6. Exposed Cave / Mine Entrance at X=-8, Z=6
Add-Block "cave_arch_1" -8 4 6 "assets/textures/mc_stone.png"
Add-Block "cave_arch_2" -7 4 6 "assets/textures/mc_cobblestone.png"
Add-Block "cave_arch_3" -6 4 6 "assets/textures/mc_stone.png"
Add-Block "cave_torch" -7 3.5 6 "assets/textures/mc_torch.png" @(1,1,1,1) 0.6 $true "fire" $false
Add-Block "cave_diamond" -8 3 7 "assets/textures/mc_diamond_ore.png" @(1,1,1,1) 0.25
Add-Block "cave_gold" -7 3 7 "assets/textures/mc_gold_ore.png" @(1,1,1,1) 0.15
Add-Block "cave_tnt" -6 3 7 "assets/textures/mc_tnt.png"

# 7. Minecraft Passive Mob: Pig
$pigBody = [ordered]@{
    id = "mob_pig_body"
    mesh = "cube"
    position = @(-2.0, 3.8, -1.5)
    rotation = @(0.0, 0.4, 0.0)
    scale = @(0.9, 0.7, 1.3)
    color = @(0.96, 0.65, 0.68, 1.0)
    emissive = 0.02
}
[void]$world.entities.Add($pigBody)

$pigHead = [ordered]@{
    id = "mob_pig_head"
    mesh = "cube"
    position = @(-2.0, 4.1, -2.2)
    rotation = @(0.0, 0.4, 0.0)
    scale = @(0.65, 0.65, 0.65)
    color = @(0.96, 0.62, 0.66, 1.0)
    emissive = 0.02
}
[void]$world.entities.Add($pigHead)

$pigSnout = [ordered]@{
    id = "mob_pig_snout"
    mesh = "cube"
    position = @(-2.0, 3.95, -2.55)
    rotation = @(0.0, 0.4, 0.0)
    scale = @(0.35, 0.22, 0.2)
    color = @(0.88, 0.48, 0.52, 1.0)
    emissive = 0.02
}
[void]$world.entities.Add($pigSnout)

# 8. Minecraft Hostile Mob: Creeper near forest edge at X=-5, Z=5
$creeperHead = [ordered]@{
    id = "mob_creeper_head"
    mesh = "cube"
    position = @(-5.0, 4.6, 4.8)
    rotation = @(0.0, 0.0, 0.0)
    scale = @(0.6, 0.6, 0.6)
    texture = "assets/textures/creeper_face.png"
    color = @(1.0, 1.0, 1.0, 1.0)
    emissive = 0.05
}
[void]$world.entities.Add($creeperHead)

$creeperBody = [ordered]@{
    id = "mob_creeper_body"
    mesh = "cube"
    position = @(-5.0, 3.8, 4.8)
    rotation = @(0.0, 0.0, 0.0)
    scale = @(0.5, 0.9, 0.35)
    color = @(0.28, 0.68, 0.28, 1.0)
    emissive = 0.02
}
[void]$world.entities.Add($creeperBody)

# 9. First-Person Steve Hand / Held Tool Item
$steveHand = [ordered]@{
    id = "steve_held_tool"
    mesh = "cube"
    position = @(0.35, 2.9, 0.6)
    rotation = @(-0.35, 0.45, -0.2)
    scale = @(0.18, 0.45, 0.18)
    texture = "assets/textures/mc_diamond_ore.png"
    color = @(0.4, 0.95, 1.0, 1.0)
    emissive = 0.15
}
[void]$world.entities.Add($steveHand)

# Save scene to both assets directories and workspace
$jsonStr = $world | ConvertTo-Json -Depth 10

$destPaths = @(
    "C:\Users\difer\Desktop\Motor-dif\assets\minecraft_world.json",
    "C:\Users\difer\Desktop\Motor-dif\target\release\assets\minecraft_world.json",
    "C:\Users\difer\Desktop\minecraft rust\minecraft_world.json"
)

foreach ($p in $destPaths) {
    [System.IO.File]::WriteAllText($p, $jsonStr, [System.Text.Encoding]::UTF8)
}

Write-Output "Successfully built minecraft_world.json with $($world.entities.Count) entities!"
