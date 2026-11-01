//! Shape catalog for the desktop GUI: geometry kinds, categories, and the
//! static list of palette entries (Basic, Flowchart, Data, Network, Security,
//! UML, AWS / Azure / GCP). Also owns the positioned SVG emitters, so the
//! export path and the canvas share one geometry definition.

use serde::{Deserialize, Serialize};

use crate::diagram::NodeShape;

/// Geometry used to draw a palette shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeKind {
    Rect,
    Rounded,
    Stadium,
    Diamond,
    Hexagon,
    Circle,
    Triangle,
    Parallelogram,
    Trapezoid,
    Cylinder,
    Cloud,
    Document,
    Actor,
    Component,
    Package,
    Shield,
    DataStore,
}

impl ShapeKind {
    /// Best-fit mapping onto the flowchart IR shape subset (lossy for kinds
    /// the IR does not model — they degrade to `Rect`).
    pub fn to_ir(self) -> NodeShape {
        match self {
            Self::Diamond => NodeShape::Diamond,
            Self::Stadium => NodeShape::Stadium,
            Self::Hexagon => NodeShape::Hexagon,
            Self::Cylinder => NodeShape::Cylinder,
            Self::Circle => NodeShape::Circle,
            _ => NodeShape::Rect,
        }
    }
}

/// Palette grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Basic,
    Flowchart,
    Data,
    Network,
    Security,
    Uml,
    Aws,
    Azure,
    Gcp,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::Flowchart => "Flowchart",
            Self::Data => "Data",
            Self::Network => "Network",
            Self::Security => "Security",
            Self::Uml => "UML",
            Self::Aws => "AWS",
            Self::Azure => "Azure",
            Self::Gcp => "GCP",
        }
    }

    pub fn all() -> [Category; 9] {
        [
            Self::Basic,
            Self::Flowchart,
            Self::Data,
            Self::Network,
            Self::Security,
            Self::Uml,
            Self::Aws,
            Self::Azure,
            Self::Gcp,
        ]
    }
}

/// One palette entry.
#[derive(Debug, Clone, Copy)]
pub struct ShapeDef {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub kind: ShapeKind,
    pub w: f32,
    pub h: f32,
    pub fill: [u8; 3],
    pub stroke: [u8; 3],
    /// Provider badge drawn in the top-left corner (e.g. "AWS").
    pub badge: Option<&'static str>,
}

const INK: [u8; 3] = [31, 35, 40];
const WHITE: [u8; 3] = [255, 255, 255];
const DATA_FILL: [u8; 3] = [246, 248, 250];
const DATA_STROKE: [u8; 3] = [87, 96, 106];
const NET_FILL: [u8; 3] = [232, 245, 255];
const NET_STROKE: [u8; 3] = [9, 105, 218];
const SEC_FILL: [u8; 3] = [255, 248, 229];
const SEC_STROKE: [u8; 3] = [154, 103, 0];
const UML_FILL: [u8; 3] = [245, 240, 255];
const UML_STROKE: [u8; 3] = [102, 60, 200];
const AWS_FILL: [u8; 3] = [255, 243, 224];
const AWS_STROKE: [u8; 3] = [230, 115, 0];
const AZ_FILL: [u8; 3] = [229, 241, 251];
const AZ_STROKE: [u8; 3] = [0, 120, 212];
const GCP_FILL: [u8; 3] = [232, 240, 254];
const GCP_STROKE: [u8; 3] = [26, 115, 232];

const fn basic(id: &'static str, name: &'static str, kind: ShapeKind, w: f32, h: f32) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Basic,
        kind,
        w,
        h,
        fill: WHITE,
        stroke: INK,
        badge: None,
    }
}

const fn flow(id: &'static str, name: &'static str, kind: ShapeKind, w: f32, h: f32) -> ShapeDef {
    ShapeDef {
        category: Category::Flowchart,
        ..basic(id, name, kind, w, h)
    }
}

const fn data(id: &'static str, name: &'static str, kind: ShapeKind, w: f32, h: f32) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Data,
        kind,
        w,
        h,
        fill: DATA_FILL,
        stroke: DATA_STROKE,
        badge: None,
    }
}

const fn net(id: &'static str, name: &'static str, kind: ShapeKind, w: f32, h: f32) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Network,
        kind,
        w,
        h,
        fill: NET_FILL,
        stroke: NET_STROKE,
        badge: None,
    }
}

