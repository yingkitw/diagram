# Diagram guide

This page embeds diagrams rendered by `diagram markdown` (supports fenced `mermaid`, `plantuml`, and `dot` blocks).

## Flowchart

```mermaid
graph TD
    A[Start] --> B[End]
```

## Sequence

```mermaid
sequenceDiagram
    participant Alice
    participant Bob
    Alice->>Bob: Hello
    Bob-->>Alice: Hi
```

Regular code is left alone:

```rust
fn main() {}
```
