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
- Render core Markdown blocks
- Render basic inline Markdown styling
- Open links from rendered content
- Live reload when the opened file changes
- English and Japanese UI strings

Currently supported Markdown elements:

- Headings
- Paragraphs
- Unordered lists
- Ordered lists
- Blockquotes
- Fenced code blocks
- Strong text
- Emphasis
- Inline code
- Links

---

## Current Scope

The current target is intentionally small:

- Windows-first
- Single-file Markdown viewing
- Reliable readability over feature breadth

Items such as tabs, TOC, search, syntax highlighting, math, and Mermaid are planned for later phases.

---

## Planned Features

### Core Features

- Fast Markdown rendering
- Local file viewing
- Real-time preview with file watching

### Usability Features

- Familiar UI for non-technical users
- Theme expansion and theme switching
- Better spacing and typography
- Keyboard shortcuts

### Advanced Features

- Syntax highlighting
- Table of contents
- Search
- Large file improvements

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
