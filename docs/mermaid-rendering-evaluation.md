# Mermaid Rendering Evaluation

Date: 2026-04-27

This note evaluates Mermaid rendering candidates for OxideMD.

## Current Status

OxideMD already recognizes fenced code blocks with `mermaid` or `mmd` info strings.

Those blocks are parsed as diagram blocks and rendered with:

- a shared embedded-content header
- source copy
- a readable source fallback

SVG rendering is now prototyped through `mermaid-rs-renderer`.

The codebase has a diagram renderer adapter and cache. It starts Mermaid SVG
rendering on a background thread, returns a pending state during rendering, and
keeps the readable source fallback visible for pending and failed renders.

## Goals

- Keep the app local and native.
- Preserve fast startup and responsive reloads.
- Avoid introducing a web-based UI architecture.
- Keep dependencies minimal and justified.
- Render Mermaid diagrams as SVG if the runtime cost is acceptable.
- Always keep readable source fallback behavior.
- Keep diagram rendering off the UI thread.

## Non-Goals

- Do not require users to install Node.js for normal viewing.
- Do not make Mermaid support a plugin system.
- Do not block the core Markdown viewer on diagram rendering.
- Do not prioritize perfect Mermaid coverage over app simplicity.

## Starting Assumption

The first prototype should prefer a Rust-native renderer if it can produce usable
SVG for common diagrams without hurting startup, binary size, or reload
responsiveness.

Mermaid CLI should be treated as the compatibility reference, not the default
runtime path.

## Parser and UI Direction

Mermaid fenced blocks should stay mapped to the shared embedded-SVG content model.

The parser should keep distinguishing diagram blocks from ordinary code blocks so that:

- diagram source can be copied separately
- future SVG rendering can be cached independently
- failed rendering can fall back to readable source text
- code syntax highlighting remains focused on real code blocks

## Candidates

### 1. Rust-native renderer (`mermaid-rs-renderer`)

Summary:

- Renders Mermaid-like source to SVG in Rust.
- Does not require Node.js, Chromium, or a WebView.
- Advertises support for common diagram types such as flowchart, sequence,
  class, state, ER, pie, Gantt, timeline, mindmap, and git graph.

Pros:

- Best current architectural fit for OxideMD's local native direction.
- SVG output matches the shared embedded SVG model already used by math.
- Avoids process spawning and external runtime setup.
- Likely easier to cache and run behind a narrow adapter boundary.
- Can be tested with default features disabled to avoid PNG or CLI extras if
  SVG-only rendering is enough.

Cons:

- Mermaid compatibility may lag the official JavaScript implementation.
- Layout quality needs visual comparison against Mermaid CLI.
- The crate is young enough that API stability and maintenance risk should be
  checked before making it a hard dependency.
- Dependency size, compile time, and transitive crates still need measurement.

Fit:

- Strongest first prototype candidate.
- Should stay behind an adapter so it can be replaced or disabled if it fails
  compatibility, size, or reliability checks.

### 2. Mermaid CLI (`mmdc`, `@mermaid-js/mermaid-cli`)

Summary:

- Uses the official Mermaid CLI.
- Usually relies on Node.js and browser automation under the hood.
- Can output SVG files from Mermaid source.

Pros:

- Strong Mermaid compatibility.
- The output format matches OxideMD's embedded SVG direction.
- Useful as a reference for rendering quality and expected output.

Cons:

- Requires an external runtime or bundled tooling.
- Increases setup and distribution complexity.
- Process spawning and file I/O need careful timeout and error handling.
- A hard dependency would conflict with OxideMD's minimal local viewer direction.

Fit:

- Good reference and optional developer tool.
- Not a good default runtime dependency.

### 3. Embedded JavaScript Engine

Summary:

- Bundle Mermaid JavaScript and run it through an embedded JS engine.
- Potential engines include QuickJS or V8-based crates.

Pros:

- Could avoid requiring a separate Node.js installation.
- Keeps rendering inside the app process.
- May support Mermaid more directly than a partial Rust renderer.

Cons:

- Adds a large and security-sensitive integration surface.
- Mermaid may expect browser-like DOM APIs beyond a bare JS engine.
- Startup cost, binary size, and memory use need measurement.
- Sandboxing and timeouts become important.

Fit:

- Technically plausible, but high risk for the current project stage.
- Worth evaluating only behind a narrow adapter boundary.

### 4. WebView or Headless Browser

Summary:

- Render Mermaid in a browser-like environment and extract SVG.

Pros:

- Mermaid is designed for browser environments.
- Compatibility can be strong.

Cons:

- Conflicts with OxideMD's non-web-based architecture direction.
- Adds substantial runtime weight.
- Headless browser control can be fragile across machines.
- Harder to keep startup and memory use small.

Fit:

