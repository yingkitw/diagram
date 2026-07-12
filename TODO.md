# TODO

## Vision

Platform pillars (see `CONTEXT.md`, `ARCHITECTURE.md`, ADR-0001):

1. **Render** — layout → SVG/PNG/PDF, Chromium-free
2. **Generate** — structured create/edit (CLI + MCP)
3. **Analyze** — validate, diff, merge, metrics on IR
4. **Interchange** — import/export via canonical IR; Mermaid/PlantUML Compatibility

Competitive edge vs Mermaid.js / PlantUML: native binary, MCP-first agents, analysis without browser/JVM, multi-format IR — not “another Mermaid clone.”

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
- [ ] Plugin API for custom shapes / render backends
- [x] Wasm embed for browser preview without local server
- [x] Semantic diff v1 (IR-level `DocumentDiff` for all kinds incl. state/er; multi-diagram)

## Brainstorming (competitive)

| Advantage we want | vs Mermaid | vs PlantUML |
|-------------------|------------|-------------|
| Native speed / small install | No Node/Chromium | No JVM |
| MCP agent generate+analyze | Weak / external | Weak / external |
| Multi-format IR hub | Mermaid-centric | PlantUML-centric |
| Structural analysis API | Limited | Limited |
| Docs CI without heavy runtimes | mmdc heavy | Docker/Java common |

Prioritize next: plugin API / further format depth as demand warrants.

## Audit follow-ups (done)

- [x] IR-aware flowchart mutate gate (`load_flowchart`) + kind-aware validate
- [x] MCP D2 in schemars/errors; get/set Mermaid via IR
- [x] Integration roundtrips assert notes/fragments/stereotypes/cardinality/milestones
- [x] Class mmd↔puml interchange test; analyze cardinality/stereotype/milestone keys
- [x] SPEC kind list + MCP format docs sync
- [x] Deeper DOT/D2: D2 container↔subgraph roundtrip; DOT fillcolor/color/fontcolor/URL
