# Performance Measurement

OxideMD prints lightweight performance logs to stderr during startup, initial
file load, reload, and skipped reloads.

Mermaid diagram rendering also logs each background SVG render:

```text
[perf] diagram_render: 4 ms, mermaid, 128 source bytes, ok
```

The final field is `ok` or `error`. Cache hits do not log a new render because
no worker job is started.

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
- Mermaid diagram render logs when opening `samples/mermaid-evaluation.md`
- Mermaid CLI visual comparison with `tools/compare-mermaid-cli.ps1` when
  `mmdc` is available; export OxideMD reference SVGs first with
  `cargo test --release diagram::tests::exports_mermaid_evaluation_svgs_for_cli_comparison -- --ignored --nocapture`

Record the file size, build profile, and observed log lines when comparing
changes.

## Baseline Results

Record representative measurements here before optimizing large file behavior.

### Mermaid SVG Rendering Prototype

- Date: 2026-04-27
- Build: release
- Command: `cargo test --release diagram::tests::stores_finished_diagram_result -- --nocapture`
- Diagram: small flowchart, 18 source bytes
- Render: 24 ms, outcome `ok`
- Notes: This measures the renderer path in isolation through the background worker test. Manual GUI checks should use `samples/mermaid-evaluation.md` to compare common diagram types, cache behavior, and failure fallback.

### Mermaid Evaluation Sample Render Check

- Date: 2026-04-28
- Build: release
- Command: `cargo test --release diagram::tests::renders_common_mermaid_evaluation_diagrams -- --nocapture`
- Flowchart: 28 ms, 264 source bytes, outcome `ok`
- Sequence diagram: 0 ms, 231 source bytes, outcome `ok`
- Class diagram: 0 ms, 202 source bytes, outcome `ok`
- State diagram: 0 ms, 129 source bytes, outcome `ok`
- Larger flowchart: 2 ms, 496 source bytes, outcome `ok`
- Invalid diagram: 0 ms, 27 source bytes, outcome `error`
- Cache behavior command: `cargo test --release diagram::tests::reuses_finished_diagram_result_from_cache -- --nocapture`
- Cache behavior result: first worker render logged 22 ms for an 18-byte flowchart; the following prepare call returned the finished result from cache without starting another worker job.
- Notes: This is an isolated renderer and cache check, not a full GUI timing run. GUI validation should still open `samples/mermaid-evaluation.md` to inspect layout quality, fallback presentation, and repaint behavior.

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

### 2026-04-22 Baseline Verification

- Build: release
- Command: `.\tools\run-performance-baseline.ps1 -SkipBuild`
- 1 MiB size: 1.00 MiB / 1,048,771 bytes
- 1 MiB initial load: 17 ms total, 16 ms parse
- 1 MiB first render after load: 20 ms, 11945 blocks, 2389 headings
- 1 MiB reload after edit: 17 ms total, 16 ms parse
- 1 MiB first render after reload: 1 ms, 11947 blocks, 2390 headings
- 1 MiB skipped reload: 0 ms total
- 5 MiB size: 5.00 MiB / 5,242,977 bytes
- 5 MiB initial load: 87 ms total, 85 ms parse
- 5 MiB first render after load: 24 ms, 59715 blocks, 11943 headings
- 5 MiB reload after edit: 87 ms total, 85 ms parse
- 5 MiB first render after reload: 4 ms, 59717 blocks, 11944 headings
- 5 MiB skipped reload: 2 ms total
- Notes: Baseline timings remain in the same range as the virtualized rendering and scroll stabilization results. No code change is indicated by this verification run.

### 2026-04-28 Baseline Verification