const fn sec(id: &'static str, name: &'static str, kind: ShapeKind, w: f32, h: f32) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Security,
        kind,
        w,
        h,
        fill: SEC_FILL,
        stroke: SEC_STROKE,
        badge: None,
    }
}

const fn uml(id: &'static str, name: &'static str, kind: ShapeKind, w: f32, h: f32) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Uml,
        kind,
        w,
        h,
        fill: UML_FILL,
        stroke: UML_STROKE,
        badge: None,
    }
}

const fn aws(id: &'static str, name: &'static str, kind: ShapeKind) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Aws,
        kind,
        w: 120.0,
        h: 60.0,
        fill: AWS_FILL,
        stroke: AWS_STROKE,
        badge: Some("AWS"),
    }
}

const fn azure(id: &'static str, name: &'static str, kind: ShapeKind) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Azure,
        kind,
        w: 120.0,
        h: 60.0,
        fill: AZ_FILL,
        stroke: AZ_STROKE,
        badge: Some("AZ"),
    }
}

const fn gcp(id: &'static str, name: &'static str, kind: ShapeKind) -> ShapeDef {
    ShapeDef {
        id,
        name,
        category: Category::Gcp,
        kind,
        w: 120.0,
        h: 60.0,
        fill: GCP_FILL,
        stroke: GCP_STROKE,
        badge: Some("GCP"),
    }
}

use ShapeKind::*;

/// Static palette catalog. Order within a category is display order.
pub const CATALOG: &[ShapeDef] = &[
    // Basic
    basic("basic/rect", "Rectangle", Rect, 120.0, 60.0),
    basic("basic/rounded", "Rounded", Rounded, 120.0, 60.0),
    basic("basic/circle", "Circle", Circle, 72.0, 72.0),
    basic("basic/diamond", "Diamond", Diamond, 100.0, 72.0),
    basic("basic/triangle", "Triangle", Triangle, 90.0, 80.0),
    basic("basic/cloud", "Cloud", Cloud, 120.0, 72.0),
    // Flowchart
    flow("flow/process", "Process", Rect, 120.0, 60.0),
    flow("flow/decision", "Decision", Diamond, 110.0, 80.0),
    flow("flow/terminator", "Terminator", Stadium, 120.0, 48.0),
    flow("flow/data", "Data (I/O)", Parallelogram, 120.0, 60.0),
    flow("flow/document", "Document", Document, 120.0, 64.0),
    flow("flow/manual-input", "Manual Input", Trapezoid, 120.0, 60.0),
    flow("flow/preparation", "Preparation", Hexagon, 130.0, 60.0),
    // Data
    data("data/database", "Database", Cylinder, 90.0, 80.0),
    data("data/cache", "Cache", Cylinder, 90.0, 72.0),
    data("data/queue", "Queue", Rounded, 110.0, 52.0),
    data("data/store", "Data Store", DataStore, 120.0, 56.0),
    // Network
    net("net/router", "Router", Rect, 110.0, 56.0),
    net("net/switch", "Switch", Rect, 110.0, 56.0),
    net("net/firewall", "Firewall", Rect, 110.0, 64.0),
    net("net/lb", "Load Balancer", Rect, 120.0, 56.0),
    net("net/internet", "Internet", Cloud, 120.0, 72.0),
    net("net/subnet", "Subnet", Rounded, 130.0, 56.0),
    // Security
    sec("sec/shield", "Shield", Shield, 80.0, 92.0),
    sec("sec/lock", "Lock", Rounded, 90.0, 56.0),
    sec("sec/key", "Key", Rounded, 90.0, 56.0),
    sec("sec/cert", "Certificate", Rounded, 110.0, 56.0),
    // UML
    uml("uml/actor", "Actor", Actor, 56.0, 100.0),
    uml("uml/class", "Class", Rect, 130.0, 72.0),
    uml("uml/interface", "Interface", Circle, 64.0, 64.0),
    uml("uml/component", "Component", Component, 110.0, 64.0),
    uml("uml/package", "Package", Package, 110.0, 64.0),
    // AWS
    aws("aws/ec2", "EC2", Rounded),
    aws("aws/lambda", "Lambda", Rounded),
    aws("aws/s3", "S3", Rounded),
    aws("aws/rds", "RDS", Cylinder),
    aws("aws/dynamodb", "DynamoDB", Cylinder),
    aws("aws/api-gateway", "API Gateway", Rounded),
    aws("aws/cloudfront", "CloudFront", Rounded),
    aws("aws/vpc", "VPC", Rounded),
    aws("aws/sqs", "SQS", Rounded),
    aws("aws/sns", "SNS", Rounded),
    aws("aws/ecs", "ECS", Rounded),
    aws("aws/eks", "EKS", Rounded),
    aws("aws/elb", "ELB", Rounded),
    aws("aws/iam", "IAM", Rounded),
    aws("aws/cloudwatch", "CloudWatch", Rounded),
    // Azure
    azure("az/vm", "Virtual Machine", Rounded),
    azure("az/functions", "Functions", Rounded),
    azure("az/blob", "Blob Storage", Rounded),
    azure("az/sql", "SQL Database", Cylinder),
    azure("az/cosmos", "Cosmos DB", Cylinder),
    azure("az/vnet", "Virtual Network", Rounded),
    azure("az/lb", "Load Balancer", Rounded),
    azure("az/aks", "AKS", Rounded),
    azure("az/app-service", "App Service", Rounded),
    azure("az/entra", "Entra ID", Rounded),
    // GCP
    gcp("gcp/compute", "Compute Engine", Rounded),
    gcp("gcp/functions", "Cloud Functions", Rounded),
    gcp("gcp/storage", "Cloud Storage", Rounded),
    gcp("gcp/sql", "Cloud SQL", Cylinder),
    gcp("gcp/firestore", "Firestore", Cylinder),
    gcp("gcp/gke", "GKE", Rounded),
    gcp("gcp/pubsub", "Pub/Sub", Rounded),
    gcp("gcp/vpc", "VPC", Rounded),
    gcp("gcp/run", "Cloud Run", Rounded),
    gcp("gcp/bigquery", "BigQuery", Rounded),
];

