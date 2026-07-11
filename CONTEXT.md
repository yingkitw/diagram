# Diagram Platform

A native Rust platform for rendering, generating, analyzing, and interchanging diagrams. Mermaid and PlantUML are compatibility formats — not the product identity.

## Language

**Document**:
The top-level unit of work: one or more diagrams of a known kind, optionally with metadata.
_Avoid_: File, chart, drawing (except when speaking of SVG output)

**Diagram**:
A single typed model (flowchart, sequence, class, gantt, …) independent of any concrete syntax.
_Avoid_: Mermaid file, graph (unless graph-theoretic analysis)

**Kind**:
The diagram type (flowchart, sequence, class, gantt, state, ER, …).
_Avoid_: Format, dialect

**Format**:
A concrete serialization (Mermaid, PlantUML, Graphviz DOT, D2, native JSON IR, SVG, PNG, PDF).
_Avoid_: Language, dialect (prefer Format for interchange; Kind for semantics)

**IR (Intermediate Representation)**:
The canonical in-memory / JSON form of a Document or Diagram that all formats import into and export from.
_Avoid_: AST (unless discussing a specific parser), model blob

**Adapter**:
A bidirectional (or unidirectional) bridge between a Format and the IR.
_Avoid_: Plugin (reserved for renderer/shape extensions), converter (too vague)

**Render**:
Produce a visual artifact (SVG today; PNG/PDF later) from the IR.
_Avoid_: Draw, compile (unless talking about layout)

**Generate**:
Create or extend diagrams from structured input, templates, or agent tools — not only by editing source text.
_Avoid_: Create (too vague), hallucinate

**Analyze**:
Compute structural facts about a Diagram (validation, metrics, diffs, cycles, complexity) without requiring a render.
_Avoid_: Lint (subset of Analyze), inspect (CLI verb is fine)

**Interchange**:
Import from and export to external Formats while preserving as much semantics as the target allows.
_Avoid_: Convert-only (implies lossy one-shot), sync

**Compatibility**:
Best-effort roundtrip with Mermaid/PlantUML (and peers) so existing docs and tools keep working.
_Avoid_: Parity (implies 100% feature match — we aim for useful Compatibility, not clone parity)
