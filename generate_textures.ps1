Add-Type -AssemblyName System.Drawing

$destDirs = @(
    "C:\Users\difer\Desktop\Motor-dif\assets\textures",
    "C:\Users\difer\Desktop\Motor-dif\target\release\assets\textures",
    "C:\Users\difer\Desktop\minecraft rust\textures"
)

foreach ($d in $destDirs) {
    if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d -Force | Out-Null }
}

function Save-All($bmp, $name) {
    foreach ($d in $destDirs) {
        $path = Join-Path $d $name
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
}

$rnd = New-Object System.Random(1337)

# Helper: make noise block
function New-NoiseBmp($palette, $size=16) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    for ($y = 0; $y -lt $size; $y++) {
        for ($x = 0; $x -lt $size; $x++) {
            $idx = $rnd.Next(0, $palette.Count)
            $c = [System.Drawing.ColorTranslator]::FromHtml($palette[$idx])
            $bmp.SetPixel($x, $y, $c)
        }
    }
    return $bmp
}

# 1. mc_dirt.png
$dirtPalette = @("#866043", "#755238", "#976D4D", "#5C3E26", "#8F6647", "#6E4A2E")
$dirtBmp = New-NoiseBmp $dirtPalette
Save-All $dirtBmp "mc_dirt.png"
$dirtBmp.Dispose()

# 2. mc_grass_top.png
$grassTopPalette = @("#4E9432", "#55A236", "#44822B", "#5EB33C", "#3C7426", "#68C643")
$grassTopBmp = New-NoiseBmp $grassTopPalette
Save-All $grassTopBmp "mc_grass_top.png"
$grassTopBmp.Dispose()

# 3. mc_grass.png (Iconic side with grass fringe dripping onto dirt)
$grassSideBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        $drip = 3 + ($x * 7 % 3) # fringe pattern 3-5 px deep
        if ($y -lt $drip) {
            $idx = $rnd.Next(0, $grassTopPalette.Count)
            $c = [System.Drawing.ColorTranslator]::FromHtml($grassTopPalette[$idx])
        } else {
            $idx = $rnd.Next(0, $dirtPalette.Count)
            $c = [System.Drawing.ColorTranslator]::FromHtml($dirtPalette[$idx])
        }
        $grassSideBmp.SetPixel($x, $y, $c)
    }
}
Save-All $grassSideBmp "mc_grass.png"
$grassSideBmp.Dispose()

# 4. mc_stone.png
$stonePalette = @("#7F7F7F", "#6D6D6D", "#8C8C8C", "#5E5E5E", "#969696", "#737373")
$stoneBmp = New-NoiseBmp $stonePalette
Save-All $stoneBmp "mc_stone.png"
$stoneBmp.Dispose()

# 5. mc_cobblestone.png
$cobbBmp = New-Object System.Drawing.Bitmap(16, 16)
$cobbP = @("#7A7A7A", "#646464", "#8E8E8E", "#585858", "#9E9E9E")
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        # Mortar grid pattern
        if (($y % 4 -eq 0) -or (($y % 8 -lt 4) -and ($x % 8 -eq 0)) -or (($y % 8 -ge 4) -and (($x + 4) % 8 -eq 0))) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#3D3D3D")
        } else {
            $idx = $rnd.Next(0, $cobbP.Count)
            $c = [System.Drawing.ColorTranslator]::FromHtml($cobbP[$idx])
        }
        $cobbBmp.SetPixel($x, $y, $c)
    }
}
Save-All $cobbBmp "mc_cobblestone.png"
$cobbBmp.Dispose()

# 6. mc_oak_log.png (Bark with vertical grain)
$barkBmp = New-Object System.Drawing.Bitmap(16, 16)
$barkP = @("#6B5130", "#523C21", "#7E623E", "#46331C", "#5D4528")
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        $colIdx = ($x * 3 + ($y % 3)) % $barkP.Count
        $c = [System.Drawing.ColorTranslator]::FromHtml($barkP[$colIdx])
        $barkBmp.SetPixel($x, $y, $c)
    }
}
Save-All $barkBmp "mc_oak_log.png"
$barkBmp.Dispose()

# 7. mc_oak_planks.png
$plankBmp = New-Object System.Drawing.Bitmap(16, 16)
$plankP = @("#A58352", "#B89462", "#947242", "#C6A371", "#886737")
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($y % 4 -eq 0) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#543F20")
        } elseif (($x -eq 1 -or $x -eq 14) -and ($y % 4 -eq 2)) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#423219") # nail
        } else {
            $idx = $rnd.Next(0, $plankP.Count)
            $c = [System.Drawing.ColorTranslator]::FromHtml($plankP[$idx])
        }
        $plankBmp.SetPixel($x, $y, $c)
    }
}
Save-All $plankBmp "mc_oak_planks.png"
$plankBmp.Dispose()

