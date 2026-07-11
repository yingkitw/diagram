# diagram

A native Rust platform for **diagram rendering, generation, analysis, and interchange** — with Mermaid/PlantUML compatibility, not Mermaid lock-in.

| Pillar | What it means |
|--------|----------------|
| **Render** | Fast, Chromium-free layout → SVG, PNG, and PDF (raster) |
| **Generate** | Structured create/edit via CLI + MCP (agents and scripts) |
| **Analyze** | Validate, structural diff (all IR kinds), merge (flowchart), metrics on the IR |
| **Interchange** | Import/export across formats via a canonical IR |

**Why this vs Mermaid.js / PlantUML?** Single native binary, MCP-first agent workflows, analysis without a browser or JVM, and a format-agnostic core so you can keep existing Mermaid/PlantUML sources while moving toward a richer IR.

**Formats today:** Mermaid (flowchart, sequence, class, gantt); native JSON IR; Graphviz DOT and PlantUML (sequence, class, activity) via adapters; SVG/PNG/PDF render output.

## Quick start

```bash
cargo install --path .

# Render (auto-detects kind; extension picks output format)
diagram render examples/multi-document.json --output all.svg
diagram render examples/simple-flowchart.mmd --output out.png
diagram render examples/simple-flowchart.mmd --output out.pdf
diagram render examples/multi-document.json --output-dir figures/ --output fig.png
diagram preview sample.mmd

# Parse canonical JSON IR
diagram parse sample.mmd
diagram ir sample.mmd

# Import / export interchange
diagram import examples/sequence.puml --output sample.ir.json
diagram import examples/activity.puml --output activity.ir.json
diagram import examples/simple-flowchart.dot --output flow.ir.json
diagram export sample.ir.json --output out.mmd --to mermaid
diagram export sample.ir.json --output out.dot --to dot
diagram export sample.ir.json --output out.puml --to plantuml
diagram lossiness sample.ir.json --to mermaid

# Analyze
diagram validate sample.mmd
diagram metrics sample.mmd
diagram info sample.mmd
diagram diff base.mmd modified.mmd   # IR-level diff; supports Mermaid, DOT, PlantUML, JSON

# Generate
diagram create --kind flowchart --output new.mmd
diagram create --kind sequence --output new.puml

# Markdown docs pipeline
diagram markdown examples/doc-with-diagrams.md --output-dir assets/diagrams --output guide.rendered.md
diagram add-node sample.mmd X "New Node" --shape stadium
diagram mcp   # agent tools over stdio
```

## Compatibility

| Format | Role today | Direction |
|--------|------------|-----------|
| Mermaid (`.mmd`) | Primary import + export for flowchart/sequence/class/gantt | Keep high Compatibility |
| Native JSON IR | Canonical interchange (`diagram ir`, `import`/`export`) | Stable |
| SVG | Render export | Stable |
| PNG | Render export (`.png` via `diagram render`) | Stable |
| PDF | Render export (`.pdf` via `diagram render`; raster embed) | Stable |
| PlantUML (`.puml`) | Sequence + class + activity-shaped flowchart import/export | Expand syntax |
| Graphviz DOT (`.dot`) | Flowchart import + export (digraph subset) | Expand subset |
| Lossiness report | `diagram lossiness` / `export --report` / MCP | Expand per-format warnings |

## CLI (current)

```bash
diagram parse | ir | import | export | lossiness
diagram info | render | preview | validate | metrics | diff | merge
diagram create --kind flowchart|sequence|class|gantt
diagram markdown
diagram add-node | remove-node | update-node | add-edge | remove-edge | update-edge ...
diagram get-node | get-edge | list-nodes | list-edges | get-mermaid | set-mermaid ...
diagram mcp
```

`diagram render` picks output format from `--output` extension: `.svg`, `.png`, or `.pdf`.

## Examples

Under `examples/`:

| File | Kind / format |
|------|----------------|
| `simple-flowchart.mmd`, `shapes.mmd`, `subgraphs.mmd`, … | Mermaid flowchart |
| `sequence.mmd`, `class.mmd`, `gantt.mmd` | Mermaid sequence / class / gantt |
| `sequence.puml`, `class.puml`, `activity.puml` | PlantUML |
| `simple-flowchart.dot` | Graphviz DOT |
| `multi-document.json` | Multi-diagram JSON IR |
| `doc-with-diagrams.md` | Markdown pipeline demo |

## MCP

`diagram mcp` exposes parse, import/export, lossiness, render (SVG/PNG/PDF), validate, diff/merge, metrics, markdown processing, and graph edit tools over stdio — designed for AI assistants without Chromium or Java.

See the full tool table in `SPEC.md`.

### Claude Desktop

```json
{
  "mcpServers": {
    "diagram": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "/path/to/diagram/Cargo.toml", "--", "mcp"]
    }
  }
}
```

## Architecture (target)

```
Formats (Mermaid, PlantUML, DOT, …)  ──import──►  Canonical IR  ──export──►  Formats
                                                      │
                                          ┌───────────┼───────────┐
                                          ▼           ▼           ▼
                                       Render      Analyze     Generate
                                    (SVG/PNG/PDF) (validate,   (CLI/MCP
                                                  diff, …)     tools)
```

See `ARCHITECTURE.md`, `CONTEXT.md`, and `docs/adr/0001-canonical-ir-and-format-adapters.md`.

## Testing

```bash
cargo test
```

## Project layout

```
src/
├── main.rs / cli.rs / mcp.rs / preview.rs
├── ir.rs / formats/ / lossiness.rs / analyze.rs / generate.rs / markdown.rs / composite.rs
├── diagram.rs / parser.rs / layout.rs / renderer.rs   # flowchart
├── sequence.rs / class.rs / gantt.rs                  # other kinds
├── png.rs / pdf.rs                                    # raster render backends
```
