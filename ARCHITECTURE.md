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
                     PNG/PDF…     diff/merge  structured edits
                                  metrics
```

## Current modules (as implemented)

```
main.rs
  ├── cli.rs      — clap CLI dispatch
  ├── mcp.rs      — MCP tools (stdio)
  ├── preview.rs  — localhost live SVG preview; render to SVG/PNG
  ├── png.rs      — SVG → PNG via resvg
  ├── pdf.rs      — SVG → PDF via resvg + printpdf (raster embed)
  ├── lossiness.rs — export fidelity reports per Format
  ├── composite.rs — multi-diagram vertical SVG composite
  ├── markdown.rs — fenced block extract → render → rewrite links
  ├── diagram.rs  — flowchart IR (Node, Edge, Subgraph, styles)
  ├── parser.rs   — Mermaid flowchart → flowchart IR
  ├── sequence.rs — Mermaid sequence → sequence IR → SVG
  ├── class.rs    — Mermaid class → class IR → SVG
  ├── gantt.rs    — Mermaid gantt → gantt IR → SVG
  ├── ir.rs       — canonical Document / Diagram / Kind
  ├── formats/    — format detect, import, export; `mermaid`, `dot`, `plantuml` adapters
  ├── analyze.rs  — structural metrics on IR
  ├── generate.rs — kind-aware scaffolds (create)
  ├── layout.rs   — flowchart layered layout
  └── renderer.rs — flowchart Layout → SVG
```

Detection today: `formats::detect` on path/content → Mermaid or JSON IR → `ir::Document`. Kind-specific Mermaid parsers feed `ir::Diagram` variants. `import`/`export` and MCP `import_diagram`/`export_diagram` use `formats.rs`.

## Planned module boundaries

| Area | Responsibility | Status |
|------|----------------|--------|
| `ir` | Canonical `Document` / `Diagram` / `Kind`; JSON | Shipped |
| `formats` | Adapters: Mermaid, JSON IR, DOT, PlantUML (sequence + class) | Partial |
| `render` | Kind-aware layout + SVG/PNG/PDF backends | Partial (SVG + PNG + PDF raster) |
| `analyze` | Validate, diff, merge, metrics (IR in → report out) | Partial (metrics shipped) |
| `generate` | CLI/MCP mutations and templates against IR | Partial (create scaffolds) |

Mermaid parsers live under `formats::mermaid`; kind IR types remain in `parser` / `sequence` / `class` / `gantt`.

## Data flow (today)

```
.mmd → detect Mermaid kind → kind IR → SVG
flowchart IR ↔ CLI/MCP mutate ↔ Mermaid text
```

## Data flow (target)

```
any supported Format → Adapter::import → Document IR
Document IR → Render | Analyze | Generate
Document IR → Adapter::export → Format
```

## Design constraints

- **Native + lean**: prefer std and small crates; no Chromium/JVM for the default path
- **Compatibility over clone parity**: lossy export is OK if documented; roundtrip tests per adapter
- **IR-first APIs**: new MCP/CLI features operate on IR (or files via import), not Mermaid strings alone
- **MCP-native generation/analysis**: agent workflows are a primary interface, not an afterthought

## Existing technical notes

### File-based MCP state

Mutating tools read a path, modify, write back — stateless server, durable files.

### Layered flowchart layout

BFS layers from sources; four directions via axis swap. No dagre.js dependency.

### Preview server

Minimal `tokio` HTTP on localhost; HTML polls `/svg`.
