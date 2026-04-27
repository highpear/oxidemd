param(
    [string]$SamplePath = "samples/mermaid-evaluation.md",
    [string]$OutputPath = "$env:TEMP\oxidemd-mermaid-cli-comparison",
    [string]$NativeOutputPath = "$env:TEMP\oxidemd-mermaid-native-comparison",
    [string]$MermaidCliCommand = "mmdc"
)

$ErrorActionPreference = "Stop"

function Resolve-MermaidCli {
    param([string]$CommandName)

    $command = Get-Command $CommandName -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $fnmRoot = Join-Path $env:APPDATA "fnm\node-versions"
    if (Test-Path -LiteralPath $fnmRoot) {
        $fnmCommand = Get-ChildItem -LiteralPath $fnmRoot -Recurse -Filter "mmdc.cmd" -ErrorAction SilentlyContinue |
            Sort-Object -Property FullName -Descending |
            Select-Object -First 1

        if ($fnmCommand) {
            return $fnmCommand.FullName
        }
    }

    return $null
}

$resolvedSamplePath = Resolve-Path -Path $SamplePath
$resolvedOutputPath = [System.IO.Path]::GetFullPath($OutputPath)
$resolvedNativeOutputPath = [System.IO.Path]::GetFullPath($NativeOutputPath)
$resolvedTempPath = [System.IO.Path]::GetFullPath($env:TEMP)

if ([string]::IsNullOrWhiteSpace($resolvedOutputPath) -or $resolvedOutputPath.Length -le 3) {
    throw "Refusing to use an unsafe output path: $resolvedOutputPath"
}

if (-not $resolvedOutputPath.StartsWith($resolvedTempPath, [System.StringComparison]::OrdinalIgnoreCase)) {
    Write-Warning "Output path is outside TEMP: $resolvedOutputPath"
}

if (Test-Path -LiteralPath $resolvedOutputPath) {
    Remove-Item -LiteralPath $resolvedOutputPath -Recurse -Force
}

New-Item -ItemType Directory -Path $resolvedOutputPath | Out-Null

$blocks = New-Object System.Collections.Generic.List[object]
$currentTitle = $null
$currentLines = $null
$inMermaidBlock = $false

foreach ($line in Get-Content -LiteralPath $resolvedSamplePath) {
    if ($line.StartsWith("## ")) {
        $currentTitle = $line.Substring(3).Trim()
        continue
    }

    if ($line.Trim() -eq '```mermaid') {
        $inMermaidBlock = $true
        $currentLines = New-Object System.Collections.Generic.List[string]
        continue
    }

    if ($inMermaidBlock -and $line.Trim() -eq '```') {
        $title = $currentTitle
        if (-not $title) {
            $title = "Diagram $($blocks.Count + 1)"
        }

        $blocks.Add([pscustomobject]@{
            Title = $title
            Source = ($currentLines -join "`n")
        })
        $inMermaidBlock = $false
        $currentLines = $null
        continue
    }

    if ($inMermaidBlock) {
        $currentLines.Add($line)
    }
}

if ($blocks.Count -eq 0) {
    throw "No Mermaid blocks found in $resolvedSamplePath"
}

$mmdcPath = Resolve-MermaidCli -CommandName $MermaidCliCommand
if (-not $mmdcPath) {
    Write-Error "Mermaid CLI command '$MermaidCliCommand' was not found. Install Mermaid CLI, or pass -MermaidCliCommand with the path to mmdc."
}

$reportPath = Join-Path $resolvedOutputPath "comparison-report.md"
$htmlReportPath = Join-Path $resolvedOutputPath "visual-comparison.html"
$report = New-Object System.Collections.Generic.List[string]
$html = New-Object System.Collections.Generic.List[string]
$report.Add("# Mermaid CLI Comparison Report")
$report.Add("")
$report.Add("- Sample: $resolvedSamplePath")
$report.Add("- Mermaid CLI: $mmdcPath")
$report.Add("- Output: $resolvedOutputPath")
$report.Add("- Native SVG output: $resolvedNativeOutputPath")
$report.Add("")
$report.Add("| Diagram | Source | OxideMD SVG | Mermaid CLI SVG | CLI Result | Manual Notes |")
$report.Add("| --- | --- | --- | --- | --- | --- |")

