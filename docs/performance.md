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

- Date: 2026-04-21
- Build: release
- Command: `.\tools\run-performance-baseline.ps1 -SkipBuild`
- Size: 1.00 MiB / 1,048,771 bytes
- Startup: not captured by the helper script
- Initial load: 17 ms total, 17 ms parse
- First render after load: 154 ms, 11945 blocks, 2389 headings
- Reload after edit: 17 ms total, 17 ms parse
- First render after reload: 112 ms, 11947 blocks, 2390 headings
- Skipped reload: 0 ms total
- Notes: Parsing is comfortably fast at this size. Rendering is already notably more expensive than parsing.

### 5 MiB Markdown

- Date: 2026-04-21
- Build: release
- Command: `.\tools\run-performance-baseline.ps1 -SkipBuild`
- Size: 5.00 MiB / 5,242,977 bytes
- Startup: not captured by the helper script
- Initial load: 91 ms total, 89 ms parse
- First render after load: 818 ms, 59715 blocks, 11943 headings
- Reload after edit: 92 ms total, 90 ms parse
- First render after reload: 641 ms, 59717 blocks, 11944 headings
- Skipped reload: 1 ms total
- Notes: Full parse remains under 100 ms, so immediate parser replacement is not justified by this baseline alone.

### First Measured Bottleneck

- Area: First render after load and reload.
- Evidence: On the 5 MiB baseline, parse takes about 90 ms while first render takes 641-818 ms.
- Next action: Reduce first-render cost for large documents before changing the parser or adding large-file dependencies.

## Optimization Notes

### 2026-04-21: Avoid Empty Search Highlight Work

Change:

- Pass no search query to the renderer when the search input is empty.
- Avoid highlight segment allocation when no search query is active.
- Avoid inline line-splitting allocation when inline content has no line breaks.

Result:

- 1 MiB first render after load: 154 ms -> 147 ms
- 1 MiB first render after reload: 112 ms -> 112 ms
- 5 MiB first render after load: 818 ms -> 785 ms
- 5 MiB first render after reload: 641 ms -> 620 ms

Conclusion:

- This removes avoidable work, but first render is still dominated by rendering a very large number of blocks.

### 2026-04-21: Virtualize TOC Rendering

Change:

- Render the heading navigation with `ScrollArea::show_rows`.
- Only visible TOC rows are built each frame.
- Keep full heading titles available through hover text while truncating long rows.

Result:

- 1 MiB first render after load: 147 ms -> 154 ms
- 1 MiB first render after reload: 112 ms -> 111 ms
- 5 MiB first render after load: 785 ms -> 768 ms
- 5 MiB first render after reload: 620 ms -> 604 ms

Conclusion:

- TOC virtualization helps a little on the 5 MiB benchmark, but the main remaining cost is still document body rendering.
