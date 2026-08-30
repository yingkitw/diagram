# TODO

## Vision

`diagram` is a **non-AI code ↔ UML conversion platform**. The mission is to make every diagram ↔ source roundtrip deterministic, zero-token, and local — so agents and humans don't pay model cost or latency for diagram work a tree-sitter grammar can do in microseconds. See `CONTEXT.md`, `ARCHITECTURE.md`, ADR-0001, ADR-0002.

Platform pillars (in priority order):

1. **Code ↔ UML** — bidirectional, deterministic, zero-token
   - Code → UML: tree-sitter extraction for class / tree / call (Rust, TypeScript; pluggable)
   - UML → Code: skeleton generator (Rust, TypeScript) — planned
2. **Interchange** — IR ↔ Mermaid / PlantUML / DOT / D2 / JSON IR
3. **Render** — IR → SVG / PNG / PDF, Chromium-free
4. **Analyze** — validate, diff, merge, metrics on IR
5. **Generate (graph edit)** — CLI + MCP mutations against IR

Competitive edge vs Mermaid.js / PlantUML / AI diagram tools: native binary, MCP-first agents, zero tokens, sub-millisecond extraction, multi-format IR — not "another Mermaid clone".

## Completed (foundation)

- [x] Flowchart IR + Mermaid parse/render (shapes, edges, subgraphs, styles, themes)
- [x] Sequence / class / gantt Mermaid MVP parse → SVG
- [x] CLI + MCP generate/edit (flowchart), validate, diff/merge
- [x] Live preview, interactive SVG (href/tooltip), watch mode
- [x] Product reposition: CONTEXT, ADR-0001, architecture target (IR + adapters)
- [x] Canonical IR spine (`ir`, `formats`) + import/export CLI/MCP

## Short-term (platform spine)

- [x] Canonical **IR**: `Document` / `Diagram` / `Kind` + JSON (`diagram ir`, `parse`, `import`/`export`)
- [x] **Format detection** + `import` / `export` CLI + MCP tools
- [x] Analysis pack v1: metrics (node/edge counts, depth, cycle list, orphan rate) as JSON
- [x] Fold Mermaid parsers behind `formats::mermaid` (behavior-preserving move)
- [x] Generation: kind-aware `create` scaffold (`diagram create --kind flowchart|sequence|class|gantt|state|er`)

## Medium-term (interchange + render)

- [x] PlantUML adapter (sequence, class, activity import/export)
- [x] Graphviz DOT import + export (digraph subset) ↔ flowchart IR
- [x] PNG export (resvg; `.png` output via `diagram render`)
- [x] PDF export (`.pdf` via `diagram render`; vector via svg2pdf)
- [x] Multi-diagram Document (several kinds / figures per file)
- [x] Markdown pipeline: extract fenced blocks → render → rewrite links
- [x] Lossiness report on export (what could not be represented)

## Longer-term

- [x] D2 adapter (flat flowchart import/export)
- [ ] Excalidraw / Kroki-adjacent adapters as demand warrants
- [x] Vector PDF export (svg2pdf; replaces prior raster embed)
- [x] State + ER kinds (IR + Mermaid Compatibility)
- [x] VS Code extension (preview / validate / render via CLI)
- [x] Sequence extras: notes (`left of` / `right of` / `over`) + self-messages
- [x] PlantUML sequence note import/export (one-liner + multiline + over)
- [x] Class stereotypes (`<<interface>>` / PlantUML `interface`/`enum`/`abstract`)
- [x] Gantt milestones (`milestone` + diamond render)
- [x] Sequence loops / alt / opt fragments
- [x] Class relation cardinality (`"1" --> "*"`, PlantUML roundtrip)
- [x] Class generics (`Stack~T~` / PlantUML `Stack<T>`)
- [x] Class notes (`note for Class "…"`; PlantUML `note for` / left|right of)
- [x] Architecture templates (`create --template aws-3tier|gcp-microservices|azure-hub-spoke`) — deterministic, zero-token (learned from graphine)
- [x] ASCII art render (`diagram render … --output .txt`) — deterministic, zero-token, for READMEs/terminal (learned from graphine)
- [x] draw.io XML adapter (import/export) — uncompressed `<mxfile>` subset, flowchart IR (learned from graphine)
- [ ] Plugin API for custom shapes / render backends
- [x] Wasm embed for browser preview without local server
- [x] Semantic diff v1 (IR-level `DocumentDiff` for all kinds incl. state/er; multi-diagram)
- [x] **Code → UML** (tree-sitter): `generate class|tree|call <file>` — Rust, TypeScript; CLI + MCP
- [x] **UML → Code** (skeleton generator): `generate-skeleton <diagram> --lang rust|typescript` — class / flowchart / sequence / state / ER; CLI + MCP
- [x] Roundtrip stability: Source → IR → Skeleton → IR matches Source → IR for type-level classes (see `integration_tests::test_code_uml_skeleton_roundtrip_*`)
- [ ] Pluggable tree-sitter grammar: `Language` trait so new languages slot in without touching the IR pipeline

## Brainstorming (competitive)

| Advantage we want | vs Mermaid | vs PlantUML | vs AI helpers |
|-------------------|------------|-------------|---------------|
| Native speed / small install | No Node/Chromium | No JVM | No model latency |
| Zero-token code↔UML | None | None | The whole pitch |
| MCP agent generate+analyze | Weak / external | Weak / external | Slow + costly |
| Multi-format IR hub | Mermaid-centric | PlantUML-centric | Single-format usually |
| Structural analysis API | Limited | Limited | None |
| Docs CI without heavy runtimes | mmdc heavy | Docker/Java common | API-key gated |

Features learned from graphine (re-implemented as non-AI / deterministic / zero-token where possible):

- [x] **Architecture templates** (`aws-3tier`, `gcp-microservices`, `azure-hub-spoke`)
- [x] **ASCII art export** (IR → text; no canvas/rasterizer)
- [x] **draw.io XML adapter** (import + export)
- [ ] **IaC ↔ IR** — Terraform / CDK / Pulumi / Bicep / CloudFormation → IR and IR → IaC skeletons (extends the bidirectional code↔UML story to infrastructure)
- [ ] **Force-directed layout** (renderer; improve on layered BFS)
- [ ] **Color-coded SVG diff render** (visual diff output)
- [ ] **Deterministic security/cost analysis on IR** (detect public S3, estimate resources)
- [ ] **Cloud-icon shapes in renderer** (AWS/GCP/Azure shape support)

Prioritize next: pluggable tree-sitter grammars (broadens Code → UML reach) and deterministic IaC ↔ IR (Terraform / CDK / CloudFormation — deferred from graphine competitive learn, closes the infrastructure ↔ UML bidirectional story).

## Audit follow-ups (done)

- [x] IR-aware flowchart mutate gate (`load_flowchart`) + kind-aware validate
- [x] MCP D2 in schemars/errors; get/set Mermaid via IR
- [x] Integration roundtrips assert notes/fragments/stereotypes/cardinality/milestones
- [x] Class mmd↔puml interchange test; analyze cardinality/stereotype/milestone keys
- [x] SPEC kind list + MCP format docs sync
- [x] Deeper DOT/D2: D2 container↔subgraph roundtrip; DOT fillcolor/color/fontcolor/URL
- [x] Clippy clean under `-D warnings`; CLI/MCP code-gen dispatch deduped into `codegen::write_to_path`