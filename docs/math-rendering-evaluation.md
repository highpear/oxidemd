# Math Rendering Evaluation

Date: 2026-04-24

This note evaluates math rendering candidates for OxideMD.

## Goals

- Keep the app local and native.
- Avoid web views and external runtimes.
- Keep dependencies and architecture simple.
- Preserve fast startup and responsive reloads.
- Support both inline and display math.

## Parser Direction

OxideMD already uses `pulldown-cmark`.

As of `pulldown-cmark` 0.13.x, `Options::ENABLE_MATH` emits
`Event::InlineMath` and `Event::DisplayMath`.

That means OxideMD should prefer the built-in parser support instead of
growing custom math tokenization further.

## Candidates

### 1. `mathjax_svg_rs`

Summary:

- Renders TeX to SVG.
- Uses an embedded MathJax-based engine through a Rust wrapper.
- Exposes a shared worker thread model.

Pros:

- Output format is SVG, which fits document rendering well.
- Inline and display math can share the same backend.
- No Node.js or browser runtime is required at app runtime.

Cons:

- Still pulls in a non-trivial rendering stack.
- New crate with a short track record.
- SVG output would still need sizing, caching, and painting integration in egui.

Fit:

- Strong candidate for a first real renderer.
- Best match if OxideMD wants good TeX compatibility without introducing a web view.

### 2. Typst-based rendering (`typst`, `typst-svg`, related crates)

Summary:

- Typst can compile and lay out math, then export SVG or raster output.
- Existing tools such as `mdbook-typst-math` use this route.

Pros:

- Native Rust stack.
- High-quality typesetting potential.
- SVG export path already exists in the Typst ecosystem.

Cons:

- Much heavier integration surface than OxideMD currently needs.
- Requires compiler-style world, font loading, and document setup.
- Adds complexity beyond "render one formula quickly inside a viewer".

Fit:

- Technically viable, but oversized for the current project stage.
- Better suited if OxideMD later grows a broader document rendering pipeline.

### 3. `mathjax`

Summary:

- Renders MathJax expressions to SVG or PNG.
- Uses Node.js and/or headless Chrome backends.

Pros:

- Familiar output model.

Cons:

- Depends on external runtime choices.
- Directly conflicts with OxideMD's native and minimal direction.

Fit:

- Not a good fit.

### 4. Reuse logic from a larger renderer such as `markie`

Summary:

- `markie` advertises native Rust support for both LaTeX math and Mermaid.

Pros:

- Shows that a pure Rust path is possible.
- May be useful as a reference implementation when evaluating rendering quality.

Cons:

- It is a full renderer, not a small focused math crate.
- Pulling it in directly would likely introduce too much scope and coupling.

Fit:

- Good reference project.
- Not a likely direct dependency.

## Recommendation

Use this order:

1. Switch parser handling to `pulldown-cmark` `ENABLE_MATH`.
2. Prototype real rendering with `mathjax_svg_rs`.
3. Keep Typst as a fallback path only if SVG sizing quality or TeX coverage becomes a blocker.

## Why This Order

- The parser change is low risk and reduces custom logic.
- `mathjax_svg_rs` appears to give the best balance of native delivery and implementation scope.
- Typst looks powerful, but it brings in compiler-level complexity too early.

## Suggested Implementation Stages

### Stage 1

- Enable `Options::ENABLE_MATH`.
- Replace current custom `$...$` and `$$...$$` parsing with pulldown-cmark events.
- Keep today's fallback preview UI.

### Stage 2

- Add a math renderer adapter module with a narrow API:
  - render inline math
  - render display math
  - cache render results by expression, mode, theme, and zoom bucket

### Stage 3

- Prototype `mathjax_svg_rs` behind that adapter.
- Measure startup cost, first formula render cost, repeated render cost, and reload behavior.

### Stage 4

- If the prototype is acceptable, add:
  - error fallback UI
  - theme-aware color handling if needed
  - image/SVG cache invalidation rules

## Evaluation Criteria for the Prototype

- Build impact
- Startup impact
- First render latency
- Repeated render latency with cache hits
- Memory growth on a document with many repeated formulas
- Inline baseline alignment quality
- Display math readability at different zoom levels
- Windows reliability

## Sources

- `pulldown-cmark` options and math events:
  - https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html
  - https://docs.rs/pulldown-cmark/latest/pulldown_cmark/enum.Event.html
- `mathjax_svg_rs`:
  - https://docs.rs/crate/mathjax-svg-rs/0.4.0
  - https://docs.rs/crate/mathjax-svg-rs/latest/target-redirect/mathjax_svg_rs/
- `mathjax`:
  - https://docs.rs/mathjax/latest/mathjax/
- Typst:
  - https://docs.rs/typst/latest/typst/
  - https://docs.rs/typst-svg/latest/typst_svg/
- Example of Typst-based math preprocessing:
  - https://docs.rs/mdbook-typst-math/latest/src/mdbook_typst_math/lib.rs.html
- Reference renderer:
  - https://docs.rs/crate/markie/0.3.0