- Poor fit for OxideMD's current goals.

### 5. Other Rust-Native or Compatible Renderer

Summary:

- Use or build a Rust renderer for Mermaid-like diagrams.

Pros:

- Best architectural fit if a capable crate exists.
- No JavaScript runtime required.
- Easier to integrate with native caching and threading.

Cons:

- Full Mermaid compatibility is a large surface area.
- Available crates may cover only a subset or differ from Mermaid semantics.
- Building a custom renderer would be too much scope for OxideMD.

Fit:

- Good to watch and evaluate.
- Not enough evidence yet for a direct dependency.

### 6. Keep Source Fallback Only

Summary:

- Keep Mermaid blocks readable and copyable without rendering diagrams.

Pros:

- Zero new runtime dependency.
- Stable and fast.
- Keeps documents usable even without SVG rendering.
- Preserves a safe fallback path for all future renderer failures.

Cons:

- Does not provide visual diagrams.
- Less useful for users reading diagram-heavy documents.

Fit:

- Best current baseline.
- Should remain available even if SVG rendering is added later.

## Recommendation

Use this order:

1. Keep the current Mermaid source fallback as the stable baseline.
2. Add a narrow diagram renderer adapter API. Done as a fallback-only adapter.
3. Prototype `mermaid-rs-renderer` behind that adapter with SVG-only output if
   possible. Done.
4. Use Mermaid CLI as an external reference path for manual quality comparison.
5. Only prototype an embedded JS or browser-like renderer if the Rust-native
   path fails and measurement justifies the dependency.

## Why This Order

- The fallback path is already useful and low risk.
- Mermaid rendering can easily pull OxideMD toward heavy runtime dependencies,
  so the first prototype should avoid Node.js, Chromium, and WebView paths.
- An adapter boundary lets the app keep parsing, caching, fallback, and UI behavior stable.
- Manual comparison against Mermaid CLI can validate output expectations before committing to a backend.

## First Prototype Scope

The first prototype should be intentionally narrow:

- Add a diagram renderer module with a cache shape similar to math rendering.
- Accept only Mermaid source, language, theme, and zoom-independent render
  options.
- Return the shared embedded SVG result type.
- Render only block diagrams, not inline content.
- Keep the current readable source fallback for pending, unsupported, and failed
  renders.
- Run rendering off the UI thread before enabling it for normal viewing.

The first supported sample set should include:

- one small flowchart
- one sequence diagram
- one class diagram
- one state diagram
- one intentionally invalid diagram
- one larger flowchart for timeout and cache behavior

## Suggested Adapter Shape

The first adapter should be small:

- input: Mermaid source text and diagram language
- output:
  - rendered SVG content
  - pending state, if rendering becomes asynchronous
  - fallback error text
- cache key:
  - source text
  - language
  - theme or color mode, if the backend supports them
  - zoom bucket only if SVG generation depends on zoom

The UI should keep using the shared embedded SVG block rendering functions.

## Evaluation Criteria for a Prototype

- Build impact
- Binary size impact
- Startup impact
- First diagram render latency
- Repeated render latency with cache hits
- Memory growth across many diagrams
- Windows reliability
- Offline behavior
- Error quality and fallback behavior
- Timeout handling for malformed or very large diagrams
- Security and sandboxing implications
- Compatibility with common diagram types:
  - flowchart
  - sequence diagram
  - class diagram
  - state diagram

## Known Prototype Limitations

- State diagram edge labels can overlap when several labeled transitions leave
  the same state. Keep evaluation labels short until renderer-side label
  placement is improved or a workaround is chosen.
- Flowchart edge routing can look awkward in dense left-to-right diagrams with
  nearby branches and joins. Prefer top-down evaluation diagrams for now and
  compare layout quality against Mermaid CLI before treating this backend as
  final.
- OxideMD adds a narrow local validation guard for clearly incomplete arrows
  such as `Broken -->` because the Rust renderer can otherwise interpret some
  incomplete input as a renderable diagram.

## Current Decision

`mermaid-rs-renderer` is now added as the first measured SVG backend candidate
with default features disabled.

Release-test measurement for the evaluation sample now covers flowchart,
sequence, class, state, a larger flowchart, invalid input fallback, and finished
result cache reuse. Results are recorded in `docs/performance.md`.

The next useful implementation step is visual comparison against Mermaid CLI on
common diagram types, followed by documenting known syntax limitations and
fallback behavior.

## Sources

- Mermaid project:
  - https://mermaid.js.org/
- Mermaid CLI:
  - https://github.com/mermaid-js/mermaid-cli
- `mermaid-rs-renderer`:
  - https://docs.rs/mermaid-rs-renderer/latest/mermaid_rs_renderer/
- Existing OxideMD math rendering evaluation:
  - `docs/math-rendering-evaluation.md`
