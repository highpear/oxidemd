# OxideMD Long Form Sample

This sample exists for manual testing of longer reading flows in OxideMD.
It is intentionally structured to exercise heading navigation, scrolling, zoom, theme changes, lists, quotes, links, and code blocks.

## Reading Goals

OxideMD is designed as a reading-first Markdown viewer.
That means the document should remain comfortable to scan over time, even when it is longer than a quick note or short README.

When you open this file, try a few things:

- Scroll from top to bottom
- Change the zoom level
- Switch themes
- Use heading navigation
- Edit and save the file while live reload is active

### Why This Matters

A short sample can confirm whether parsing works.
A longer sample is better for evaluating whether the interface still feels calm and readable after a few minutes of use.

### Secondary Checks

This file also helps verify:

1. Whether spacing between sections remains consistent
2. Whether headings form a clear visual hierarchy
3. Whether long paragraphs wrap cleanly
4. Whether code blocks still feel distinct at different zoom levels

## Document Shape

This section is mostly ordinary prose so that scrolling behavior is easy to observe.
The text does not need to say anything remarkable.
Its main purpose is to create a reading rhythm that feels closer to a real article than a synthetic parser test.

OxideMD does not aim to become a full editor.
Because of that, the viewing experience has to carry more weight.
Even small things such as line length, section spacing, and navigation affordances start to matter once the document is long enough.

### A Mid-Level Section

This section adds more length and another point in the heading tree.
Try clicking here from the navigation panel after scrolling further down.

Lorem ipsum style filler is easy to generate, but it is not always ideal for testing.
Natural language paragraphs are usually better because they expose visual pacing issues more clearly.
Repeated sentence lengths and realistic punctuation often reveal awkward spacing faster than placeholder text.

#### Deep Section

At this depth, indentation in the navigation panel should still be understandable.
If the left panel becomes noisy with many levels, that is a hint that future refinements may be useful.

## Mixed Content

Below is a mix of lists, quotes, links, and code.
The goal is not semantic depth.
The goal is to give the renderer several transitions between block types.

### Unordered Notes

- The first item is intentionally short.
- The second item is a little longer so wrapping can be checked when the window width is reduced.
- The third item mentions [the Rust website](https://www.rust-lang.org/) as a clickable link target.

### Ordered Steps

1. Open this sample in OxideMD.
2. Collapse the app width until paragraph wrapping changes.
3. Increase zoom and confirm the document remains readable.
4. Use the heading navigation panel to jump back to earlier sections.

### Block Quote

> Good reading software should disappear behind the document.
> The interface still matters, but it should support attention rather than compete for it.

### Code Block

```rust
fn summarize_sections(headings: &[&str]) -> String {
    headings.join(" -> ")
}

fn main() {
    let headings = ["Intro", "Mixed Content", "Notes"];
    println!("{}", summarize_sections(&headings));
}
```

## Longer Body Text

This section exists mostly to create more scrolling distance.
The paragraphs are intentionally moderate in length so they read like ordinary documentation rather than stress-test gibberish.

A viewer often feels acceptable with short content.
The real differences appear when the file becomes long enough that people need orientation, not just rendering.
That is where a table of contents, heading navigation, search, zoom, and stable spacing all begin to work together.

Performance concerns also become easier to notice with longer content.
Even when a file is not huge, users can feel jitter, unnecessary redraws, or delayed reload behavior surprisingly quickly.
That is why this file is useful not only for UI polish, but also for observing practical responsiveness.

### Another Subsection

If you are testing live reload, try editing a few sentences in this area and saving the file multiple times.
Watch whether the reload status updates clearly and whether the document position remains comfortable.

### Yet Another Subsection

One future improvement could be storing reading position when the same file reloads.
That is outside the current scope, but a longer sample makes the need more visible.

## Japanese Notes

このセクションは、日本語を少し含む長めの確認用です。
Meiryo フォント設定や折り返し、ズーム時の見え方を確認しやすくするために入れています。

Markdown ビューアでは、単に表示できるだけでなく、読み続けても疲れにくいことが大切です。
見出しの階層、余白、文字サイズ、配色のバランスが崩れると、長文ではすぐに気になります。

### 日本語の小見出し

ここからナビゲーションで移動したときに、見出し位置が自然に見えるかも確認できます。

## Final Notes

If this file becomes too predictable over time, it can be expanded with more sections, tables once supported, and additional code examples.
For now, it should be long enough to make the current viewer features easier to evaluate.
