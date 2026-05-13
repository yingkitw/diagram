# Specification

## Overview

`diagram` is a CLI tool and MCP server for manipulating Mermaid flowchart diagrams. It parses `.mmd` files into an in-memory graph model, allows inspection and modification, and can render to SVG.

## Supported Mermaid Syntax

### Flowcharts

- **Direction**: `graph TD`, `graph LR`, `graph RL`, `graph BT`
- **Nodes**:
  - `A[text]` — rectangle (default)
  - `A{text}` — diamond
  - `A{{text}}` — diamond (alt syntax)
  - `A(text)` — stadium / rounded
  - `A` — bare rectangle (id as text)
- **Edges**: `A --> B` (directed), `A -->|label| B` (labeled)
- **Comments**: `%% line comment`

### Not yet supported

- Subgraphs
- Styling (style, classDef)
- Edge types (`===`, `-.->`)
- Multi-character node IDs (only `[A-Za-z0-9_]`)

## CLI Interface

```
diagram <COMMAND>

Commands:
  parse        Parse and print JSON
  info         Show diagram summary
  render       Render as SVG
  mcp          Start MCP server (stdio)
  add-node     Add a node
  remove-node  Remove a node
  update-node  Update node text/shape
  add-edge     Add an edge
  remove-edge  Remove an edge
```

## MCP Tools

All tools accept `path` (path to `.mmd` file) plus operation-specific parameters:

| Tool | Parameters | Returns |
|------|-----------|---------|
| `parse_diagram` | path | JSON diagram |
| `get_info` | path | Summary JSON |
| `render_svg` | path | SVG string |
| `add_node` | path, id, text, shape? | Status JSON |
| `remove_node` | path, id | Status JSON |
| `update_node` | path, id, text?, shape? | Status JSON |
| `add_edge` | path, from, to, label? | Status JSON |
| `remove_edge` | path, from, to | Status JSON |
| `get_mermaid` | path | Mermaid source |
| `list_nodes` | path | Node list JSON |
| `list_edges` | path | Edge list JSON |

## Data Model

```rust
struct Diagram {
    rankdir: String,  // "TB", "LR", "RL", "BT"
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

struct Node {
    id: String,
    text: String,
    shape: NodeShape,  // Rect | Diamond | Stadium
}

struct Edge {
    from: String,
    to: String,
    label: String,
}
```

## Rendering

SVG output uses a layered graph layout algorithm:
1. Sources (no incoming edges) are assigned layer 0
2. BFS assigns subsequent layers
3. Nodes within each layer are positioned with even spacing
4. Edges are drawn as straight lines with arrow markers
5. Dark theme by default

## Transport

The MCP server uses stdio transport (JSON-RPC over stdin/stdout). Suitable for local AI assistant integration.
