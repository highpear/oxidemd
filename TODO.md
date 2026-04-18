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
* [ ] Create the first runnable native window with `egui` / `eframe`
* [ ] Add basic project metadata to `Cargo.toml`

---

## Phase 1 - Minimal GUI Application

Goal: launch a native window and establish the smallest useful application structure.

* [ ] Add `egui` / `eframe`
* [ ] Split the code into:

  * [ ] `src/main.rs`
  * [ ] `src/app.rs`
* [ ] Show the app title in the UI
* [ ] Show a simple placeholder message
* [ ] Confirm the project builds and runs successfully

Exit criteria:

* The application opens a native window
* The UI renders correctly
* The code is clean and easy to extend

---

## Phase 1.5 - Internationalization

- [ ] Add i18n module
- [ ] Add en.json
- [ ] Add ja.json
- [ ] Replace hardcoded UI strings
- [ ] Add language switch (basic)

---

## Phase 2 - Load a Markdown File

Goal: load a Markdown file and display its raw text.

* [ ] Define how the first file is selected

  * [ ] Hardcoded sample path
  * [ ] Command-line argument
  * [ ] Drag and drop (later if needed)
* [ ] Read a `.md` file from disk
* [ ] Handle file read errors gracefully
* [ ] Display the loaded file contents as plain text
* [ ] Keep the UI responsive while loading

Exit criteria:

* A Markdown file can be opened
* Its contents are visible in the app
* Errors are shown clearly

---

## Phase 3 - Basic Markdown Parsing

Goal: parse Markdown and render core elements in a structured way.

* [ ] Add `pulldown-cmark`
* [ ] Create `src/parser.rs`
* [ ] Parse headings
* [ ] Parse paragraphs
* [ ] Parse unordered lists
* [ ] Parse ordered lists
* [ ] Parse blockquotes
* [ ] Parse fenced code blocks
* [ ] Define a simple intermediate representation for rendered content

Exit criteria:

* Markdown is not shown as raw text anymore
* Core block elements render correctly
* Parsing and rendering are separated

---

## Phase 4 - Basic Markdown Rendering

Goal: render parsed Markdown with a simple but readable UI.

* [ ] Create `src/renderer.rs`
* [ ] Render headings with visual hierarchy
* [ ] Render paragraphs with readable spacing
* [ ] Render lists cleanly
* [ ] Render blockquotes distinctly
* [ ] Render code blocks in a monospaced style
* [ ] Add scrollable document view

Exit criteria:

* Common Markdown documents are readable
* The viewer feels usable as a real application
* Rendering code remains simple and maintainable

---

## Phase 5 - File Watching / Live Reload

Goal: update the view automatically when the source file changes.

* [ ] Add `notify`
* [ ] Watch the currently opened file
* [ ] Re-read the file when it changes
* [ ] Re-parse content in the background
* [ ] Update the UI safely after reload
* [ ] Avoid duplicate or excessive reload events

Exit criteria:

* Editing the Markdown file updates the view automatically
* The UI remains responsive during reload
* File watching works reliably on the target platform

---

## Phase 6 - Performance Foundation

Goal: prepare the app for large files and frequent updates.

* [ ] Measure startup time
* [ ] Measure reload time
* [ ] Measure parse time
* [ ] Cache parsed output
* [ ] Avoid unnecessary re-parsing
* [ ] Avoid unnecessary allocations in hot paths
* [ ] Keep rendering work predictable

Exit criteria:

* Basic performance metrics are available
* Major bottlenecks are visible
* The codebase is ready for focused optimization

---

## Phase 7 - Syntax Highlighting

Goal: improve readability of fenced code blocks.

* [ ] Add `syntect`
* [ ] Highlight fenced code blocks
* [ ] Support common languages
* [ ] Add fallback behavior for unknown languages
* [ ] Cache highlighted results if needed

Exit criteria:

* Code blocks are clearly easier to read
* Highlighting does not noticeably hurt responsiveness

---

## Phase 8 - Viewer Quality Improvements

Goal: make the viewer pleasant for daily use.

* [ ] Add a top bar or header
* [ ] Show the current file name
* [ ] Add reload status feedback
* [ ] Expand the theme system
* [ ] Add theme switching
* [ ] Add a dark mode friendly theme
* [ ] Improve spacing and typography
* [ ] Add basic keyboard shortcuts
* [ ] Add simple zoom in / zoom out

Exit criteria:

* The app feels polished enough for regular use
* Basic usability issues are addressed

---

## Phase 9 - Navigation Features

Goal: improve usability for larger Markdown documents.

* [ ] Add table of contents (TOC)
* [ ] Support heading-based navigation
* [ ] Add in-document search
* [ ] Highlight search matches
* [ ] Add jump-to-section behavior

Exit criteria:

* Large documents are easy to navigate
* Search and TOC work reliably

---

## Phase 10 - Large File Support

Goal: keep the app usable with larger Markdown files.

* [ ] Evaluate `ropey`
* [ ] Evaluate `memmap2`
* [ ] Measure performance on large documents
* [ ] Reduce memory copies where useful
* [ ] Improve incremental reload behavior if needed

Exit criteria:

* Large files remain usable
* Performance work is based on measurement, not guesswork

---

## Phase 11 - Optional Advanced Features

Goal: consider future extensions without committing too early.

* [ ] Image rendering
* [ ] Link clicking
* [ ] External link opening
* [ ] Drag and drop file open
* [ ] Multiple tabs
* [ ] Session restore
* [ ] Export options
* [ ] Optional CLI mode

Exit criteria:

* Advanced features are selected intentionally
* The core viewer remains simple

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

1. Create the first runnable native window
2. Load and display a Markdown file as plain text
3. Parse basic Markdown structure
4. Render it cleanly
5. Add live reload

---
