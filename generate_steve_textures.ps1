Add-Type -AssemblyName System.Drawing

$destDirs = @(
    "C:\Users\difer\Desktop\Motor-dif\assets\textures",
    "C:\Users\difer\Desktop\Motor-dif\target\release\assets\textures",
    "C:\Users\difer\Desktop\minecraft rust\textures"
)

function Save-All($bmp, $name) {
    foreach ($d in $destDirs) {
        $path = Join-Path $d $name
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    }
}

# 1. mc_steve_shirt.png
$shirtBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if ($y -lt 3 -and $x -ge 6 -and $x -le 9) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#D8A580") # skin neckline
        } elseif (($x + $y) % 3 -eq 0) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#00848E") # dark cyan
        } else {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#00A2AF") # cyan
        }
        $shirtBmp.SetPixel($x, $y, $c)
    }
}
Save-All $shirtBmp "mc_steve_shirt.png"
$shirtBmp.Dispose()

# 2. mc_steve_pants.png
$pantsBmp = New-Object System.Drawing.Bitmap(16, 16)
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        if (($x + $y) % 3 -eq 0) {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#1C2C6B") # dark denim
        } else {
            $c = [System.Drawing.ColorTranslator]::FromHtml("#273D96") # denim blue
        }
        $pantsBmp.SetPixel($x, $y, $c)
    }
}
Save-All $pantsBmp "mc_steve_pants.png"
$pantsBmp.Dispose()

Write-Output "Steve shirt and pants textures generated!"
