# TODO.md

# OxideMD Development Roadmap

OxideMD is a fast and minimal Markdown viewer built with Rust.

This file tracks the implementation roadmap in small, incremental steps.
The project should remain buildable at every stage.

---

## Phase 0 - Project Setup

* [x] Create the Cargo project
* [x] Decide the project name: OxideMD
* [x] Create `AGENTS.md`
* [x] Create the first runnable native window with `egui` / `eframe`
* [x] Add basic project metadata to `Cargo.toml`

---

## Phase 1 - Minimal GUI Application

Goal: launch a native window and establish the smallest useful application structure.

* [x] Add `egui` / `eframe`
* [x] Split the code into:
  * [x] `src/main.rs`
  * [x] `src/app.rs`
* [x] Show the app title in the UI
* [x] Show a simple placeholder message
* [x] Confirm the project builds and runs successfully

Exit criteria:

* [x] The application opens a native window
* [x] The UI renders correctly
* [x] The code is clean and easy to extend

---

## Phase 1.5 - Internationalization

- [x] Add i18n module
- [x] Replace hardcoded UI strings
- [x] Add language switch (basic)
- [ ] Evaluate whether the static i18n table should stay or move to external resources later

---

## Phase 2 - Load a Markdown File

Goal: load a Markdown file and display its contents in the app.

* [x] Define how the first file is selected
  * [x] File dialog
  * [x] Command-line argument
  * [x] Drag and drop
* [x] Read a `.md` file from disk
* [x] Handle file read errors gracefully
* [x] Display the loaded file contents in the app
* [x] Keep the UI responsive while loading

Exit criteria:

* [x] A Markdown file can be opened
* [x] Its contents are visible in the app
* [x] Errors are shown clearly

---

## Phase 3 - Basic Markdown Parsing

Goal: parse Markdown and render core elements in a structured way.

* [x] Add `pulldown-cmark`
* [x] Create `src/parser.rs`
* [x] Parse headings
* [x] Parse paragraphs
* [x] Parse unordered lists
* [x] Parse ordered lists
* [x] Parse blockquotes
* [x] Parse fenced code blocks
* [x] Parse tables
* [x] Define a simple intermediate representation for rendered content

Exit criteria:

* [x] Markdown is not shown as raw text anymore
* [x] Core block elements render correctly
* [x] Parsing and rendering are separated

---

## Phase 4 - Basic Markdown Rendering

Goal: render parsed Markdown with a simple but readable UI.

* [x] Create `src/renderer.rs`
* [x] Render headings with visual hierarchy
* [x] Render paragraphs with readable spacing
* [x] Render lists cleanly
* [x] Render blockquotes distinctly
* [x] Render code blocks in a monospaced style
* [x] Render tables
* [x] Add scrollable document view
* [x] Render basic inline Markdown
* [x] Make rendered links clickable

Exit criteria:

* [x] Common Markdown documents are readable
* [x] The viewer feels usable as a real application
* [x] Rendering code remains simple and maintainable

---

## Phase 5 - File Watching / Live Reload

Goal: update the view automatically when the source file changes.

* [x] Add `notify`
* [x] Watch the currently opened file
* [x] Re-read the file when it changes
* [x] Update the UI safely after reload
* [x] Avoid duplicate or excessive reload events
* [x] Add reload status feedback
* [x] Re-parse content in the background

Exit criteria:

* [x] Editing the Markdown file updates the view automatically
* [x] The UI remains responsive during reload
* [x] File watching works reliably on the target platform

---

## Phase 6 - Performance Foundation

Goal: prepare the app for large files and frequent updates.

* [x] Measure startup time
* [x] Measure reload time
* [x] Measure parse time
* [x] Cache parsed output
* [x] Avoid unnecessary re-parsing
* [ ] Avoid unnecessary allocations in hot paths
* [ ] Keep rendering work predictable

Exit criteria:

* [x] Basic performance metrics are available
* [ ] Major bottlenecks are visible
* [ ] The codebase is ready for focused optimization

---

## Phase 7 - Syntax Highlighting

Goal: improve readability of fenced code blocks.

* [x] Add `syntect`
* [x] Highlight fenced code blocks
* [x] Support common languages
* [x] Add fallback behavior for unknown languages
* [ ] Cache highlighted results if needed

Exit criteria:

* [x] Code blocks are clearly easier to read
* [ ] Highlighting does not noticeably hurt responsiveness

---

## Phase 8 - Viewer Quality Improvements

Goal: make the viewer pleasant for daily use.

* [x] Add a top bar or header
* [x] Show the current file name
* [x] Add reload status feedback
* [x] Add a basic theme foundation
* [x] Expand the theme system
* [x] Add theme switching
* [x] Add a dark mode friendly theme
* [x] Improve spacing and typography
* [x] Add basic keyboard shortcuts
* [x] Add simple zoom in / zoom out

Exit criteria:

* [ ] The app feels polished enough for regular use
* [ ] Basic usability issues are addressed

---

## Phase 9 - Navigation Features

Goal: improve usability for larger Markdown documents.

* [x] Add table of contents (TOC)
* [x] Support heading-based navigation
* [x] Add in-document search
* [x] Highlight search matches
* [x] Add jump-to-section behavior

Exit criteria:

* [ ] Large documents are easy to navigate
* [ ] Search and TOC work reliably

---

## Phase 10 - Large File Support

Goal: keep the app usable with larger Markdown files.

* [ ] Evaluate `ropey`
* [ ] Evaluate `memmap2`
* [x] Document large document performance measurement flow
* [x] Add helper script for baseline performance measurement
* [x] Record baseline performance results for 1 MiB and 5 MiB Markdown files
* [x] Identify the first measured bottleneck
* [x] Add lightweight render timing after load and reload
* [x] Avoid empty search highlight work during document rendering
* [x] Virtualize TOC rendering for large documents
* [x] Measure first render after TOC virtualization
* [x] Design viewport-based document rendering
* [x] Reduce memory copies where useful
* [ ] Improve incremental reload behavior if needed

Exit criteria:

* [ ] Large files remain usable
* [ ] Performance work is based on measurement, not guesswork

---

## Phase 11 - Optional Advanced Features

Goal: consider future extensions without committing too early.

* [x] Image rendering (local PNG/JPEG)
* [x] Link clicking
* [ ] External link opening behavior options
* [x] Drag and drop file open
* [ ] Multiple tabs
* [ ] Session restore
* [ ] Export options
* [ ] Optional CLI mode

Exit criteria:

* [ ] Advanced features are selected intentionally
* [ ] The core viewer remains simple

---

## Refactoring Rules

* [ ] Keep the project buildable after every meaningful change
* [ ] Prefer small, reviewable commits
* [ ] Avoid speculative abstractions
* [ ] Measure before optimizing
* [ ] Keep comments and documentation in English
* [ ] Prefer minimal dependencies unless clearly justified

---

## Current Priority

1. Reduce first-render cost for large documents
2. Reduce memory copies where useful
3. Improve incremental reload behavior if needed
4. Evaluate whether the static i18n table should stay or move to external resources later
---
