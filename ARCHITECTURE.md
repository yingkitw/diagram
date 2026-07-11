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
  ├── preview.rs  — localhost live SVG preview
  ├── diagram.rs  — flowchart IR (Node, Edge, Subgraph, styles)
  ├── parser.rs   — Mermaid flowchart → flowchart IR
  ├── sequence.rs — Mermaid sequence → sequence IR → SVG
  ├── class.rs    — Mermaid class → class IR → SVG
  ├── gantt.rs    — Mermaid gantt → gantt IR → SVG
  ├── layout.rs   — flowchart layered layout
  └── renderer.rs — flowchart Layout → SVG
```

Detection today is Mermaid-header based (`graph` / `sequenceDiagram` / `classDiagram` / `gantt`). Kind-specific modules each own parse + (for non-flowchart) layout/render. Flowchart mutating CLI/MCP ops write Mermaid back via `to_mermaid()`.

## Planned module boundaries

| Area | Responsibility |
|------|----------------|
| `ir` | Canonical `Document` / `Diagram` / `Kind`; JSON schema |
| `formats::*` | Adapters: Mermaid, PlantUML, DOT, D2, … |
| `render` | Kind-aware layout + SVG/PNG/PDF backends |
| `analyze` | Validate, diff, merge, metrics (IR in → report out) |
| `generate` | CLI/MCP mutations and templates against IR |

Migrate existing `parser` / `sequence` / `class` / `gantt` into `formats::mermaid` + kind IR types without breaking CLI behavior.

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
