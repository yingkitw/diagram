# diagram

**Native non-AI code ↔ UML conversion** — bidirectional, deterministic, zero tokens, instant. Rust, TypeScript → Mermaid/PlantUML/DOT/D2/JSON IR, and back as compilable source skeletons.

> The single-binary, no-network, no-LLM alternative to Mermaid + PlantUML + Mermaid-JS + AI diagram helpers. Built for agents and humans who don't want to pay model tokens or wait on model latency for diagram work that a tree-sitter grammar can do in microseconds.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/badge-rust-2024-orange.svg)](https://www.rust-lang.org/)

**Search terms:** code to uml, code to mermaid, code to plantuml, code to d2, code to graphviz, uml to code, mermaid to rust, mermaid to typescript, class diagram generator, call graph generator, module tree diagram, tree-sitter diagram, mcp diagram server, ai agent diagrams, mermaid renderer, mermaid to svg, plantuml converter, dot to mermaid, d2 to svg, diagram diff, architecture diagram as code, **non-ai diagram, no token diagram**.

## Why

Diagrams and source drift the moment a human (or a slow, token-billed LLM) has to keep them in sync. `diagram` closes that gap with:

| | Cost | Latency | Determinism |
|--|------|---------|-------------|
| LLM-based code→diagram | tokens per call | network + model | varies |
| Hand-edited diagrams | human time | hours | yes |
| **`diagram` (this project)** | **zero** | **sub-millisecond per file** | **byte-identical** |

- **Non-AI** — extraction runs on tree-sitter grammars; generation runs on the canonical IR. No model, no API.
- **No token consumption** — every conversion is local and free.
- **Fast** — a typical Rust source file extracts a class diagram in single-digit milliseconds on commodity hardware.
- **Bidirectional** — code → diagram (class / tree / call) and diagram → skeleton source (Rust / TypeScript).
- **Multi-format** — Mermaid, PlantUML, Graphviz DOT, D2, draw.io XML, JSON IR; SVG/PNG/PDF/ASCII render.

## Contents

