# diagram

A Rust CLI and MCP (Model Context Protocol) server for manipulating Mermaid diagrams. Parse, inspect, modify, and render `.mmd` files from the command line or through any MCP-compatible AI assistant.

## Usage

### CLI

```bash
# Parse a diagram and print as JSON
diagram parse sample.mmd

# Show diagram summary
diagram info sample.mmd

# Render diagram as SVG (stdout or --output)
diagram render sample.mmd
diagram render sample.mmd --output out.svg

# Watch mode: auto-re-render when file changes
diagram render sample.mmd --output out.svg --watch

# Manipulate nodes and edges
diagram add-node sample.mmd X "New Node"
diagram add-node sample.mmd X "New Node" --shape stadium
diagram update-node sample.mmd X --text "Updated"
diagram remove-node sample.mmd X
diagram add-edge sample.mmd A X --label "connects to"
diagram add-edge sample.mmd A X --style dashed
diagram remove-edge sample.mmd A X
diagram update-edge sample.mmd A X --style thick
diagram get-node sample.mmd A
diagram get-edge sample.mmd A X
diagram validate sample.mmd

# Start MCP server
diagram mcp
```

### MCP (AI Assistant Integration)

Start the MCP server with `diagram mcp`. It communicates over stdio using the Model Context Protocol.

**Available tools:**

| Tool | Description |
|------|-------------|
| `parse_diagram` | Parse a .mmd file and return JSON |
| `get_info` | Diagram summary (node/edge count, shapes) |
| `render_svg` | Render diagram as SVG |
| `add_node` | Add a node (id, text, optional shape) |
| `remove_node` | Remove a node and its edges |
| `update_node` | Update node text/shape |
| `add_edge` | Add an edge (from, to, optional label) |
| `remove_edge` | Remove an edge |
| `get_mermaid` | Get the mermaid source code |
| `set_mermaid` | Write raw mermaid source to file |
| `list_nodes` | List all nodes |
| `list_edges` | List all edges |
| `update_edge` | Update edge label/style |
| `get_node` | Get a single node by ID |
| `get_edge` | Get a single edge by from/to |
| `validate_diagram` | Validate for orphans/dangling edges/cycles |
| `add_nodes` | Add multiple nodes at once |
| `add_edges` | Add multiple edges at once |

### MCP Client Configuration

**Claude Desktop:**
```json
{
  "mcpServers": {
    "diagram": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "/path/to/diagram-rs/Cargo.toml", "--", "mcp"]
    }
  }
}
```

## Testing

```bash
cargo test
```

Includes parser unit tests, integration tests for all `examples/` files, CLI subprocess tests, and roundtrip fidelity checks.

## Installation

```bash
cargo install --path .
```

## Dependencies

- [rmcp](https://crates.io/crates/rmcp) — Rust MCP SDK (server + macros)
- [clap](https://crates.io/crates/clap) — CLI argument parsing
- [serde](https://crates.io/crates/serde) / serde_json — serialization
- [tokio](https://crates.io/crates/tokio) — async runtime
- [schemars](https://crates.io/crates/schemars) — JSON schema generation for MCP tools

## Project Structure

```
src/
├── main.rs      # Entry point
├── cli.rs       # CLI subcommands
├── mcp.rs       # MCP server & tools
├── diagram.rs   # Core data model
├── parser.rs    # Mermaid parser
├── layout.rs    # Graph layout algorithm
└── renderer.rs  # SVG generation
```
