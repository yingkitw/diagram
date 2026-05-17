use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeShape {
    Rect,
    Diamond,
    Stadium,
    Hexagon,
    Cylinder,
    Circle,
}

impl NodeShape {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rect" | "rectangle" => Some(Self::Rect),
            "diamond" => Some(Self::Diamond),
            "stadium" => Some(Self::Stadium),
            "hexagon" => Some(Self::Hexagon),
            "cylinder" => Some(Self::Cylinder),
            "circle" => Some(Self::Circle),
            _ => None,
        }
    }
}

impl fmt::Display for NodeShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rect => write!(f, "rect"),
            Self::Diamond => write!(f, "diamond"),
            Self::Stadium => write!(f, "stadium"),
            Self::Hexagon => write!(f, "hexagon"),
            Self::Cylinder => write!(f, "cylinder"),
            Self::Circle => write!(f, "circle"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EdgeStyle {
    Arrow,
    Dashed,
    Thick,
}

impl EdgeStyle {
    pub fn arrow_str(&self) -> &str {
        match self {
            Self::Arrow => "-->",
            Self::Dashed => "-.->",
            Self::Thick => "==>",
        }
    }
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self::Arrow
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub text: String,
    pub shape: NodeShape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
    #[serde(default)]
    pub style: EdgeStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subgraph {
    pub id: String,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStyle {
    pub node_id: String,
    pub properties: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDef {
    pub name: String,
    pub properties: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassApply {
    pub node_ids: Vec<String>,
    pub class_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    pub rankdir: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    pub styles: Vec<NodeStyle>,
    pub class_defs: Vec<ClassDef>,
    pub class_applies: Vec<ClassApply>,
}

pub fn format_id(id: &str) -> String {
    if id.chars().all(|c| c.is_alphanumeric() || c == '_') && !id.is_empty() {
        id.to_string()
    } else {
        format!("\"{}\"", id.replace('"', "\\\""))
    }
}

impl Diagram {
    pub fn new(rankdir: &str) -> Self {
        Self {
            rankdir: rankdir.to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            styles: Vec::new(),
            class_defs: Vec::new(),
            class_applies: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) -> Result<(), String> {
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(format!("node '{}' already exists", node.id));
        }
        self.nodes.push(node);
        Ok(())
    }

    pub fn remove_node(&mut self, id: &str) {
        self.nodes.retain(|n| n.id != id);
        self.edges.retain(|e| e.from != id && e.to != id);
    }

    pub fn update_node(
        &mut self,
        id: &str,
        text: Option<&str>,
        shape: Option<NodeShape>,
    ) -> Result<(), String> {
        let node = self
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or_else(|| format!("node '{}' not found", id))?;
        if let Some(t) = text {
            node.text = t.to_string();
        }
        if let Some(s) = shape {
            node.shape = s;
        }
        Ok(())
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), String> {
        if !self.nodes.iter().any(|n| n.id == edge.from) {
            return Err(format!("node '{}' not found", edge.from));
        }
        if !self.nodes.iter().any(|n| n.id == edge.to) {
            return Err(format!("node '{}' not found", edge.to));
        }
        self.edges.push(edge);
        Ok(())
    }

    pub fn remove_edge(&mut self, from: &str, to: &str) -> Result<(), String> {
        let before = self.edges.len();
        self.edges.retain(|e| e.from != from || e.to != to);
        if self.edges.len() == before {
            return Err(format!("edge '{} -> {}' not found", from, to));
        }
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn to_mermaid(&self) -> String {
        let mut out = format!("graph {}\n", self.rankdir);
        for n in &self.nodes {
            let id = format_id(&n.id);
            let t = &n.text;
            match n.shape {
                NodeShape::Rect => out.push_str(&format!("    {}[{}]\n", id, t)),
                NodeShape::Diamond => out.push_str(&format!("    {}{{{}}}\n", id, t)),
                NodeShape::Stadium => out.push_str(&format!("    {}({})\n", id, t)),
                NodeShape::Hexagon => out.push_str(&format!("    {}{{{{{}}}}}\n", id, t)),
                NodeShape::Cylinder => out.push_str(&format!("    {}[({})]\n", id, t)),
                NodeShape::Circle => out.push_str(&format!("    {}(({}))\n", id, t)),
            }
        }
        for e in &self.edges {
            let arrow = e.style.arrow_str();
            let from = format_id(&e.from);
            let to = format_id(&e.to);
            if e.label.is_empty() {
                out.push_str(&format!("    {} {} {}\n", from, arrow, to));
            } else {
                out.push_str(&format!("    {} {}|{}| {}\n", from, arrow, e.label, to));
            }
        }
        for sg in &self.subgraphs {
            out.push_str(&format!("    subgraph {}\n", format_id(&sg.id)));
            for nid in &sg.nodes {
                out.push_str(&format!("        {}\n", format_id(nid)));
            }
            out.push_str("    end\n");
        }
        for s in &self.styles {
            out.push_str(&format!("    style {} {}\n", format_id(&s.node_id), s.properties));
        }
        for cd in &self.class_defs {
            out.push_str(&format!("    classDef {} {}\n", cd.name, cd.properties));
        }
        for ca in &self.class_applies {
            let ids: Vec<String> = ca.node_ids.iter().map(|id| format_id(id)).collect();
            out.push_str(&format!("    class {} {}\n", ids.join(","), ca.class_name));
        }
        out.trim().to_string()
    }
}
