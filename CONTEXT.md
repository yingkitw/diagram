# Diagram Platform

A native Rust platform for **non-AI code ↔ UML conversion** — bidirectional, deterministic, fast, and free of model/LLM tokens. UML formats (Mermaid, PlantUML, Graphviz DOT, D2, JSON IR) are compatibility surfaces; the canonical IR is the spine.

## Why this exists

Diagrams and source code drift apart when humans have to keep both in sync by hand, and AI assistants make it expensive in tokens and slow in latency. `diagram` closes the loop with **deterministic** extraction (tree-sitter for code → IR) and **deterministic** generation (IR → skeleton source) so every conversion is local, instant, and free of API calls.

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
Produce a visual artifact (SVG, PNG, or vector PDF) from the IR.
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

**Lossiness**:
Explicit report of IR fields or semantics that a target Format cannot represent on export.
_Avoid_: Silent data loss, "full fidelity" claims without tests

**Code**:
Source text in a programming language (Rust, TypeScript today; pluggable via tree-sitter grammars).
_Avoid_: Program (too broad), source code (redundant)

**UML / Diagram kind**:
A typed visual model — flowchart, class, sequence, gantt, state, ER — that documents a slice of software.
_Avoid_: Diagram (overlaps with our `Diagram` IR term; qualify as "UML diagram" when contrasting with code)

**Conversion (code ↔ UML)**:
The bidirectional translation between **Code** and **UML**, mediated by the canonical IR.
- `Code → UML`: deterministic extraction via tree-sitter (`generate-class|tree|call`); produces a Document the same way any Format would.
- `UML → Code`: deterministic generation of a skeleton source file (`generate-skeleton`); bodies are stubs, signatures are real.
_Avoid_: Sync (implies polling), AI-generation (we are explicitly non-AI), transpile (implies source→source)

**Skeleton**:
A compilable stub of source code produced from an IR — class/struct/enum/trait signatures, function names with empty bodies, no logic. It is the "↔" half of code ↔ UML.
_Avoid_: Template (overlaps with our `template` notion elsewhere), scaffold (already used for new-diagram scaffolds)

**Non-AI / Deterministic**:
All conversion is performed by local grammars and code, never by a language model. Every run on the same input produces the same output; no network, no tokens, no latency budget.
_Avoid_: Heuristic (we are explicit; heuristics are fine when labelled), "smart" (implies LLM)