# 8. mc_oak_leaves.png
$leafP = @("#25631E", "#307C27", "#1C4E17", "#3CA231", "#153A11", "#2B6E22")
$leafBmp = New-NoiseBmp $leafP
Save-All $leafBmp "mc_oak_leaves.png"
$leafBmp.Dispose()

# 9. mc_diamond_ore.png
$diamondBmp = New-NoiseBmp $stonePalette
$gemCoords = @(
    @(3,4), @(4,4), @(4,5), @(5,5),
    @(10,3), @(11,3), @(11,4),
    @(7,9), @(8,9), @(8,10), @(7,11),
    @(3,12), @(4,12), @(12,11), @(13,12)
)
$diaP = @("#5DF6FF", "#28D3E0", "#E0FFFF", "#159BA5")
foreach ($pt in $gemCoords) {
    $c = [System.Drawing.ColorTranslator]::FromHtml($diaP[$rnd.Next(0, $diaP.Count)])
    $diamondBmp.SetPixel($pt[0], $pt[1], $c)
}
Save-All $diamondBmp "mc_diamond_ore.png"
$diamondBmp.Dispose()

# 10. mc_coal_ore.png
$coalBmp = New-NoiseBmp $stonePalette
$coalP = @("#1F1F1F", "#111111", "#333333", "#282828")
foreach ($pt in $gemCoords) {
    $c = [System.Drawing.ColorTranslator]::FromHtml($coalP[$rnd.Next(0, $coalP.Count)])
    $coalBmp.SetPixel($pt[0], $pt[1], $c)
}
Save-All $coalBmp "mc_coal_ore.png"
$coalBmp.Dispose()

# 11. mc_iron_ore.png
$ironBmp = New-NoiseBmp $stonePalette
$ironP = @("#D8AF93", "#BA8C70", "#EDC4A8", "#9E7358")
foreach ($pt in $gemCoords) {
    $c = [System.Drawing.ColorTranslator]::FromHtml($ironP[$rnd.Next(0, $ironP.Count)])
    $ironBmp.SetPixel($pt[0], $pt[1], $c)
}
Save-All $ironBmp "mc_iron_ore.png"
$ironBmp.Dispose()

# 12. mc_gold_ore.png
$goldBmp = New-NoiseBmp $stonePalette
$goldP = @("#FCEE4B", "#E5B81B", "#FFF585", "#C4970E")
foreach ($pt in $gemCoords) {
    $c = [System.Drawing.ColorTranslator]::FromHtml($goldP[$rnd.Next(0, $goldP.Count)])
    $goldBmp.SetPixel($pt[0], $pt[1], $c)
}
Save-All $goldBmp "mc_gold_ore.png"
$goldBmp.Dispose()

# 13. mc_sand.png
$sandP = @("#DBD3A0", "#E3DCB1", "#C7BE8A", "#D1C894", "#EDE7C5")
$sandBmp = New-NoiseBmp $sandP
Save-All $sandBmp "mc_sand.png"
$sandBmp.Dispose()

# 14. mc_water.png
$waterP = @("#2F65D6", "#3A73E8", "#2354BF", "#4B85F7", "#1D47A6")
$waterBmp = New-NoiseBmp $waterP
Save-All $waterBmp "mc_water.png"
$waterBmp.Dispose()

# 15. mc_crafting_table.png
$craftBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($x -eq 0 -or $x -eq 15 -or $y -eq 0 -or $y -eq 15) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#52361B")
        } elseif ($y -eq 7 -or $y -eq 8) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#3D2711")
        } elseif ($x -ge 4 -and $x -le 11 -and $y -ge 2 -and $y -le 5) {
            # Saw graphic
            $c = [System.Drawing.ColorTranslator]::FromHtml("#B8B8B8")
        } else {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#966F3E")
        }
        $craftBmp.SetPixel($x, $y, $c)
    }
}
Save-All $craftBmp "mc_crafting_table.png"
$craftBmp.Dispose()

# 16. mc_bricks.png
$brickBmp = New-Object System.Drawing.Bitmap(16, 16)
$brickP = @("#9E4032", "#B34A3B", "#8C362A", "#C25444")
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($y % 4 -eq 0) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#D8D0C7")
        } elseif (($y % 8 -lt 4 -and $x % 8 -eq 0) -or ($y % 8 -ge 4 -and ($x + 4) % 8 -eq 0)) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#D8D0C7")
        } else {
            $c = [System.Drawing.ColorTranslator]::FromHtml($brickP[$rnd.Next(0, $brickP.Count)])
        }
        $brickBmp.SetPixel($x, $y, $c)
    }
}
Save-All $brickBmp "mc_bricks.png"
$brickBmp.Dispose()

