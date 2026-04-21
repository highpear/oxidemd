# Performance Measurement

OxideMD prints lightweight performance logs to stderr during startup, initial
file load, reload, and skipped reloads.

Use these logs before making performance changes. Prefer measuring with a
release build because debug builds include extra overhead.

## Build

```powershell
cargo build --release
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
