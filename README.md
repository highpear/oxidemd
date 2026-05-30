## Product Vision

OxideMD is a fast, local-first Markdown viewer designed for both developers and non-technical users.

The goal is to provide a simple, intuitive, and high-performance experience for reading Markdown documents.

---

## Core Principles

- Fully local
- Fast and lightweight
- Simple and intuitive UI
- Designed for reading first
- Clean and distraction-free experience

---

## Current Status

The project is in early `v0.1` development.

Current capabilities:

- Open Markdown files with a native file dialog
- Open a Markdown file from the command line
- Open one or more Markdown files with drag and drop
- View multiple Markdown files in tabs
- Switch to an already open tab instead of opening duplicate tabs for the same file
- Render core Markdown blocks
- Render basic inline Markdown styling
- Open links from rendered content
- Choose whether external links open directly or ask first
- Render local PNG and JPEG images
- Render inline and display math visually
- Live reload when opened files change, including inactive tabs
- English and Japanese UI strings
- Theme switching and document zoom
- Table of contents and heading navigation
- In-document search with match highlighting
- Keyboard shortcut help
- Syntax highlighting for fenced code blocks
- Copy fenced code block contents
- Show Mermaid fenced blocks with readable fallback and source copy
- Copy the current file path
- Export the active Markdown file as HTML
- Export Markdown as HTML from the command line
- Restore the last session settings, open tabs, and active tab
- Reopen recently opened Markdown files

Currently supported Markdown elements:

- Headings
- Paragraphs
- Unordered lists
- Ordered lists
- Blockquotes
- Fenced code blocks
- Tables
- Strong text
- Emphasis
- Inline code
- Links
- Images (local PNG/JPEG)
- Inline and display math (`$...$`, `$$...$$`)
- Mermaid fenced blocks as readable source fallback

---

## Current Scope

The current target is intentionally small:

- Windows and macOS local desktop viewing
- Local Markdown viewing with lightweight tabs
- Reliable readability over feature breadth

Items such as large file improvements, richer math polish, and Mermaid refinement are planned for later phases.

---

## Planned Features

### Core Features

- Fast Markdown rendering
- Local file viewing
- Real-time preview with file watching

### Usability Features

- Familiar UI for non-technical users
- Further theme expansion
- Further spacing and typography improvements
- Keyboard shortcuts

### Advanced Features

- Large file improvements
- External link opening options

---

## Non-Goals (for now)

- Full Markdown editor
- Cloud sync
- Plugin ecosystem
- Web-based UI

---

## Development Notes

- UI framework: `egui` / `eframe`
- Markdown parser: `pulldown-cmark`
- File watching: `notify`
- Current i18n approach: a single Rust static translation table. This keeps the app dependency-free and simple while the supported UI text remains small.

Shared manual test files live in `samples/`.
Use `samples/long-form.md` to test longer reading flows such as heading navigation, zoom, theme changes, and live reload on a larger document.

Performance measurement notes live in `docs/performance.md`. Record Windows and
macOS baseline results separately.

## Development Checks

Run formatting before committing Rust changes:

```bash
cargo fmt
cargo fmt --check
```

The project-wide rustfmt rules live in `rustfmt.toml`.

Run the normal test suite for behavior changes:

```bash
cargo test
```

Run a build check for build-sensitive changes:

```bash
cargo build
```

## Command Line

Open a Markdown file in the viewer:

Windows:

```powershell
.\target\release\oxidemd.exe path\to\file.md
```

macOS:

```bash
./target/release/oxidemd path/to/file.md
```

Start without reopening the previous file:

```bash
oxidemd --no-restore-file
```

When a Markdown file is provided on the command line, OxideMD opens that file instead of restoring the previous tabs. Other saved settings, such as theme and zoom, are still restored.

Reset saved session settings and recent files:

```bash
oxidemd --reset-session
```

Export Markdown as a standalone HTML file without opening the GUI:

Windows:

```powershell
oxidemd --export-html path\to\input.md path\to\output.html
```

macOS:

```bash
oxidemd --export-html path/to/input.md path/to/output.html
```
