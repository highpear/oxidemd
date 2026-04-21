# AGENTS.md

## Project Overview

**OxideMD** is a fast and minimal Markdown viewer built with Rust.

The primary goal is to achieve:

* High performance (fast startup and rendering)
* Low memory usage
* Simple and maintainable architecture

This project prioritizes native performance over web-based approaches.

---

## Product Direction

Refer to README.md for product vision and feature scope.
Do not introduce features that conflict with the core principles.

---

## Tech Stack (Current)

* Language: Rust (stable)
* UI Framework: egui / eframe
* Markdown Parser: pulldown-cmark
* File Watching: notify
* Concurrency: std::thread or crossbeam-channel

Optional (future):

* Syntax Highlighting: syntect
* Large file handling: ropey / memmap2

---

## Internationalization (i18n)

- All user-facing text must go through the i18n system
- Do not hardcode UI strings
- Use short and simple phrases
- Default language is English
- Japanese must be supported

Translation keys must be stable and descriptive.

---

## Project Goals

* Render Markdown files quickly and efficiently
* Support real-time preview (file watching)
* Keep dependencies minimal
* Maintain clean and readable code

---

## Non-Goals (for now)

* Full-featured Markdown editor
* Complex plugin system
* Web-based UI (e.g., Tauri)

---

## Architecture Principles

* Keep parsing and rendering separated
* Avoid unnecessary allocations in hot paths
* Use background threads for heavy processing
* Cache parsed results when possible
* UI thread must remain responsive

---

## Coding Guidelines

* All code, comments, and documentation must be written in English
* Prefer simple and explicit implementations
* Avoid premature abstraction
* Keep functions small and focused
* Use descriptive naming

---

## File Structure (Initial)

src/

* main.rs        → entry point
* app.rs         → egui application logic (planned)
* renderer.rs    → Markdown rendering (planned)
* parser.rs      → Markdown parsing (planned)

---

## Performance Philosophy

* Measure before optimizing
* Avoid unnecessary re-parsing
* Minimize UI redraw cost
* Optimize only proven bottlenecks
* For performance-sensitive changes, use `docs/performance.md` as the source of truth for benchmark steps
* Prefer release builds for benchmark measurements
* Use `tools/run-performance-baseline.ps1` for Windows baseline measurements
* Record meaningful baseline or comparison results in `docs/performance.md`
* Do not commit generated large benchmark Markdown files; generate them outside the repository

---

## Naming Conventions

* crate name: oxidemd
* app name: OxideMD

---

## Agent Instructions

When modifying this project:

* Do not introduce heavy dependencies without justification
* Keep changes minimal and incremental
* Ensure the project builds at all times
* Prefer readability over cleverness
* If unsure, propose changes instead of implementing blindly

---

## Future Directions

* Incremental Markdown parsing
* Render tree caching
* Code block syntax highlighting
* Image lazy loading
* Table of contents (TOC)

---
