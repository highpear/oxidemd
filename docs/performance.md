# Performance Measurement

OxideMD prints lightweight performance logs to stderr during startup, initial
file load, reload, and skipped reloads.

Use these logs before making performance changes. Prefer measuring with a
release build because debug builds include extra overhead.

## Build

```powershell
cargo build --release
```

## Automated Baseline Run

Use the helper script to generate temporary 1 MiB and 5 MiB Markdown files,
start OxideMD, capture perf logs, trigger one content reload, trigger one
unchanged reload, and print the collected log lines:

```powershell
.\tools\run-performance-baseline.ps1
```

Generated files and logs are written under:

```powershell
$env:TEMP\oxidemd-performance
```

The generated Markdown files are deleted by default. Keep them for inspection
with:

```powershell
.\tools\run-performance-baseline.ps1 -KeepGeneratedFiles
```

## Measure a Large Markdown File

Use an existing large document when possible. If you need a temporary test file,
generate one outside the repository:

```powershell
$section = @"
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

"@

1..2000 | ForEach-Object { $section } | Set-Content -Encoding UTF8 $env:TEMP\oxidemd-large.md
```

Then open it from the command line:

```powershell
.\target\release\oxidemd.exe $env:TEMP\oxidemd-large.md
```

Example log shape:

```text
[perf] startup: 120 ms
[perf] initial_load: 84 ms total, 52 ms parse, 1.15 MiB (C:\Users\...\oxidemd-large.md)
```

## Manual Checks

- Initial load time
- Reload time after saving the file
- Skipped reload time when the saved content is unchanged
- UI responsiveness while scrolling
- Search responsiveness on common terms
- TOC usability with many headings

Record the file size, build profile, and observed log lines when comparing
changes.

## Baseline Results

Record representative measurements here before optimizing large file behavior.

### 1 MiB Markdown

- Date:
- Build:
- Command:
- Size:
- Startup:
- Initial load:
- Reload after edit:
- Skipped reload:
- Notes:

### 5 MiB Markdown

- Date:
- Build:
- Command:
- Size:
- Startup:
- Initial load:
- Reload after edit:
- Skipped reload:
- Notes:

### First Measured Bottleneck

- Area:
- Evidence:
- Next action:
