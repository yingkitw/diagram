# Architecture

## Product position

`diagram` is a **non-AI code ↔ UML conversion platform**. Source code and UML diagrams are two views of the same canonical IR. Mermaid / PlantUML / DOT / D2 are compatibility **Formats**, not the core identity. The conversion is **deterministic, zero-token, and local**; see `CONTEXT.md`, `docs/adr/0001-canonical-ir-and-format-adapters.md`, and `docs/adr/0002-non-ai-code-uml-conversion.md`.

## Target shape

```
                                   ┌──────────────┐
                                   │   Source code │
                                   │  Rust / TS    │
                                   └──────┬───────┘
                                          │ tree-sitter (deterministic, no tokens)
                                          ▼
   ┌─────────────┐   import    ┌──────────────────┐   export   ┌─────────────┐
   │  Formats    │ ──────────► │  Canonical IR    │ ─────────► │  Formats    │
   │  Mermaid    │             │  Document        │            │  Mermaid    │
   │  PlantUML   │             │   └─ Diagram[]   │            │  PlantUML   │
   │  DOT / D2   │             │       by Kind    │            │  DOT / D2   │
   │  JSON IR    │             └────────┬─────────┘            │  JSON IR    │
   └─────────────┘                      │                      └─────────────┘
                          ┌───────────┼───────────┐
                          ▼           ▼           ▼
                       Render      Analyze     Generate
                      layout+SVG   validate    CLI / MCP
                      PNG/PDF      diff/merge  structured edits
                                   metrics
                                          │
                                          │ skeleton generator (deterministic, no tokens)
                                          ▼
                                   ┌──────────────┐
                                   │  Skeleton src │
                                   │ Rust / TS    │
                                   └──────────────┘
```

The two arrows in/out of the IR (code on the left, skeleton on the right) make the bidirectional story explicit. Both sides share the same renderer, the same lossiness report, and the same MCP / CLI surface.

## Current modules (as implemented)

```
main.rs
  ├── cli.rs       — clap CLI dispatch (native feature)
  ├── mcp.rs       — MCP tools stdio (native)
  ├── embed.rs     — string-in SVG/JSON helpers (Wasm + embeds)
  ├── wasm.rs      — wasm-bindgen exports (`render_to_svg`, `parse_to_ir_json`)
  ├── preview.rs   — localhost live SVG preview; render to SVG/PNG/PDF (native)
  ├── png.rs       — SVG → PNG via resvg (native)
  ├── pdf.rs       — SVG → vector PDF via svg2pdf (usvg) (native)
  ├── ascii.rs     — IR → monospace ASCII art (flowchart boxes; text outline for other kinds)
  ├── lossiness.rs — export fidelity reports per Format
  ├── composite.rs — multi-diagram vertical SVG composite
  ├── markdown.rs  — fenced block extract → render → rewrite links
  ├── diagram.rs   — flowchart IR (Node, Edge, Subgraph, styles)
  ├── parser.rs    — Mermaid flowchart → flowchart IR
  ├── sequence.rs  — Mermaid sequence → sequence IR → SVG
  ├── class.rs     — Mermaid class → class IR → SVG
  ├── gantt.rs     — Mermaid gantt → gantt IR → SVG
  ├── state.rs     — Mermaid state → state IR → SVG
  ├── er.rs        — Mermaid ER → ER IR → SVG
  ├── ir.rs        — canonical Document / Diagram / Kind
  ├── formats/     — detect, import, export
  │     ├── mermaid.rs
  │     ├── dot.rs
  │     ├── d2.rs
  │     ├── plantuml.rs
  │     └── drawio.rs
  ├── analyze.rs   — structural metrics on IR
  ├── generate.rs  — kind-aware scaffolds + architecture templates (create)
  ├── codegen/     — code ↔ UML (the bidirectional story)
  │     ├── mod.rs         — public API, language + kind enums, write_to_path helper
  │     ├── rust_lang.rs   — tree-sitter Rust → IR (class / tree / call)
  │     ├── typescript_lang.rs — tree-sitter TypeScript → IR (class / tree / call)
  │     └── skeleton.rs    — IR → compilable Rust / TypeScript skeleton (UML → code)
  ├── layout.rs    — flowchart layered layout
  └── renderer.rs  — flowchart Layout → SVG
```

`formats::detect` chooses Format from content and path; `import_str` / `export_str` bridge to `ir::Document`. Lossiness runs before blocked exports.

## Module status