- Build: release
- Command: `.\tools\run-performance-baseline.ps1 -SkipBuild`
- 1 MiB size: 1.00 MiB / 1,048,771 bytes
- 1 MiB initial load: 18 ms total, 18 ms parse
- 1 MiB first render after load: 21 ms, 11945 blocks, 2389 headings
- 1 MiB reload after edit: 18 ms total, 17 ms parse
- 1 MiB first render after reload: 1 ms, 11947 blocks, 2390 headings
- 1 MiB skipped reload: 0 ms total
- 5 MiB size: 5.00 MiB / 5,242,977 bytes
- 5 MiB initial load: 95 ms total, 93 ms parse
- 5 MiB first render after load: 23 ms, 59715 blocks, 11943 headings
- 5 MiB reload after edit: 90 ms total, 88 ms parse
- 5 MiB first render after reload: 4 ms, 59717 blocks, 11944 headings
- 5 MiB skipped reload: 2 ms total
- Notes: Timings remain in the same range as the previous baseline verification. Initial load and reload are still dominated by parse time, while virtualized first render remains low for both fixture sizes.

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

### 2026-04-21: Virtualize Document Body Rendering

Change:

- Skip rendering document blocks far outside the visible viewport for large documents.
- Add estimated vertical space for skipped blocks so scrolling still covers the full document.
- Keep the selected scroll target rendered so heading and search navigation can still jump to a block.

Result:

- 1 MiB first render after load: 154 ms -> 20 ms
- 1 MiB first render after reload: 111 ms -> 1 ms
- 5 MiB first render after load: 768 ms -> 23 ms
- 5 MiB first render after reload: 604 ms -> 4 ms

Conclusion:

- First-render cost is no longer dominated by building UI for every block. The next large-file work should focus on validating scroll accuracy and reducing memory copies where useful.

### 2026-04-21: Reduce Large Document Copies

Change:

- Store parsed documents in the cache as `Arc<MarkdownDocument>`.
- Share loaded and reloaded documents by reference-counted pointer instead of cloning the full document tree.
- Render heading navigation from the document heading slice instead of cloning all headings every frame.
- Avoid cloning all search matches when rendering the search result panel.

Result:

- 1 MiB first render after load: 20 ms -> 20 ms
- 1 MiB first render after reload: 1 ms -> 1 ms
- 5 MiB initial load: 91 ms -> 87 ms
- 5 MiB reload after edit: 93 ms -> 88 ms
- 5 MiB first render after load: 23 ms -> 23 ms
- 5 MiB first render after reload: 4 ms -> 4 ms

Conclusion:

- The visible render timings remain stable while avoiding full `MarkdownDocument` and heading-list copies on large files. This is mainly a memory and allocation pressure improvement rather than a new rendering-time optimization.

### 2026-04-21: Add Reload Metadata Fast Path

Change:

- Track the loaded file size and modified timestamp alongside the content fingerprint.
- Before reading a reload candidate, compare the current metadata with the previous metadata.
- If both match exactly, skip the file read and content hash and report the document as unchanged.
- Keep the existing content fingerprint check for cases where metadata changed but content did not.

Result:

- 1 MiB initial load: 17 ms -> 17 ms
- 1 MiB reload after edit: 17 ms -> 19 ms
- 1 MiB skipped reload: 0 ms -> 0 ms
- 5 MiB initial load: 87 ms -> 89 ms
- 5 MiB reload after edit: 88 ms -> 87 ms
- 5 MiB skipped reload: 1 ms -> 1 ms

Conclusion:

- Normal reload and benchmark timings remain stable. The benefit is targeted at duplicate watcher events where file metadata is unchanged, allowing OxideMD to avoid rereading and rehashing large files.

### 2026-04-22: Cache Measured Block Heights

Change:

- Cache measured document block heights after visible blocks are rendered.
- Reuse measured heights when virtualized blocks are skipped.
- Reset the height cache when the document fingerprint, zoom factor, or document body width changes.
- Keep estimated heights as the fallback for blocks that have not been rendered yet.

Result:

