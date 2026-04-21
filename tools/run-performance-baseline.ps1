param(
    [int[]]$SizesMiB = @(1, 5),
    [int]$TimeoutSeconds = 20,
    [switch]$SkipBuild,
    [switch]$KeepGeneratedFiles
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$releaseExe = Join-Path $repoRoot "target\release\oxidemd.exe"
$outputDir = Join-Path $env:TEMP "oxidemd-performance"

function New-MarkdownFixture {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [int]$SizeMiB
    )

    $targetBytes = $SizeMiB * 1024 * 1024
    $encoding = [System.Text.UTF8Encoding]::new($false)
    $section = @'
# Large Document Section

This paragraph gives OxideMD enough ordinary prose to parse and render. It
includes **strong text**, *emphasis*, `inline code`, and a [link](https://example.com).

- First item
- Second item
- Third item

| Area | Status | Notes |
| --- | --- | --- |
| Parser | Active | Repeated table content |
| Renderer | Active | Repeated table content |

````rust
fn main() {
    println!("large document sample");
}
````

'@

    $writer = [System.IO.StreamWriter]::new($Path, $false, $encoding)
    try {
        while ($writer.BaseStream.Length -lt $targetBytes) {
            $writer.WriteLine($section)
        }
    }
    finally {
        $writer.Dispose()
    }
}

function Wait-ForPerfLine {
    param(
        [Parameter(Mandatory = $true)]
        [string]$LogPath,
        [Parameter(Mandatory = $true)]
        [string]$Pattern,
        [Parameter(Mandatory = $true)]
        [int]$PreviousCount,
        [Parameter(Mandatory = $true)]
        [int]$TimeoutSeconds
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path $LogPath) {
            $matches = Select-String -Path $LogPath -Pattern $Pattern
            if ($matches.Count -gt $PreviousCount) {
                return $matches[$matches.Count - 1].Line
            }
        }

        Start-Sleep -Milliseconds 250
    }

    throw "Timed out waiting for perf log pattern: $Pattern"
}

function Count-PerfLines {
    param(
        [Parameter(Mandatory = $true)]
        [string]$LogPath,
        [Parameter(Mandatory = $true)]
        [string]$Pattern
    )

    if (!(Test-Path $LogPath)) {
        return 0
    }

    return (Select-String -Path $LogPath -Pattern $Pattern).Count
}

function Append-ReloadChange {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $encoding = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::AppendAllText(
        $Path,
        "`n## Reload Check`nThis line checks reload timing.`n",
        $encoding
    )
}

if (!$SkipBuild) {
    Push-Location $repoRoot
    try {
        cargo build --release
    }
    finally {
        Pop-Location
    }
}

if (!(Test-Path $releaseExe)) {
    throw "Missing release executable: $releaseExe"
}

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

foreach ($sizeMiB in $SizesMiB) {
    $fixturePath = Join-Path $outputDir "oxidemd-large-${sizeMiB}mib.md"
    $stderrPath = Join-Path $outputDir "oxidemd-${sizeMiB}mib.stderr.log"
    $stdoutPath = Join-Path $outputDir "oxidemd-${sizeMiB}mib.stdout.log"

    Remove-Item -Force -ErrorAction SilentlyContinue $stderrPath, $stdoutPath
    New-MarkdownFixture -Path $fixturePath -SizeMiB $sizeMiB

    $actualSize = (Get-Item $fixturePath).Length
    Write-Host ""
    Write-Host "== $sizeMiB MiB target =="
    Write-Host "Fixture: $fixturePath"
    Write-Host ("Actual size: {0:N0} bytes" -f $actualSize)

    $process = Start-Process `
        -FilePath $releaseExe `
        -ArgumentList @($fixturePath) `
        -RedirectStandardError $stderrPath `
        -RedirectStandardOutput $stdoutPath `
        -PassThru

    try {
        $initialLoad = Wait-ForPerfLine `
            -LogPath $stderrPath `
            -Pattern "\[perf\] initial_load:" `
            -PreviousCount 0 `
            -TimeoutSeconds $TimeoutSeconds
        $renderAfterLoad = Wait-ForPerfLine `
            -LogPath $stderrPath `
            -Pattern "\[perf\] render_after_load:" `
            -PreviousCount 0 `
            -TimeoutSeconds $TimeoutSeconds

        $reloadCount = Count-PerfLines -LogPath $stderrPath -Pattern "\[perf\] reload:"
        $renderAfterReloadCount = Count-PerfLines -LogPath $stderrPath -Pattern "\[perf\] render_after_reload:"
        Append-ReloadChange -Path $fixturePath
        $reload = Wait-ForPerfLine `
            -LogPath $stderrPath `
            -Pattern "\[perf\] reload:" `
            -PreviousCount $reloadCount `
            -TimeoutSeconds $TimeoutSeconds
        $renderAfterReload = Wait-ForPerfLine `
            -LogPath $stderrPath `
            -Pattern "\[perf\] render_after_reload:" `
            -PreviousCount $renderAfterReloadCount `
            -TimeoutSeconds $TimeoutSeconds

        $skippedCount = Count-PerfLines -LogPath $stderrPath -Pattern "\[perf\] reload_skipped:"
        (Get-Item $fixturePath).LastWriteTime = Get-Date
        $skippedReload = Wait-ForPerfLine `
            -LogPath $stderrPath `
            -Pattern "\[perf\] reload_skipped:" `
            -PreviousCount $skippedCount `
            -TimeoutSeconds $TimeoutSeconds

        Write-Host $initialLoad
        Write-Host $renderAfterLoad
        Write-Host $reload
        Write-Host $renderAfterReload
        Write-Host $skippedReload
        Write-Host "Log: $stderrPath"
    }
    finally {
        if (!$process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }

    if (!$KeepGeneratedFiles) {
        Remove-Item -Force -ErrorAction SilentlyContinue $fixturePath
    }
}
