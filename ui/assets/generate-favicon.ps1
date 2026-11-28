$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$assets = $PSScriptRoot
$sourcePath = Join-Path $assets 'favicon-source.png'

function New-LeafPath([string] $direction) {
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    switch ($direction) {
        'left' {
            $path.AddBezier(250, 153, 219, 119, 163, 103, 119, 116)
            $path.AddBezier(119, 116, 145, 155, 192, 176, 250, 167)
        }
        'right' {
            $path.AddBezier(262, 153, 293, 119, 349, 103, 393, 116)
            $path.AddBezier(393, 116, 367, 155, 320, 176, 262, 167)
        }
        'lower-left' {
            $path.AddBezier(250, 158, 219, 163, 193, 182, 184, 213)
            $path.AddBezier(184, 213, 220, 203, 244, 184, 257, 164)
        }
        'lower-right' {
            $path.AddBezier(262, 158, 293, 163, 319, 182, 328, 213)
            $path.AddBezier(328, 213, 292, 203, 268, 184, 255, 164)
        }
        'top' {
            $path.AddBezier(250, 157, 237, 126, 242, 87, 266, 60)
            $path.AddBezier(266, 60, 291, 91, 278, 131, 262, 160)
        }
    }
    $path.CloseFigure()
    return $path
}

function New-BerryBitmap([int] $size) {
    $bitmap = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.ScaleTransform($size / 512.0, $size / 512.0)

    $berry = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $berry.AddBezier(252, 125, 208, 131, 153, 158, 126, 207)
    $berry.AddBezier(126, 207, 97, 260, 110, 352, 156, 416)
    $berry.AddBezier(156, 416, 186, 459, 224, 483, 264, 491)
    $berry.AddBezier(264, 491, 309, 476, 354, 435, 378, 379)
    $berry.AddBezier(378, 379, 408, 310, 392, 223, 352, 176)
    $berry.AddBezier(352, 176, 324, 142, 287, 126, 252, 125)
    $berry.CloseFigure()
    $berryRect = [System.Drawing.Rectangle]::new(100, 120, 310, 370)
    $berryBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
        $berryRect,
        [System.Drawing.Color]::FromArgb(255, 40, 222, 169),
        [System.Drawing.Color]::FromArgb(255, 20, 77, 105),
        48
    )
    $edgePen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 7, 50, 65), 15)
    $edgePen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    $graphics.FillPath($berryBrush, $berry)
    $graphics.DrawPath($edgePen, $berry)

    $facet = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $facet.AddBezier(159, 217, 189, 177, 220, 157, 254, 149)
    $facet.AddBezier(254, 149, 222, 229, 220, 335, 257, 448)
    $facet.AddBezier(257, 448, 196, 425, 153, 376, 143, 316)
    $facet.AddBezier(143, 316, 137, 276, 143, 241, 159, 217)
    $facet.CloseFigure()
    $facetBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(36, 196, 255, 224))
    $graphics.FillPath($facetBrush, $facet)

    $leafDark = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 8, 72, 67))
    $leafLight = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 171, 245, 117))
    foreach ($leaf in @('left', 'right', 'lower-left', 'lower-right', 'top')) {
        $path = New-LeafPath $leaf
        $graphics.FillPath($leafDark, $path)
        $path.Dispose()
    }
    $topLeaf = New-LeafPath 'top'
    $graphics.FillPath($leafLight, $topLeaf)
    $topLeaf.Dispose()
    $veinPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(180, 211, 255, 166), 5)
    $veinPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $veinPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $graphics.DrawLine($veinPen, 255, 153, 142, 121)
    $graphics.DrawLine($veinPen, 257, 153, 370, 121)

    $bubble = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $bubble.AddBezier(196, 236, 180, 236, 168, 246, 168, 260)
    $bubble.AddLine(168, 260, 168, 270)
    $bubble.AddBezier(168, 270, 168, 282, 177, 291, 189, 291)
    $bubble.AddLine(189, 291, 294, 291)
    $bubble.AddLine(294, 291, 322, 312)
    $bubble.AddBezier(322, 312, 329, 317, 339, 312, 339, 303)
    $bubble.AddLine(339, 303, 339, 257)
    $bubble.AddBezier(339, 257, 339, 245, 330, 236, 318, 236)
    $bubble.AddLine(318, 236, 196, 236)
    $bubble.CloseFigure()
    $bubbleShadow = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(70, 3, 37, 54))
    $bubbleBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 244, 255, 231))
    $shadowMatrix = [System.Drawing.Drawing2D.Matrix]::new()
    $shadowMatrix.Translate(0, 8)
    $bubble.Transform($shadowMatrix)
    $graphics.FillPath($bubbleShadow, $bubble)
    $shadowMatrix.Translate(0, -8)
    $bubble.Transform($shadowMatrix)
    $graphics.FillPath($bubbleBrush, $bubble)
    $dotBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 16, 88, 82))
    foreach ($x in @(213, 253, 293)) { $graphics.FillEllipse($dotBrush, $x, 264, 17, 17) }

    $spark = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $spark.AddBezier(377, 191, 384, 174, 385, 174, 392, 191)
    $spark.AddBezier(392, 191, 409, 198, 409, 200, 392, 207)
    $spark.AddBezier(392, 207, 385, 224, 384, 224, 377, 207)
    $spark.AddBezier(377, 207, 360, 200, 360, 198, 377, 191)
    $spark.CloseFigure()
    $sparkBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 255, 191, 92))
    $graphics.FillPath($sparkBrush, $spark)

    $graphics.ResetTransform()
    $graphics.Dispose()
    foreach ($item in @($berry, $berryBrush, $edgePen, $facet, $facetBrush, $leafDark, $leafLight, $veinPen, $bubble, $bubbleShadow, $bubbleBrush, $shadowMatrix, $dotBrush, $spark, $sparkBrush)) {
        $item.Dispose()
    }
    return $bitmap
}

function Get-PngBytes([int] $size) {
    $bitmap = New-BerryBitmap $size
    $stream = [System.IO.MemoryStream]::new()
    $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $stream.ToArray()
    $stream.Dispose()
    $bitmap.Dispose()
    return ,$bytes
}

function Save-Png([int] $size, [string] $name) {
    $bytes = Get-PngBytes $size
    [System.IO.File]::WriteAllBytes((Join-Path $assets $name), $bytes)
}

Save-Png 512 'favicon-source.png'
Save-Png 32 'favicon-32.png'
Save-Png 16 'favicon-16.png'
Save-Png 512 'favicon.png'
Save-Png 180 'apple-touch-icon.png'

$sizes = @(16, 32, 48, 64, 128, 256)
$frames = foreach ($size in $sizes) { Get-PngBytes $size }
$icoPath = Join-Path $assets 'favicon.ico'
$stream = [System.IO.File]::Create($icoPath)
$writer = [System.IO.BinaryWriter]::new($stream)
$writer.Write([uint16]0)
$writer.Write([uint16]1)
$writer.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $size = $sizes[$i]
    $bytes = $frames[$i]
    $dimension = if ($size -eq 256) { [byte]0 } else { [byte]$size }
    $writer.Write($dimension)
    $writer.Write($dimension)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]32)
    $writer.Write([uint32]$bytes.Length)
    $writer.Write([uint32]$offset)
    $offset += $bytes.Length
}
foreach ($frame in $frames) { $writer.Write($frame) }
$writer.Dispose()
$stream.Dispose()
