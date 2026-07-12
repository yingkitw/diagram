# diagram

**Rust diagram CLI & MCP server** — render, generate, analyze, and convert **Mermaid**, **PlantUML**, **Graphviz DOT**, **D2**, and **JSON IR** to **SVG**, **PNG**, or **PDF**. No Chromium, Node, or JVM required.

> Native alternative to **mermaid-cli (mmdc)**, **Kroki**, and headless **PlantUML** / **D2** for docs CI, agents, and local tooling.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

**Search terms:** mermaid renderer, mermaid to svg, mermaid to png, plantuml converter, dot to mermaid, d2 to svg, flowchart generator, sequence diagram tool, class diagram renderer, gantt chart cli, diagram diff, mcp diagram server, ai agent diagrams, architecture diagram as code.

## Contents

- [Features](#features)
- [Quick start](#quick-start)
- [Supported formats](#supported-formats)
- [CLI commands](#cli-commands)
- [Examples](#examples)
- [MCP (AI agents)](#mcp-ai-agents)
- [VS Code / Cursor](#vs-code--cursor)
- [Wasm embed](#wasm-embed)
- [Architecture](#architecture)
- [Testing](#testing)
- [Project layout](#project-layout)

## Features

| Capability | Keywords |
|------------|----------|
| **Render** | mermaid svg, png export, pdf export, live preview, wasm browser embed, watch mode, dark/light theme |
| **Interchange** | import export, format conversion, canonical json ir, lossiness report |
| **Analyze** | validate diagram, structural diff, merge flowcharts, metrics (cycles, depth, orphans) |
| **Generate** | scaffold flowchart/sequence/class/gantt, cli edit nodes and edges |
| **Agents** | model context protocol, stdio mcp, claude desktop, cursor |

**Diagram kinds:** flowchart, sequence diagram, class diagram, gantt chart, state diagram, ER diagram.

**Why choose this over Mermaid.js / PlantUML?** Single native binary, MCP-first agent workflows, structural analysis on a canonical IR, and multi-format interchange — not a Mermaid-only clone.

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
diagram import examples/simple-flowchart.d2 --output flow-d2.ir.json
diagram export sample.ir.json --output out.mmd --to mermaid
diagram export sample.ir.json --output out.dot --to dot
diagram export sample.ir.json --output out.d2 --to d2
diagram export sample.ir.json --output out.puml --to plantuml
diagram lossiness sample.ir.json --to mermaid

# Analyze
diagram validate sample.mmd
diagram metrics sample.mmd
diagram info sample.mmd
diagram diff base.mmd modified.mmd   # IR-level diff; Mermaid, DOT, D2, PlantUML, JSON

# Generate
diagram create --kind flowchart --output new.mmd
diagram create --kind sequence --output new.puml
diagram create --kind flowchart --output new.d2

# Markdown docs pipeline
diagram markdown examples/doc-with-diagrams.md --output-dir assets/diagrams --output guide.rendered.md
diagram add-node sample.mmd X "New Node" --shape stadium
diagram mcp   # agent tools over stdio
```

## Supported formats

| Format | Extensions | Import | Export | Notes |
|--------|------------|--------|--------|-------|
| **Mermaid** | `.mmd`, `.mermaid` | ✓ | ✓ | Flowchart, sequence, class, gantt, state, er |
| **JSON IR** | `.json` | ✓ | ✓ | Canonical, lossless interchange |
| **PlantUML** | `.puml`, `.plantuml` | ✓ | ✓ | Sequence, class, activity → flowchart |
| **Graphviz DOT** | `.dot`, `.gv` | ✓ | ✓ | Digraph subset ↔ flowchart |
| **D2** | `.d2` | ✓ | ✓ | Flat flowchart subset |
| **SVG** | `.svg` | — | ✓ | Primary render output |
| **PNG** | `.png` | — | ✓ | Raster via resvg |
| **PDF** | `.pdf` | — | ✓ | Vector via svg2pdf |

Use `diagram lossiness` before export to see what IR semantics a target format cannot represent.

## CLI commands

```bash
diagram parse | ir | import | export | lossiness
diagram info | render | preview | validate | metrics | diff | merge
diagram create --kind flowchart|sequence|class|gantt|state|er
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
| `sequence.mmd`, `class.mmd`, `gantt.mmd`, `state.mmd`, `er.mmd` | Mermaid sequence / class / gantt / state / er |
| `sequence.puml`, `class.puml`, `activity.puml` | PlantUML |
| `simple-flowchart.d2`, `containers.d2` | D2 (flat + containers/subgraphs) |
| `simple-flowchart.dot` | Graphviz DOT |
| `multi-document.json` | Multi-diagram JSON IR |
| `doc-with-diagrams.md` | Markdown pipeline demo |
| `embed/*` | Compact fixtures for Wasm/embed string API (all formats) |
| `wasm/` | Browser demo (`make wasm` → `pkg/`) |

## MCP (AI agents)

`diagram mcp` exposes parse, import/export, lossiness, render (SVG/PNG/PDF), validate, diff/merge, metrics, markdown processing, and graph edit tools over stdio — designed for AI assistants without Chromium or Java.

See the full tool table in [`SPEC.md`](SPEC.md).

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

## VS Code / Cursor

A lean extension under [`editors/vscode/`](editors/vscode/) shells out to the `diagram` CLI:

- **Diagram: Preview SVG** — side webview (refreshes on save)
- **Diagram: Validate** — flowchart validation
- **Diagram: Render SVG to File**

```bash
# Install CLI first, then load the extension
cargo install --path .
cursor --extensionDevelopmentPath="$(pwd)/editors/vscode"
```

See [`editors/vscode/README.md`](editors/vscode/README.md) for settings (`diagram.cliPath`, theme).

## Wasm embed

Browser SVG preview without a local server (parse + layout + SVG in Wasm):

```bash
# Requires: rustup target add wasm32-unknown-unknown && cargo install wasm-pack
make wasm
# Serve examples/wasm/ (needs the generated pkg/)
python3 -m http.server -d examples/wasm 8080
```

JS API after `await init()`:

- `render_to_svg(source, theme)` — Mermaid / DOT / D2 / PlantUML / JSON IR → SVG (`theme`: `dark`|`light`)
- `parse_to_ir_json(source)` — same sources → Document IR JSON

Native-only features (CLI, MCP, PNG/PDF, preview server) are behind the default `native` Cargo feature; Wasm builds use `--no-default-features --features wasm`.

## Architecture

```
Formats (Mermaid, PlantUML, DOT, D2, …)  ──import──►  Canonical IR  ──export──►  Formats
                                                      │
                                          ┌───────────┼───────────┐
                                          ▼           ▼           ▼
                                       Render      Analyze     Generate
                                    (SVG/PNG/PDF) (validate,   (CLI/MCP
                                                  diff, …)     tools)
```

See [`ARCHITECTURE.md`](ARCHITECTURE.md), [`CONTEXT.md`](CONTEXT.md), and [`docs/adr/0001-canonical-ir-and-format-adapters.md`](docs/adr/0001-canonical-ir-and-format-adapters.md).

## Testing

```bash
cargo test
cargo test --test embed_tests   # Wasm/embed string API × examples/embed
make vscode-check   # validates editors/vscode package.json + extension.js
make wasm-check     # typecheck Wasm feature for wasm32-unknown-unknown
```

## Project layout

```
src/
├── main.rs / cli.rs / mcp.rs / preview.rs   # native feature
├── embed.rs / wasm.rs                       # browser embed
├── ir.rs / formats/ / lossiness.rs / analyze.rs / generate.rs / markdown.rs / composite.rs
├── diagram.rs / parser.rs / layout.rs / renderer.rs   # flowchart
├── sequence.rs / class.rs / gantt.rs / state.rs / er.rs
├── png.rs / pdf.rs                          # native feature
editors/vscode/                              # VS Code / Cursor extension
examples/wasm/                               # Wasm demo (pkg/ from `make wasm`)
```