| Area | Responsibility | Status |
|------|----------------|--------|
| `ir` | Canonical `Document` / `Diagram` / `Kind`; JSON | Shipped |
| `formats` | Mermaid, JSON IR, DOT, D2, PlantUML (seq/class/activity), draw.io XML | Shipped — draw.io flowchart import/export added; D2 containers + DOT colors/URL; expand further as needed |
| `render` | Kind-aware layout + SVG/PNG/PDF/ASCII art backends | Shipped — PDF vector; ASCII art; SVG also via Wasm embed |
| `analyze` | Validate, diff, merge, metrics | Partial — diff + metrics shipped; merge flowchart-only |
| `generate` | CLI/MCP mutations, scaffolds, and architecture templates | Shipped — `create --kind`, `create --template`, `list-templates`; flowchart edit |
| `codegen` (code → UML) | tree-sitter Rust / TypeScript → IR (class / tree / call) | Shipped v1 — `generate-class|tree|call`; CLI + MCP |
| `codegen` (UML → code) | IR → skeleton Rust / TypeScript source | Shipped v1 — `generate-skeleton`; CLI + MCP; roundtrip stability test |
| `lossiness` | Export fidelity warnings per Format | Shipped v1 |

Mermaid parsers live under `formats::mermaid`; kind IR types remain in `parser` / `sequence` / `class` / `gantt`.

## Data flow (today)

```
.mmd / .puml / .dot / .d2 / .drawio / .json
    → formats::import → Document IR
    → render_svg / render (SVG|PNG|PDF|ASCII)
    → analyze (validate, metrics, diff, merge)
    → formats::export (+ lossiness) → target Format
```

PlantUML activity imports as flowchart `Diagram`. Multi-diagram documents composite vertically for render or export per-index / per-file in output-dir.

## Data flow (Code → UML, today)

```
source.rs / .ts
    → codegen::from_source (tree-sitter, language inferred)
    → Document IR (Class | Flowchart with items/calls)
    → render_svg / render / analyze / formats::export (+ lossiness)
```

Code → UML goes through the same `Document IR` spine as Format interchange. There is no second pipeline, no model call, and no token cost. `codegen::write_to_path` is the shared CLI/MCP entry point.

## Data flow (UML → Code, shipped)

```
.mmd / .json / .dot / .d2 / .puml / .drawio
    → formats::import → Document IR
    → codegen::skeleton (kind-aware, language-targeted)
    → source_skeleton.{rs,ts}
```

The skeleton generator walks the IR, preserves names and signatures, and emits empty function bodies. Determinism and zero-token are preserved.

## Design constraints

- **Non-AI first**: every conversion runs locally on grammars and IR. No model call, no API key, no network, no token cost.
- **Deterministic**: same input → byte-identical output. Roundtrip tests assert this per adapter.
- **Native + lean**: prefer std and small crates; no Chromium/JVM for the default path.
- **Compatibility over clone parity**: lossy export is OK if documented; roundtrip + lossiness tests per adapter.
- **IR-first APIs**: new MCP/CLI features operate on IR (or files via import), not Format strings alone.
- **MCP-native generation/analysis**: agent workflows are a primary interface — when an agent needs a diagram, calling `generate_class_diagram` should be cheaper and faster than asking an LLM.

## Technical notes

### File-based MCP state

Mutating tools read a path, modify, write back — stateless server, durable files.

### Tree-sitter for Code → UML

Per-language extractor modules (`codegen::rust_lang`, `codegen::typescript_lang`) share the same shape: parse → walk top-level declarations → emit `Document`. Adding a new language is a `Language` variant + an `extract(source, kind)` module; the IR pipeline is unchanged.

### Layered flowchart layout

BFS layers from sources; four directions via axis swap. No dagre.js dependency.

### Vector PDF

SVG → usvg tree → svg2pdf path/content streams → single-page PDF. Filters (if any) may rasterize locally; typical diagram SVGs stay vector.

### Preview server

Minimal `tokio` HTTP on localhost; HTML polls `/svg`.

### Wasm embed

`embed` + `wasm` features expose `render_to_svg` / `parse_to_ir_json` for browsers (`make wasm` → `examples/wasm/`). Default `native` feature keeps CLI/MCP/PNG/PDF and code→UML generation off the Wasm dependency graph.

### VS Code extension

`editors/vscode/` is a thin client: commands shell out to the `diagram` binary and show SVG in a webview. Layout/render stay in Rust — the extension does not embed Mermaid.js or Chromium.