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

- Open a single Markdown file with a native file dialog
- Open a Markdown file from the command line
- Open a Markdown file with drag and drop
- Render core Markdown blocks
- Render basic inline Markdown styling
- Open links from rendered content
- Render local PNG and JPEG images
- Live reload when the opened file changes
- English and Japanese UI strings
- Theme switching and document zoom
- Table of contents and heading navigation
- In-document search with match highlighting
- Syntax highlighting for fenced code blocks
- Copy fenced code block contents

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

---

## Current Scope

The current target is intentionally small:

- Windows-first
- Single-file Markdown viewing
- Reliable readability over feature breadth

Items such as tabs, large file improvements, math, and Mermaid are planned for later phases.

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
- Current i18n approach: Rust static mappings

Shared manual test files live in `samples/`.
Use `samples/long-form.md` to test longer reading flows such as heading navigation, zoom, theme changes, and live reload on a larger document.

Performance measurement notes live in `docs/performance.md`.