/// Look up a palette entry by id.
pub fn shape_def(id: &str) -> Option<&'static ShapeDef> {
    CATALOG.iter().find(|d| d.id == id)
}

// ---------------------------------------------------------------------------
// SVG geometry
// ---------------------------------------------------------------------------

/// Format a coordinate deterministically (trim trailing zeros).
pub fn num(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

/// Escape XML text content / attribute values.
pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Fill + stroke colors for SVG emission.
#[derive(Debug, Clone, Copy)]
pub struct SvgStyle<'a> {
    pub fill: &'a str,
    pub stroke: &'a str,
}

/// Emit the SVG element(s) for a shape body centred at (x, y) with size w×h.
pub fn shape_svg(out: &mut String, kind: ShapeKind, x: f32, y: f32, w: f32, h: f32, style: SvgStyle<'_>) {
    let SvgStyle { fill, stroke } = style;
    let (l, t) = (num(x - w / 2.0), num(y - h / 2.0));
    let (x, y, w, h) = (num(x), num(y), num(w), num(h));
    match kind {
        ShapeKind::Rect => {
            out.push_str(&format!(
                "<rect x=\"{l}\" y=\"{t}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
            ));
        }
        ShapeKind::Rounded | ShapeKind::Stadium => {
            let rx = if kind == ShapeKind::Stadium {
                num(h.parse::<f32>().unwrap_or(0.0) / 2.0)
            } else {
                "8".to_string()
            };
            out.push_str(&format!(
                "<rect x=\"{l}\" y=\"{t}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
            ));
        }
        ShapeKind::Circle => {
            let (rx, ry) = (
                num(w.parse::<f32>().unwrap_or(0.0) / 2.0),
                num(h.parse::<f32>().unwrap_or(0.0) / 2.0),
            );
            out.push_str(&format!(
                "<ellipse cx=\"{x}\" cy=\"{y}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
            ));
        }
        ShapeKind::Diamond => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(xf),
                num(yf - hf / 2.0),
                num(xf + wf / 2.0),
                num(yf),
                num(xf),
                num(yf + hf / 2.0),
                num(xf - wf / 2.0),
                num(yf),
            ));
        }
        ShapeKind::Hexagon => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let dx = wf * 0.22;
            let (l, r) = (xf - wf / 2.0, xf + wf / 2.0);
            let (t, b) = (yf - hf / 2.0, yf + hf / 2.0);
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(l + dx),
                num(t),
                num(r - dx),
                num(t),
                num(r),
                num(yf),
                num(r - dx),
                num(b),
                num(l + dx),
                num(b),
                num(l),
                num(yf),
            ));
        }
        ShapeKind::Triangle => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(xf),
                num(yf - hf / 2.0),
                num(xf + wf / 2.0),
                num(yf + hf / 2.0),
                num(xf - wf / 2.0),
                num(yf + hf / 2.0),
            ));
        }
        ShapeKind::Parallelogram | ShapeKind::Trapezoid => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let (l, r) = (xf - wf / 2.0, xf + wf / 2.0);
            let (t, b) = (yf - hf / 2.0, yf + hf / 2.0);
            let s = wf * 0.18;
            let pts = if kind == ShapeKind::Parallelogram {
                format!(
                    "{},{} {},{} {},{} {},{}",
                    num(l + s),
                    num(t),
                    num(r),
                    num(t),
                    num(r - s),
                    num(b),
                    num(l),
                    num(b)
                )
            } else {
                format!(
                    "{},{} {},{} {},{} {},{}",
                    num(l + s),
                    num(t),
                    num(r - s),
                    num(t),
                    num(r),
                    num(b),
                    num(l),
                    num(b)
                )
            };
            out.push_str(&format!(
                "<polygon points=\"{pts}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
            ));
        }
        ShapeKind::Cylinder => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let rx = wf / 2.0;
            let ry = (hf * 0.14).max(4.0);
            let (l, r) = (xf - rx, xf + rx);
            let (ty, by) = (yf - hf / 2.0 + ry, yf + hf / 2.0 - ry);
            out.push_str(&format!(
                "<path d=\"M {},{} L {},{} A {},{} 0 0 0 {},{} L {},{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(l),
                num(ty),
                num(l),
                num(by),
                num(rx),
                num(ry),
                num(r),
                num(by),
                num(r),
                num(ty),
            ));
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(xf),
                num(ty),
                num(rx),
                num(ry),
            ));
        }
        ShapeKind::Cloud => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            // Canonical 100×60 cloud outline, scaled into the bounding box.
            let sx = wf / 100.0;
            let sy = hf / 60.0;
            let px = |cx: f32| num(xf - wf / 2.0 + cx * sx);
            let py = |cy: f32| num(yf - hf / 2.0 + cy * sy);
            out.push_str(&format!(
                "<path d=\"M {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                px(20.0), py(50.0),
                px(8.0), py(50.0), px(4.0), py(40.0), px(10.0), py(33.0),
                px(6.0), py(22.0), px(16.0), py(14.0), px(26.0), py(18.0),
                px(30.0), py(6.0), px(46.0), py(4.0), px(54.0), py(12.0),
                px(64.0), py(6.0), px(78.0), py(10.0), px(80.0), py(20.0),
                px(92.0), py(20.0), px(97.0), py(30.0), px(91.0), py(38.0),
                px(96.0), py(46.0), px(88.0), py(50.0), px(80.0), py(50.0),
            ));
        }
        ShapeKind::Document => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let (l, r) = (xf - wf / 2.0, xf + wf / 2.0);
            let (t, b) = (yf - hf / 2.0, yf + hf / 2.0);
            let wave = (hf * 0.12).max(3.0);
            out.push_str(&format!(
                "<path d=\"M {},{} L {},{} L {},{} Q {},{} {},{} T {},{} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(l),
                num(t),
                num(r),
                num(t),
                num(r),
                num(b - wave),
                num((l + r) / 2.0 + wf * 0.25),
                num(b + wave),
                num((l + r) / 2.0),
                num(b - wave),
                num(l),
                num(b - wave),
            ));
        }
        ShapeKind::Actor => {
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let head_r = hf * 0.13;
            let head_cy = yf - hf / 2.0 + head_r;
            let neck = head_cy + head_r;
            let hip = yf + hf * 0.22;
            let foot = yf + hf / 2.0;
            let arm_y = neck + hf * 0.10;
            let arm_x = hf * 0.22;
            let leg_dx = hf * 0.16;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(xf),
                num(head_cy),
                num(head_r),
            ));
            out.push_str(&format!(
                "<path d=\"M {},{} L {},{} M {},{} L {},{} M {},{} L {},{} M {},{} L {},{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(xf), num(neck), num(xf), num(hip),
                num(xf - arm_x), num(arm_y), num(xf + arm_x), num(arm_y),
                num(xf), num(hip), num(xf - leg_dx), num(foot),
                num(xf), num(hip), num(xf + leg_dx), num(foot),
            ));
        }
        ShapeKind::Component => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let (l, t) = (xf - wf / 2.0, yf - hf / 2.0);
            let tab_w = (wf * 0.10).max(6.0);
            let tab_h = hf * 0.22;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(l), num(t), num(wf), num(hf),
            ));
            for i in [0.18_f32, 0.60] {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                    num(l - tab_w / 2.0),
                    num(t + hf * i),
                    num(tab_w),
                    num(tab_h),
                ));
            }
        }
        ShapeKind::Package => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let (l, r) = (xf - wf / 2.0, xf + wf / 2.0);
            let (t, b) = (yf - hf / 2.0, yf + hf / 2.0);
            let tab_w = wf * 0.42;
            let tab_h = hf * 0.22;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(l),
                num(t + tab_h),
                num(l),
                num(b),
                num(r),
                num(b),
                num(r),
                num(t + tab_h),
                num(l + tab_w),
                num(t + tab_h),
                num(l + tab_w),
                num(t),
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(l),
                num(t + tab_h),
                num(l + tab_w),
                num(t + tab_h),
            ));
        }
        ShapeKind::Shield => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let (l, r) = (xf - wf / 2.0, xf + wf / 2.0);
            let (t, b) = (yf - hf / 2.0, yf + hf / 2.0);
            out.push_str(&format!(
                "<path d=\"M {},{} L {},{} L {},{} C {},{} {},{} {},{} C {},{} {},{} {},{} L {},{} Z\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(xf), num(t),
                num(r), num(t + hf * 0.15),
                num(r), num(t + hf * 0.45),
                num(r), num(t + hf * 0.8), num(xf), num(t + hf * 0.9), num(xf), num(b),
                num(xf), num(t + hf * 0.9), num(l), num(t + hf * 0.8), num(l), num(t + hf * 0.45),
                num(l), num(t + hf * 0.15),
            ));
        }
        ShapeKind::DataStore => {
            let wf = w.parse::<f32>().unwrap_or(0.0);
            let hf = h.parse::<f32>().unwrap_or(0.0);
            let xf = x.parse::<f32>().unwrap_or(0.0);
            let yf = y.parse::<f32>().unwrap_or(0.0);
            let (l, r) = (xf - wf / 2.0, xf + wf / 2.0);
            let (t, b) = (yf - hf / 2.0, yf + hf / 2.0);
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{fill}\"/>",
                num(l), num(t), num(wf), num(hf),
            ));
            out.push_str(&format!(
                "<path d=\"M {},{} L {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
                num(r), num(t), num(l), num(t), num(l), num(b), num(r), num(b),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for def in CATALOG {
            assert!(seen.insert(def.id), "duplicate catalog id {}", def.id);
        }
    }

    #[test]
    fn every_category_has_shapes() {
        for cat in Category::all() {
            assert!(
                CATALOG.iter().any(|d| d.category == cat),
                "category {:?} is empty",
                cat
            );
        }
    }

    #[test]
    fn cloud_entries_have_badges() {
        for cat in [Category::Aws, Category::Azure, Category::Gcp] {
            for def in CATALOG.iter().filter(|d| d.category == cat) {
                assert!(def.badge.is_some(), "{} missing provider badge", def.id);
            }
        }
    }

    #[test]
    fn num_trims_decimals() {
        assert_eq!(num(3.0), "3");
        assert_eq!(num(3.5), "3.5");
        assert_eq!(num(-0.0), "0");
        assert_eq!(num(12.25), "12.25");
    }

    #[test]
    fn shape_svg_emits_for_all_kinds() {
        let kinds = [
            ShapeKind::Rect,
            ShapeKind::Rounded,
            ShapeKind::Stadium,
            ShapeKind::Diamond,
            ShapeKind::Hexagon,
            ShapeKind::Circle,
            ShapeKind::Triangle,
            ShapeKind::Parallelogram,
            ShapeKind::Trapezoid,
            ShapeKind::Cylinder,
            ShapeKind::Cloud,
            ShapeKind::Document,
            ShapeKind::Actor,
            ShapeKind::Component,
            ShapeKind::Package,
            ShapeKind::Shield,
            ShapeKind::DataStore,
        ];
        for kind in kinds {
            let mut s = String::new();
            shape_svg(&mut s, kind, 60.0, 40.0, 120.0, 80.0, SvgStyle { fill: "#ffffff", stroke: "#000000" });
            assert!(!s.is_empty(), "no svg emitted for {:?}", kind);
            assert!(!s.contains("NaN"), "NaN coordinate for {:?}: {}", kind, s);
        }
    }
}