# 17. mc_torch.png
$torchBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($x -ge 7 -and $x -le 8 -and $y -ge 6 -and $y -le 15) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#6E4D25") # stick
        } elseif ($x -ge 6 -and $x -le 9 -and $y -ge 3 -and $y -le 5) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#FFA500") # flame
        } elseif ($x -ge 7 -and $x -le 8 -and $y -ge 1 -and $y -le 2) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#FFFF99") # inner flame
        } else {
            $c = [System.Drawing.Color]::FromArgb(0, 0, 0, 0)
        }
        $torchBmp.SetPixel($x, $y, $c)
    }
}
Save-All $torchBmp "mc_torch.png"
$torchBmp.Dispose()

# 18. mc_bedrock.png
$bedrockP = @("#111111", "#222222", "#353535", "#080808", "#4A4A4A")
$bedrockBmp = New-NoiseBmp $bedrockP
Save-All $bedrockBmp "mc_bedrock.png"
$bedrockBmp.Dispose()

# 19. mc_glass.png
$glassBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($x -eq 0 -or $x -eq 15 -or $y -eq 0 -or $y -eq 15) {
            $c = [System.Drawing.Color]::FromArgb(240, 210, 230, 240) # light border
        } elseif (($x -eq $y -and $x -ge 3 -and $x -le 6) -or ($x -eq $y + 4 -and $x -ge 9 -and $x -le 12)) {
            $c = [System.Drawing.Color]::FromArgb(220, 255, 255, 255) # glare
        } else {
            $c = [System.Drawing.Color]::FromArgb(70, 180, 220, 240) # translucent
        }
        $glassBmp.SetPixel($x, $y, $c)
    }
}
Save-All $glassBmp "mc_glass.png"
$glassBmp.Dispose()

# 20. mc_tnt.png
$tntBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($y -ge 6 -and $y -le 9) {
            $c = [System.Drawing.Color]::FromArgb(255, 245, 245, 245) # white label
        } else {
            $c = [System.Drawing.Color]::FromArgb(255, 210, 35, 20) # red
        }
        $tntBmp.SetPixel($x, $y, $c)
    }
}
Save-All $tntBmp "mc_tnt.png"
$tntBmp.Dispose()

# 21. mc_bookshelf.png
$bookBmp = New-Object System.Drawing.Bitmap(16, 16)
$bookColors = @("#9E2A2B", "#2A6F97", "#386641", "#8338EC", "#BC6C25")
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($y -eq 0 -or $y -eq 1 -or $y -eq 7 -or $y -eq 8 -or $y -eq 14 -or $y -eq 15 -or $x -eq 0 -or $x -eq 15) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#A58352") # shelf wood
        } else {
            $c = [System.Drawing.ColorTranslator]::FromHtml($bookColors[($x * 3) % $bookColors.Count])
        }
        $bookBmp.SetPixel($x, $y, $c)
    }
}
Save-All $bookBmp "mc_bookshelf.png"
$bookBmp.Dispose()

# 22. steve_face.png
$steveBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($y -lt 4) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#4A3222") # hair
        } elseif ($y -ge 7 -and $y -le 8 -and ($x -eq 4 -or $x -eq 5 -or $x -eq 10 -or $x -eq 11)) {
            if ($x -eq 5 -or $x -eq 10) {
                $c = [System.Drawing.ColorTranslator]::FromHtml("#2C3580") # blue eye
            } else {
                $c = [System.Drawing.ColorTranslator]::FromHtml("#FFFFFF") # white eye
            }
        } elseif ($y -eq 10 -and $x -ge 7 -and $x -le 8) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#A56345") # nose
        } elseif ($y -eq 12 -and $x -ge 5 -and $x -le 10) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#5C3E26") # mouth/beard
        } else {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#D8A580") # skin
        }
        $steveBmp.SetPixel($x, $y, $c)
    }
}
Save-All $steveBmp "steve_face.png"
$steveBmp.Dispose()

# 23. creeper_face.png
$creeperBmp = New-Object System.Drawing.Bitmap(16, 16)
$greenP = @("#45A045", "#55B055", "#3A8A3A", "#4EB84E")
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        # Eyes: 3..6 & 9..12 at y=4..7
        # Mouth: central pattern
        $isEye = ($y -ge 4 -and $y -le 7 -and (($x -ge 3 -and $x -le 6) -or ($x -ge 9 -and $x -le 12)))
        $isNose = ($y -ge 7 -and $y -le 9 -and ($x -ge 7 -and $x -le 8))
        $isMouth = ($y -ge 9 -and $y -le 13 -and ($x -ge 5 -and $x -le 10) -and -not ($y -eq 13 -and ($x -eq 7 -or $x -eq 8)))
        if ($isEye -or $isNose -or $isMouth) {
            $c = [System.Drawing.Color]::FromArgb(255, 16, 16, 16)
        } else {
            $c = [System.Drawing.ColorTranslator]::FromHtml($greenP[$rnd.Next(0, $greenP.Count)])
        }
        $creeperBmp.SetPixel($x, $y, $c)
    }
}
Save-All $creeperBmp "creeper_face.png"
$creeperBmp.Dispose()

Write-Output "Successfully generated 23 authentic Minecraft textures!"