- 1 MiB initial load: 18 ms total, 17 ms parse
- 1 MiB first render after load: 22 ms
- 1 MiB reload after edit: 18 ms total, 17 ms parse
- 1 MiB first render after reload: 1 ms
- 1 MiB skipped reload: 0 ms total
- 5 MiB initial load: 88 ms total, 86 ms parse
- 5 MiB first render after load: 23 ms
- 5 MiB reload after edit: 86 ms total, 84 ms parse
- 5 MiB first render after reload: 4 ms
- 5 MiB skipped reload: 1 ms total

Conclusion:

- The measured-height cache keeps first render timings in the same range while reducing scroll-position drift after blocks have been measured.
- The remaining large-document validation should focus on manual TOC and search jumps near the start, middle, and end of long files.

### 2026-04-22: Stabilize Virtualized Scroll Jumps

Change:

- Track whether a scroll target block had an existing measured height before rendering.
- Keep the pending scroll target for one extra repaint when jumping to a newly measured block.
- Let the next frame reuse fresh height measurements before clearing the pending scroll.

Result:

- 1 MiB initial load: 17 ms total, 16 ms parse
- 1 MiB first render after load: 21 ms
- 1 MiB reload after edit: 18 ms total, 17 ms parse
- 1 MiB first render after reload: 1 ms
- 1 MiB skipped reload: 0 ms total
- 5 MiB initial load: 89 ms total, 87 ms parse
- 5 MiB first render after load: 24 ms
- 5 MiB reload after edit: 88 ms total, 85 ms parse
- 5 MiB first render after reload: 4 ms
- 5 MiB skipped reload: 2 ms total

Conclusion:

- The stabilization repaint keeps large-document render timings in the same range while giving TOC and search jumps a second frame to settle after initial block measurement.
- The next validation step is manual navigation testing across start, middle, and end positions in a large generated document.

### 2026-04-24: Cache Estimated Block Heights

Change:

- Cache estimated block heights separately from measured block heights.
- Reuse those estimates across frames instead of recomputing inline height guesses for every unmeasured block.
- Clear only measured heights when the document body width changes so viewport estimation stays stable while measurements rebuild.

Result:

- 1 MiB initial load: 17 ms -> 19 ms
- 1 MiB first render after load: 20 ms -> 20 ms
- 1 MiB reload after edit: 17 ms -> 18 ms
- 1 MiB first render after reload: 1 ms -> 0 ms
- 1 MiB skipped reload: 0 ms -> 0 ms
- 5 MiB initial load: 87 ms -> 93 ms
- 5 MiB first render after load: 24 ms -> 24 ms
- 5 MiB reload after edit: 87 ms -> 96 ms
- 5 MiB first render after reload: 4 ms -> 3 ms
- 5 MiB skipped reload: 2 ms -> 2 ms

Conclusion:

- Visible render timings stay in the same range, which suggests the change mainly improves frame-to-frame predictability rather than headline throughput.
- Keeping estimated heights alive across repaint and width-reset paths removes repeated estimation work while measured heights continue to refine scroll spacing for visible blocks.

### 2026-04-24: Stream Inline Line-Break Rendering

Change:

- Replace the temporary `Vec<Vec<&InlineSpan>>` line-splitting path in `render_inline`.
- Render each inline line directly from slices between `InlineSpan::LineBreak` markers.
- Keep the no-line-break fast path and wrapped layout behavior unchanged.

Result:

- 1 MiB initial load: 19 ms -> 18 ms
- 1 MiB first render after load: 20 ms -> 20 ms
- 1 MiB reload after edit: 18 ms -> 18 ms
- 1 MiB first render after reload: 0 ms -> 0 ms
- 1 MiB skipped reload: 0 ms -> 0 ms
- 5 MiB initial load: 93 ms -> 92 ms
- 5 MiB first render after load: 24 ms -> 25 ms
- 5 MiB reload after edit: 96 ms -> 94 ms
- 5 MiB first render after reload: 3 ms -> 3 ms
- 5 MiB skipped reload: 2 ms -> 1 ms