$html.Add("<!doctype html>")
$html.Add("<html lang=""en"">")
$html.Add("<head>")
$html.Add("<meta charset=""utf-8"">")
$html.Add("<title>OxideMD Mermaid CLI Comparison</title>")
$html.Add("<style>")
$html.Add("body { font-family: system-ui, sans-serif; margin: 24px; color: #20242a; }")
$html.Add("section { margin: 0 0 32px; }")
$html.Add(".pair { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; align-items: start; }")
$html.Add(".panel { border: 1px solid #d4d8df; border-radius: 6px; padding: 12px; overflow: auto; }")
$html.Add(".panel h3 { font-size: 14px; margin: 0 0 8px; }")
$html.Add("img { max-width: 100%; height: auto; background: white; }")
$html.Add("code { font-family: ui-monospace, Consolas, monospace; }")
$html.Add("</style>")
$html.Add("</head>")
$html.Add("<body>")
$html.Add("<h1>OxideMD Mermaid CLI Comparison</h1>")
$html.Add("<p>Left: OxideMD native SVG output. Right: Mermaid CLI SVG output.</p>")

for ($index = 0; $index -lt $blocks.Count; $index++) {
    $block = $blocks[$index]
    $safeName = $block.Title.ToLowerInvariant() -replace "[^a-z0-9]+", "-"
    $safeName = $safeName.Trim("-")
    if ([string]::IsNullOrWhiteSpace($safeName)) {
        $safeName = "diagram-$($index + 1)"
    }

    $sourcePath = Join-Path $resolvedOutputPath ("{0:D2}-{1}.mmd" -f ($index + 1), $safeName)
    $svgPath = Join-Path $resolvedOutputPath ("{0:D2}-{1}.svg" -f ($index + 1), $safeName)
    Set-Content -LiteralPath $sourcePath -Value $block.Source -Encoding UTF8

    $status = "ok"
    try {
        $global:LASTEXITCODE = 0
        & $mmdcPath -i $sourcePath -o $svgPath | Out-Null
        if ($LASTEXITCODE -ne 0) {
            $status = "error: exit code $LASTEXITCODE"
        }
    } catch {
        $status = "error: $($_.Exception.Message)"
    }

    $sourceName = Split-Path -Leaf $sourcePath
    $svgName = Split-Path -Leaf $svgPath
    if (-not (Test-Path -LiteralPath $svgPath)) {
        $svgName = "-"
    }
    $nativeName = "{0:D2}-{1}.svg" -f ($index + 1), $safeName
    $nativePath = Join-Path $resolvedNativeOutputPath $nativeName
    if (-not (Test-Path -LiteralPath $nativePath)) {
        $nativeName = "-"
    }

    $report.Add("| $($block.Title) | ``$sourceName`` | ``$nativeName`` | ``$svgName`` | $status |  |")

    $html.Add("<section>")
    $html.Add("<h2>$($block.Title)</h2>")
    $html.Add("<p>Source: <code>$sourceName</code>; CLI result: <code>$status</code></p>")
    $html.Add("<div class=""pair"">")
    $html.Add("<div class=""panel""><h3>OxideMD</h3>")
    if ($nativeName -eq "-") {
        $html.Add("<p>No OxideMD SVG found.</p>")
    } else {
        $nativeRelativePath = [System.IO.Path]::GetRelativePath($resolvedOutputPath, $nativePath).Replace("\", "/")
        $html.Add("<img src=""$nativeRelativePath"" alt=""OxideMD $($block.Title)"">")
    }
    $html.Add("</div>")
    $html.Add("<div class=""panel""><h3>Mermaid CLI</h3>")
    if ($svgName -eq "-") {
        $html.Add("<p>No Mermaid CLI SVG generated.</p>")
    } else {
        $html.Add("<img src=""$svgName"" alt=""Mermaid CLI $($block.Title)"">")
    }
    $html.Add("</div>")
    $html.Add("</div>")
    $html.Add("</section>")
}

$html.Add("</body>")
$html.Add("</html>")

Set-Content -LiteralPath $reportPath -Value $report -Encoding UTF8
Set-Content -LiteralPath $htmlReportPath -Value $html -Encoding UTF8

Write-Host "Wrote Mermaid CLI comparison files to $resolvedOutputPath"
Write-Host "Report: $reportPath"
Write-Host "Visual report: $htmlReportPath"