- [Why](#why)
- [Quick start](#quick-start)
- [Code ↔ UML](#code--uml)
- [Supported formats](#supported-formats)
- [CLI commands](#cli-commands)
- [MCP (AI agents)](#mcp-ai-agents)
- [VS Code / Cursor](#vs-code--cursor)
- [Wasm embed](#wasm-embed)
- [Architecture](#architecture)
- [Testing](#testing)
- [Project layout](#project-layout)

## Quick start

```bash
cargo install --path .

# ─── Code → UML ───────────────────────────────────────────────────────────────
# Generate a class diagram from source (tree-sitter; deterministic, no tokens)
diagram generate-class tests/fixtures/code-sample.rs --output class.mmd
diagram generate-class tests/fixtures/code-sample.ts --output class.mmd --lang typescript

# Module / file tree as a flowchart
diagram generate-tree  tests/fixtures/code-sample.rs --output tree.mmd

# Function call graph
diagram generate-call  tests/fixtures/code-sample.rs --output call.mmd

# ─── UML → Code ───────────────────────────────────────────────────────────────
# Deterministic skeleton: class diagram → compilable Rust stubs
diagram generate-skeleton class.mmd --lang rust --output src/skeleton.rs

# ─── Render & interchange (any diagram) ───────────────────────────────────────
diagram render examples/multi-document.json --output all.svg
diagram render examples/simple-flowchart.mmd --output out.txt  # ASCII art
diagram render examples/simple-flowchart.mmd --output out.png
diagram render examples/simple-flowchart.mmd --output out.pdf
diagram preview sample.mmd

# Parse canonical JSON IR
diagram parse sample.mmd
diagram ir sample.mmd

# Import / export interchange
diagram import examples/sequence.puml --output sample.ir.json
diagram import examples/simple-flowchart.dot --output flow.ir.json
diagram import examples/simple-flowchart.d2 --output flow-d2.ir.json
diagram import examples/simple-flowchart.drawio --output flow-drawio.ir.json
diagram export sample.ir.json --output out.mmd --to mermaid
diagram export sample.ir.json --output out.dot --to dot
diagram export sample.ir.json --output out.d2 --to d2
diagram export sample.ir.json --output out.puml --to plantuml
diagram export sample.ir.json --output out.drawio --to drawio
diagram lossiness sample.ir.json --to mermaid

# Analyze
diagram validate sample.mmd
diagram metrics sample.mmd
diagram info sample.mmd
diagram diff base.mmd modified.mmd

# Generate (scaffolds and templates)
diagram create --kind flowchart --output new.mmd
diagram create --kind sequence --output new.puml
diagram create --template aws-3tier --output aws.mmd
diagram list-templates

# Markdown docs pipeline
diagram markdown examples/doc-with-diagrams.md --output-dir assets/diagrams --output guide.rendered.md
diagram add-node sample.mmd X "New Node" --shape stadium
diagram mcp   # agent tools over stdio
```

## Code ↔ UML

The two directions are first-class and share the canonical **IR**.

### Code → UML

`generate-class | generate-tree | generate-call <file>` reads a source file, parses it with a tree-sitter grammar, and emits a Diagram in any supported format.

| Kind | What it produces | Languages |
|------|------------------|-----------|
| `class` | Class diagram (struct/class/interface/enum/trait/impl + relations) | Rust, TypeScript |
| `tree` | Flowchart of top-level items + import/use edges from a root file node | Rust, TypeScript |
| `call` | Function-call graph; missing callees are synthesized as external nodes | Rust, TypeScript |

The output format is selected by the output extension: `.mmd` / `.json` / `.dot` / `.d2` / `.puml`. The pipeline is the same one used for Format interchange — generated diagrams are first-class Documents.

### UML → Code

`generate-skeleton <diagram-file> --lang rust|typescript --output <file>` reads any diagram (class / flowchart / sequence / state / ER) and writes a compilable skeleton of source code. Signatures are real; bodies are empty stubs.

```bash
# Class diagram → Rust traits + structs (impl bodies are stubbed)
diagram generate-skeleton class.mmd --lang rust --output src/generated.rs

# Flowchart of items → Rust modules with empty function bodies
diagram generate-tree code-sample.rs --output tree.mmd
diagram generate-skeleton tree.mmd --lang rust --output src/skeleton.rs

# Sequence diagram → TypeScript class skeletons
diagram generate-skeleton sequence.mmd --lang typescript --output client.ts
```

This makes the roundtrip **Code → UML → Skeleton** useful for bootstrapping a refactor, generating stub interfaces from an architecture diagram, or keeping an externally-authored diagram in lockstep with code.

## Supported formats

| Format | Extensions | Import | Export | Notes |
|--------|------------|--------|--------|-------|
| **Mermaid** | `.mmd`, `.mermaid` | ✓ | ✓ | Flowchart, sequence, class, gantt, state, er |
| **JSON IR** | `.json` | ✓ | ✓ | Canonical, lossless interchange |
| **PlantUML** | `.puml`, `.plantuml` | ✓ | ✓ | Sequence, class, activity → flowchart |
| **Graphviz DOT** | `.dot`, `.gv` | ✓ | ✓ | Digraph subset ↔ flowchart IR |
| **D2** | `.d2` | ✓ | ✓ | Flat flowchart subset |
| **draw.io** | `.drawio` | ✓ | ✓ | `<mxfile>` uncompressed subset ↔ flowchart IR |
| **SVG** | `.svg` | — | ✓ | Primary render output |
| **PNG** | `.png` | — | ✓ | Raster via resvg |
| **PDF** | `.pdf` | — | ✓ | Vector via svg2pdf |
| **ASCII art** | `.txt` / `.ascii` | — | ✓ | Monospace box-and-arrow text |

Use `diagram lossiness` before export to see what IR semantics a target format cannot represent.

## CLI commands

```bash
# Code → UML
diagram generate-class  <code-file>
diagram generate-tree   <code-file>
diagram generate-call   <code-file>

# UML → Code
diagram generate-skeleton <diagram-file> --lang rust|typescript

# Format interchange + render + analyze
diagram parse | ir | import | export | lossiness
diagram info | render | preview | validate | metrics | diff | merge
diagram create --kind flowchart|sequence|class|gantt|state|er
  --or--
  diagram create --template aws-3tier|gcp-microservices|azure-hub-spoke
  diagram list-templates
diagram markdown
diagram add-node | remove-node | update-node | add-edge | remove-edge | update-edge ...
diagram get-node | get-edge | list-nodes | list-edges | get-mermaid | set-mermaid ...
diagram mcp
```

`diagram render` picks output format from `--output` extension: `.svg`, `.png`, `.pdf`, `.txt`, or `.ascii`.

## Examples

Under `examples/`:

| File | Kind / format |
|------|----------------|
| `simple-flowchart.mmd`, `shapes.mmd`, `subgraphs.mmd`, … | Mermaid flowchart |
| `sequence.mmd`, `class.mmd`, `gantt.mmd`, `state.mmd`, `er.mmd` | Mermaid sequence / class / gantt / state / er |
| `sequence.puml`, `class.puml`, `activity.puml` | PlantUML |
| `simple-flowchart.d2`, `containers.d2` | D2 (flat + containers/subgraphs) |
| `simple-flowchart.dot` | Graphviz DOT |
| `simple-flowchart.drawio` | draw.io XML (uncompressed `<mxfile>`) |
| `multi-document.json` | Multi-diagram JSON IR |
| `doc-with-diagrams.md` | Markdown pipeline demo |
| `code-sample.rs` / `code-sample.ts` (tests/fixtures/) | Samples for `generate-class/tree/call` (Rust + TypeScript) |
| `embed/*` | Compact fixtures for Wasm/embed string API (all formats) |
| `wasm/` | Browser demo (`make wasm` → `pkg/`) |

## MCP (AI agents)

`diagram mcp` exposes parse, import/export, lossiness, render (SVG/PNG/PDF/ASCII), validate, diff/merge, metrics, markdown processing, **code→UML generation** (`generate_class_diagram` / `generate_tree_diagram` / `generate_call_diagram` for Rust/TypeScript), architecture templates, and graph edit tools over stdio. The MCP surface is the API agents should call instead of round-tripping diagram work through an LLM — same answer, zero tokens.

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

Native-only features (CLI, MCP, PNG/PDF, preview server, code→UML generation) are behind the default `native` Cargo feature; Wasm builds use `--no-default-features --features wasm`.

## Architecture

```
                        ┌── Code ──► tree-sitter ──► IR ──┐
                        │                                │
   Formats  ──import──► │          Canonical IR          │ ──export──►  Formats
 (Mermaid,   (Formats)  │           Document             │  (Formats)   (Mermaid,
 PlantUML,               │            └─ Diagram[]       │              PlantUML,
 DOT, D2,    ◄───────────┤                by Kind        ├──► DOT, D2, JSON IR)
 JSON IR)               │                                │
                        └─── IR ──► skeleton generator ──► Code ─┘
                              Render        Analyze       Generate
                              SVG/PNG/PDF   validate,     CLI / MCP
                              (native)      diff, metrics  (zero tokens)
```

See [`ARCHITECTURE.md`](ARCHITECTURE.md), [`CONTEXT.md`](CONTEXT.md), [`docs/adr/0001-canonical-ir-and-format-adapters.md`](docs/adr/0001-canonical-ir-and-format-adapters.md), and [`docs/adr/0002-non-ai-code-uml-conversion.md`](docs/adr/0002-non-ai-code-uml-conversion.md).

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
├── ascii.rs                                 # ASCII art render
├── diagram.rs / parser.rs / layout.rs / renderer.rs   # flowchart
├── sequence.rs / class.rs / gantt.rs / state.rs / er.rs
├── png.rs / pdf.rs                          # native feature
├── codegen/                                 # Code ↔ UML (tree-sitter + skeleton)
editors/vscode/                              # VS Code / Cursor extension
examples/wasm/                               # Wasm demo (pkg/ from `make wasm`)
```