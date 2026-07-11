# diagram

A native Rust platform for **diagram rendering, generation, analysis, and interchange** — with Mermaid/PlantUML compatibility, not Mermaid lock-in.

| Pillar | What it means |
|--------|----------------|
| **Render** | Fast, Chromium-free layout → SVG and PNG (PDF planned) |
| **Generate** | Structured create/edit via CLI + MCP (agents and scripts) |
| **Analyze** | Validate, diff, merge, and structural metrics on the IR |
| **Interchange** | Import/export across formats via a canonical IR |

**Why this vs Mermaid.js / PlantUML?** Single native binary, MCP-first agent workflows, analysis without a browser or JVM, and a format-agnostic core so you can keep existing Mermaid/PlantUML sources while moving toward a richer IR.

Today’s parsers speak Mermaid (flowchart, sequence, class, gantt). The roadmap centers a canonical IR with adapters for PlantUML, Graphviz DOT, D2, and more.

## Quick start

```bash
cargo install --path .

# Render (auto-detects kind)
diagram render sample.mmd --output out.png
diagram render sample.mmd --output out.svg
diagram preview sample.mmd

# Parse canonical JSON IR
diagram parse sample.mmd
diagram ir sample.mmd

# Import / export interchange
diagram import examples/sequence.puml --output sample.ir.json
diagram export sample.ir.json --output out.mmd --to mermaid

# Analyze
diagram validate sample.mmd
diagram metrics sample.mmd
diagram info sample.mmd
diagram diff base.mmd modified.mmd

# Generate
diagram create --kind flowchart --output new.mmd
diagram create --kind sequence --output new.puml
diagram add-node sample.mmd X "New Node" --shape stadium
diagram mcp   # agent tools over stdio
```

## Compatibility

| Format | Role today | Direction |
|--------|------------|-----------|
| Mermaid (`.mmd`) | Primary import + roundtrip for flowchart/sequence/class/gantt | Keep high Compatibility |
| Native JSON IR | Canonical interchange (`diagram ir`, `import`/`export`) | Stable |
| SVG | Render export | Stable |
| PlantUML (`.puml`) | Sequence import → sequence IR (MVP) | Class, activity, export |
| Graphviz DOT (`.dot`) | Import → flowchart IR (digraph subset) | Expand subset; export later |
| PNG | Render export (`.png` via `diagram render`) | Stable |
| PDF | — | Render export (planned) |

## CLI (current)

```bash
diagram parse sample.mmd
diagram info sample.mmd
diagram render sample.mmd [--output out.svg] [--watch] [--theme dark|light]
diagram preview sample.mmd [--port 3030] [--theme dark|light]
diagram validate sample.mmd
diagram metrics sample.mmd
diagram diff left.mmd right.mmd
diagram merge left.mmd right.mmd --output merged.mmd
diagram create --kind flowchart --output new.mmd
diagram add-node | remove-node | update-node | add-edge | remove-edge | update-edge ...
diagram get-node | get-edge | list-nodes | list-edges | get-mermaid | set-mermaid ...
diagram mcp
```

Examples under `examples/` cover flowchart, sequence, class, and gantt Mermaid sources.

## MCP

`diagram mcp` exposes parse, render, validate, diff/merge, and graph edit tools over stdio — designed for AI assistants that generate and analyze diagrams without shelling out to Chromium or Java.

See tool table in earlier releases / `SPEC.md`.

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
                                      (SVG/…)    (validate,   (CLI/MCP
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
├── diagram.rs / parser.rs / layout.rs / renderer.rs   # flowchart IR + Mermaid
├── sequence.rs / class.rs / gantt.rs                  # kind modules (Mermaid in → IR → SVG)
├── ir.rs / formats/ / analyze.rs / generate.rs        # canonical IR, adapters, analysis, scaffolds
```
