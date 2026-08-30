//! Kind-aware diagram scaffolds for CLI and MCP generation.

use crate::formats::{self, Format};
use crate::ir::{self, Document, Kind, IrError};

/// Parse a kind name from CLI/MCP input.
pub fn parse_kind(s: &str) -> Result<Kind, String> {
    match s.to_lowercase().as_str() {
        "flowchart" | "graph" => Ok(Kind::Flowchart),
        "sequence" | "sequencediagram" => Ok(Kind::Sequence),
        "class" | "classdiagram" => Ok(Kind::Class),
        "gantt" => Ok(Kind::Gantt),
        "state" | "statediagram" => Ok(Kind::State),
        "er" | "erdiagram" => Ok(Kind::Er),
        _ => Err(format!(
            "unknown kind '{s}'; expected flowchart, sequence, class, gantt, state, or er"
        )),
    }
}

/// Minimal Mermaid scaffold for each diagram kind.
pub fn mermaid_scaffold(kind: Kind) -> &'static str {
    match kind {
        Kind::Flowchart => "graph TD\n    A[Start] --> B[End]\n",
        Kind::Sequence => {
            "sequenceDiagram\n    participant A\n    participant B\n    A->>B: Message\n"
        }
        Kind::Class => {
            "classDiagram\n    class Example {\n        +field\n        +method()\n    }\n"
        }
        Kind::Gantt => {
            "gantt\n    title Project Plan\n    dateFormat YYYY-MM-DD\n    section Phase 1\n    Task :a1, 2024-01-01, 7d\n"
        }
        Kind::State => {
            "stateDiagram-v2\n    [*] --> Idle\n    Idle --> [*]\n"
        }
        Kind::Er => {
            "erDiagram\n    CUSTOMER ||--o{ ORDER : places\n    CUSTOMER {\n        string name\n        string custNumber PK\n    }\n    ORDER {\n        int orderNumber PK\n    }\n"
        }
    }
}

/// Minimal PlantUML sequence scaffold.
pub fn plantuml_sequence_scaffold() -> &'static str {
    "@startuml\nparticipant A\nparticipant B\nA -> B: Message\n@enduml\n"
}

/// Minimal DOT digraph scaffold for flowcharts.
pub fn dot_scaffold() -> &'static str {
    "digraph G {\n    A [label=\"Start\"];\n    B [label=\"End\"];\n    A -> B;\n}\n"
}

/// Minimal D2 scaffold for flowcharts.
pub fn d2_scaffold() -> &'static str {
    "direction: down\nstart: Start\nend: End\nstart -> end\n"
}

/// Build a canonical Document from a kind scaffold.
pub fn scaffold_document(kind: Kind) -> Result<Document, IrError> {
    ir::from_mermaid(mermaid_scaffold(kind))
}

/// Architecture diagram templates (deterministic, zero-token scaffolds).
///
/// Templates are Mermaid flowcharts parsed into the canonical IR and then
/// exported through the standard `formats::export_path` pipeline, so a single
/// template can be written in any supported format (Mermaid, JSON IR, DOT, D2,
/// PlantUML activity). Inspired by graphine's pre-built architecture templates,
/// but generated deterministically without any model call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    Aws3Tier,
    GcpMicroservices,
    AzureHubSpoke,
}

