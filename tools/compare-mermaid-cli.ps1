param(
    [string]$SamplePath = "samples/mermaid-evaluation.md",
    [string]$OutputPath = "$env:TEMP\oxidemd-mermaid-cli-comparison",
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
$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Mermaid CLI Comparison Report")
$report.Add("")
$report.Add("- Sample: $resolvedSamplePath")
$report.Add("- Mermaid CLI: $mmdcPath")
$report.Add("- Output: $resolvedOutputPath")
$report.Add("")
$report.Add("| Diagram | Source | Mermaid CLI SVG | CLI Result | Manual Notes |")
$report.Add("| --- | --- | --- | --- | --- |")

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

    $report.Add("| $($block.Title) | ``$sourceName`` | ``$svgName`` | $status |  |")
}

Set-Content -LiteralPath $reportPath -Value $report -Encoding UTF8

Write-Host "Wrote Mermaid CLI comparison files to $resolvedOutputPath"
Write-Host "Report: $reportPath"
