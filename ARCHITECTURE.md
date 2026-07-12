# Architecture

## Product position

`diagram` is a **diagram platform**: render, generate, analyze, interchange. Mermaid is a compatibility **Format**, not the core identity. See `CONTEXT.md` and `docs/adr/0001-canonical-ir-and-format-adapters.md`.

## Target shape

```
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
```

## Current modules (as implemented)

```
main.rs
  ├── cli.rs       — clap CLI dispatch
  ├── mcp.rs       — MCP tools (stdio)
  ├── preview.rs   — localhost live SVG preview; render to SVG/PNG/PDF
  ├── png.rs       — SVG → PNG via resvg
  ├── pdf.rs       — SVG → PDF via resvg + printpdf (raster embed)
  ├── lossiness.rs — export fidelity reports per Format
  ├── composite.rs — multi-diagram vertical SVG composite
  ├── markdown.rs  — fenced block extract → render → rewrite links
  ├── diagram.rs   — flowchart IR (Node, Edge, Subgraph, styles)
  ├── parser.rs    — Mermaid flowchart → flowchart IR
  ├── sequence.rs  — Mermaid sequence → sequence IR → SVG
  ├── class.rs     — Mermaid class → class IR → SVG
  ├── gantt.rs     — Mermaid gantt → gantt IR → SVG
  ├── ir.rs        — canonical Document / Diagram / Kind
  ├── formats/     — detect, import, export
  │     ├── mermaid.rs
  │     ├── dot.rs
  │     └── plantuml.rs
  ├── analyze.rs   — structural metrics on IR
  ├── generate.rs  — kind-aware scaffolds (create)
  ├── layout.rs    — flowchart layered layout
  └── renderer.rs  — flowchart Layout → SVG
```

`formats::detect` chooses Format from content and path; `import_str` / `export_str` bridge to `ir::Document`. Lossiness runs before blocked exports.

## Module status

| Area | Responsibility | Status |
|------|----------------|--------|
| `ir` | Canonical `Document` / `Diagram` / `Kind`; JSON | Shipped |
| `formats` | Mermaid, JSON IR, DOT, D2, PlantUML (seq/class/activity) | Partial — expand subsets |
| `render` | Kind-aware layout + SVG/PNG/PDF backends | Partial — PDF is raster |
| `analyze` | Validate, diff, merge, metrics | Partial — diff + metrics shipped; merge flowchart-only |
| `generate` | CLI/MCP mutations and templates | Partial — create + flowchart edit |
| `lossiness` | Export fidelity warnings per Format | Shipped v1 |

Mermaid parsers live under `formats::mermaid`; kind IR types remain in `parser` / `sequence` / `class` / `gantt`.

## Data flow (today)

```
.mmd / .puml / .dot / .json
    → formats::import → Document IR
    → render_svg / render (SVG|PNG|PDF)
    → analyze (validate, metrics, diff, merge)
    → formats::export (+ lossiness) → target Format
```

PlantUML activity imports as flowchart `Diagram`. Multi-diagram documents composite vertically for render or export per-index / per-file in output-dir.

## Data flow (target)

```
any supported Format → Adapter::import → Document IR
Document IR → Render | Analyze | Generate
Document IR → Adapter::export (+ lossiness) → Format
```

## Design constraints

- **Native + lean**: prefer std and small crates; no Chromium/JVM for the default path
- **Compatibility over clone parity**: lossy export is OK if documented; roundtrip + lossiness tests per adapter
- **IR-first APIs**: new MCP/CLI features operate on IR (or files via import), not Mermaid strings alone
- **MCP-native generation/analysis**: agent workflows are a primary interface, not an afterthought

## Technical notes

### File-based MCP state

Mutating tools read a path, modify, write back — stateless server, durable files.

### Layered flowchart layout

BFS layers from sources; four directions via axis swap. No dagre.js dependency.

### Raster PDF

SVG → resvg pixmap → RGB (+ optional alpha mask) → single-page PDF via printpdf. Not vector PDF.

### Preview server

Minimal `tokio` HTTP on localhost; HTML polls `/svg`.