impl Template {
    /// Parse a template id from CLI/MCP input.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().replace(['_', ' '], "-").as_str() {
            "aws-3tier" | "aws3tier" | "aws-3-tier" => Ok(Self::Aws3Tier),
            "gcp-microservices" | "gcp-microservice" | "gcp-micro" => Ok(Self::GcpMicroservices),
            "azure-hub-spoke" | "azure-hubspoke" | "azure-hub" => Ok(Self::AzureHubSpoke),
            _ => Err(format!(
                "unknown template '{s}'; expected aws-3tier, gcp-microservices, or azure-hub-spoke"
            )),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aws3Tier => "aws-3tier",
            Self::GcpMicroservices => "gcp-microservices",
            Self::AzureHubSpoke => "azure-hub-spoke",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Aws3Tier => "AWS 3-tier web architecture (ELB → EC2 web → EC2 app → RDS)",
            Self::GcpMicroservices => "GCP microservices (LB → GKE services → Cloud SQL / Pub/Sub)",
            Self::AzureHubSpoke => "Azure hub-spoke network (hub VNet → spoke VNets → peering)",
        }
    }

    /// Mermaid flowchart source for the template.
    pub fn mermaid(self) -> &'static str {
        match self {
            Self::Aws3Tier => {
                "graph TD\n\
                 subgraph Edge\n\
                     CDN[CloudFront CDN]\n\
                 end\n\
                 subgraph Web\n\
                     ELB[Application Load Balancer]\n\
                     Web1[EC2 Web Server A]\n\
                     Web2[EC2 Web Server B]\n\
                 end\n\
                 subgraph App\n\
                     App1[EC2 App Server A]\n\
                     App2[EC2 App Server B]\n\
                 end\n\
                 subgraph Data\n\
                     RDS[(RDS Primary)]\n\
                     Replica[(RDS Read Replica)]\n\
                 end\n\
                 CDN --> ELB\n\
                 ELB --> Web1\n\
                 ELB --> Web2\n\
                 Web1 --> App1\n\
                 Web2 --> App2\n\
                 App1 --> RDS\n\
                 App2 --> RDS\n\
                 RDS --> Replica\n"
            }
            Self::GcpMicroservices => {
                "graph TD\n\
                 subgraph Edge\n\
                     LB[Cloud Load Balancer]\n\
                 end\n\
                 subgraph GKE\n\
                     Gateway[API Gateway]\n\
                     Auth[Auth Service]\n\
                     Orders[Orders Service]\n\
                     Billing[Billing Service]\n\
                 end\n\
                 subgraph Data\n\
                     Sql[(Cloud SQL)]\n\
                     Pubsub[(Pub/Sub)]\n\
                 end\n\
                 LB --> Gateway\n\
                 Gateway --> Auth\n\
                 Gateway --> Orders\n\
                 Gateway --> Billing\n\
                 Orders --> Sql\n\
                 Billing --> Sql\n\
                 Orders --> Pubsub\n\
                 Billing --> Pubsub\n"
            }
            Self::AzureHubSpoke => {
                "graph TD\n\
                 subgraph Hub\n\
                     Firewall[Azure Firewall]\n\
                     Vpn[VPN Gateway]\n\
                 end\n\
                 subgraph Spoke1\n\
                     Vm1[Spoke1 VM]\n\
                     Peering1[Spoke1 Peering]\n\
                 end\n\
                 subgraph Spoke2\n\
                     Vm2[Spoke2 VM]\n\
                     Peering2[Spoke2 Peering]\n\
                 end\n\
                 Vpn --> Firewall\n\
                 Firewall --> Peering1\n\
                 Firewall --> Peering2\n\
                 Peering1 --> Vm1\n\
                 Peering2 --> Vm2\n"
            }
        }
    }
}

/// All built-in templates, in a stable display order.
pub fn all_templates() -> [Template; 3] {
    [Template::Aws3Tier, Template::GcpMicroservices, Template::AzureHubSpoke]
}

/// Build a canonical Document from a template.
pub fn template_document(template: Template) -> Result<Document, IrError> {
    ir::from_mermaid(template.mermaid())
}

/// Write a template scaffold to a path in any supported format (refuses to overwrite).
pub fn write_template(template_str: &str, path: &str) -> Result<Template, String> {
    if std::path::Path::new(path).exists() {
        return Err(format!("refusing to overwrite existing file '{path}'"));
    }
    let template = Template::parse(template_str)?;
    let doc = template_document(template).map_err(|e| e.to_string())?;
    formats::export_path(&doc, path, None).map_err(|e| e.to_string())?;
    Ok(template)
}