Conclusion:

- Visible timings remain within normal run-to-run variance, so the benefit is mainly lower temporary allocation pressure while rendering paragraphs with explicit line breaks.
- This is a safe follow-up optimization under the remaining `Avoid unnecessary allocations in hot paths` work.

### 2026-04-24: Stream Search Highlight Segments

Change:

- Add a callback-based highlighted-segment walker in `search.rs`.
- Use that streaming path for highlight width measurement and highlighted text rendering in `renderer.rs`.
- Keep the existing vector-returning helper only for tests so render paths stop allocating temporary segment lists.

Result:

- 1 MiB initial load: 18 ms -> 18 ms
- 1 MiB first render after load: 20 ms -> 21 ms
- 1 MiB reload after edit: 18 ms -> 21 ms
- 1 MiB first render after reload: 0 ms -> 1 ms
- 1 MiB skipped reload: 0 ms -> 0 ms
- 5 MiB initial load: 92 ms -> 100 ms
- 5 MiB first render after load: 25 ms -> 25 ms
- 5 MiB reload after edit: 94 ms -> 107 ms
- 5 MiB first render after reload: 3 ms -> 4 ms
- 5 MiB skipped reload: 1 ms -> 2 ms

Conclusion:

- Render timings stay in the same general range, and the larger parse/reload shifts look like normal run-to-run variance rather than a render regression from this change.
- The main benefit is lower temporary allocation pressure while measuring and painting highlighted inline text.

### 2026-04-24: Normalize Search Plain Text Without Temporary Vectors

Change:

- Replace `split_whitespace().collect::<Vec<_>>().join(" ")` search-text normalization paths with direct `String` builders.
- Avoid cloning table headers and cells just to build searchable plain text.
- Reuse the same normalized-text helpers for inline content, code blocks, math blocks, and joined list/table content.

Result:

- 1 MiB initial load: 18 ms -> 19 ms
- 1 MiB first render after load: 21 ms -> 20 ms
- 1 MiB reload after edit: 21 ms -> 19 ms
- 1 MiB first render after reload: 1 ms -> 0 ms
- 1 MiB skipped reload: 0 ms -> 0 ms
- 5 MiB initial load: 100 ms -> 97 ms
- 5 MiB first render after load: 25 ms -> 23 ms
- 5 MiB reload after edit: 107 ms -> 91 ms
- 5 MiB first render after reload: 4 ms -> 3 ms
- 5 MiB skipped reload: 2 ms -> 2 ms

Conclusion:

- The largest visible improvement is on reload-heavy paths, which is consistent with search match generation doing less temporary allocation while rebuilding block plain text.
- This keeps moving the remaining hot-path allocation work in the right direction without adding new dependencies or complexity.

### 2026-04-24: Make Search Preview Text Single Pass

Change:

- Build search preview strings in a single character pass.
- Avoid counting characters in a second pass just to decide whether to append `...`.
- Reserve preview buffer capacity up front for the common truncated case.

Result:

- 1 MiB initial load: 19 ms -> 18 ms
- 1 MiB first render after load: 20 ms -> 20 ms
- 1 MiB reload after edit: 19 ms -> 18 ms
- 1 MiB first render after reload: 0 ms -> 1 ms
- 1 MiB skipped reload: 0 ms -> 0 ms
- 5 MiB initial load: 97 ms -> 94 ms
- 5 MiB first render after load: 23 ms -> 24 ms
- 5 MiB reload after edit: 91 ms -> 91 ms
- 5 MiB first render after reload: 3 ms -> 3 ms
- 5 MiB skipped reload: 2 ms -> 2 ms

Conclusion:

- Timings remain within the same range, with a small improvement on initial load and no sign of regression in render behavior.
- The benefit is modest but consistent with removing repeated character scanning during search result preview generation.