/// Write a new scaffold file (refuses to overwrite). Format follows output extension.
pub fn write_scaffold(kind_str: &str, path: &str) -> Result<Kind, String> {
    if std::path::Path::new(path).exists() {
        return Err(format!("refusing to overwrite existing file '{path}'"));
    }
    let kind = parse_kind(kind_str)?;
    let format = formats::detect("", Some(path));
    match format {
        Format::JsonIr => {
            let doc = scaffold_document(kind).map_err(|e| e.to_string())?;
            formats::export_path(&doc, path, Some(Format::JsonIr)).map_err(|e| e.to_string())?;
        }
        Format::Mermaid => {
            std::fs::write(path, mermaid_scaffold(kind))
                .map_err(|e| format!("Failed to write '{path}': {e}"))?;
        }
        Format::Dot => {
            if kind != Kind::Flowchart {
                return Err(format!(
                    "DOT scaffolds only support flowchart (got {kind})"
                ));
            }
            std::fs::write(path, dot_scaffold())
                .map_err(|e| format!("Failed to write '{path}': {e}"))?;
        }
        Format::D2 => {
            if kind != Kind::Flowchart {
                return Err(format!(
                    "D2 scaffolds only support flowchart (got {kind})"
                ));
            }
            std::fs::write(path, d2_scaffold())
                .map_err(|e| format!("Failed to write '{path}': {e}"))?;
        }
        Format::PlantUml => {
            if kind != Kind::Sequence {
                return Err(format!(
                    "PlantUML scaffolds only support sequence (got {kind})"
                ));
            }
            std::fs::write(path, plantuml_sequence_scaffold())
                .map_err(|e| format!("Failed to write '{path}': {e}"))?;
        }
        Format::DrawIo => {
            // draw.io is flowchart-only; route through the IR export pipeline so
            // the lossiness gate rejects non-flowchart kinds with a clear message.
            if kind != Kind::Flowchart {
                return Err(format!(
                    "draw.io scaffolds only support flowchart (got {kind})"
                ));
            }
            let doc = scaffold_document(kind).map_err(|e| e.to_string())?;
            formats::export_path(&doc, path, Some(Format::DrawIo)).map_err(|e| e.to_string())?;
        }
    }
    Ok(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffolds_parse_for_all_kinds() {
        for kind in [
            Kind::Flowchart,
            Kind::Sequence,
            Kind::Class,
            Kind::Gantt,
            Kind::State,
            Kind::Er,
        ] {
            let doc = scaffold_document(kind).unwrap();
            assert_eq!(doc.primary().unwrap().kind(), kind);
        }
    }

    #[test]
    fn write_mermaid_scaffold() {
        let path = std::env::temp_dir().join(format!("diagram_create_{}.mmd", std::process::id()));
        let p = path.to_str().unwrap();
        let kind = write_scaffold("flowchart", p).unwrap();
        assert_eq!(kind, Kind::Flowchart);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("graph TD"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_json_scaffold() {
        let path = std::env::temp_dir().join(format!("diagram_create_{}.json", std::process::id()));
        let p = path.to_str().unwrap();
        let kind = write_scaffold("sequence", p).unwrap();
        assert_eq!(kind, Kind::Sequence);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("\"kind\": \"sequence\"") || body.contains("\"kind\":\"sequence\""));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn templates_parse_to_flowchart() {
        for t in all_templates() {
            let doc = template_document(t).unwrap();
            assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
        }
    }

    #[test]
    fn template_parse_aliases() {
        assert_eq!(Template::parse("aws-3tier").unwrap(), Template::Aws3Tier);
        assert_eq!(Template::parse("AWS 3 Tier").unwrap(), Template::Aws3Tier);
        assert_eq!(Template::parse("gcp_microservices").unwrap(), Template::GcpMicroservices);
        assert_eq!(Template::parse("azure-hubspoke").unwrap(), Template::AzureHubSpoke);
        assert!(Template::parse("nope").is_err());
    }

    #[test]
    fn write_template_mermaid_and_json() {
        let mmd = std::env::temp_dir().join(format!("diagram_tpl_{}.mmd", std::process::id()));
        let p = mmd.to_str().unwrap();
        let t = write_template("aws-3tier", p).unwrap();
        assert_eq!(t, Template::Aws3Tier);
        let body = std::fs::read_to_string(&mmd).unwrap();
        assert!(body.contains("CloudFront") || body.contains("Cloudfront"));
        let _ = std::fs::remove_file(&mmd);

        let json = std::env::temp_dir().join(format!("diagram_tpl_{}.json", std::process::id()));
        let pj = json.to_str().unwrap();
        write_template("gcp-microservices", pj).unwrap();
        let body = std::fs::read_to_string(&json).unwrap();
        assert!(body.contains("\"kind\": \"flowchart\"") || body.contains("\"kind\":\"flowchart\""));
        let _ = std::fs::remove_file(&json);
    }
}
