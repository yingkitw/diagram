use crate::layout::{Layout, LayoutNode};
use crate::diagram::{EdgeStyle, NodeShape};

pub fn render_svg(layout: &Layout) -> String {
    let mut svg = String::new();

    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"##,
        w = layout.width as i32,
        h = layout.height as i32,
    ));

    svg.push_str(
        r##"<defs><marker id="arrow" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">"##,
    );
    svg.push_str(r##"<polygon points="0 0, 10 3.5, 0 7" fill="#64748b"/></marker>"##);

    svg.push_str(
        r##"<marker id="arrow-thick" markerWidth="12" markerHeight="8" refX="10" refY="4" orient="auto">"##,
    );
    svg.push_str(r##"<polygon points="0 0, 12 4, 0 8" fill="#e2e8f0"/></marker>"##);

    svg.push_str(
        r##"<marker id="arrow-dashed" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">"##,
    );
    svg.push_str(r##"<polygon points="0 0, 10 3.5, 0 7" fill="#94a3b8"/></marker></defs>"##);
    svg.push_str(r##"<rect width="100%" height="100%" fill="#1a1a2e"/>"##);

    for e in &layout.edges {
        let d = if e.points.len() >= 2 {
            let mut path = String::from("M");
            for (i, (x, y)) in e.points.iter().enumerate() {
                if i == 0 {
                    path.push_str(&format!(" {:.1},{:.1}", x, y));
                } else {
                    path.push_str(&format!(" L {:.1},{:.1}", x, y));
                }
            }
            path
        } else {
            String::new()
        };

        if !d.is_empty() {
            let (stroke_color, stroke_width, dash_array, marker) = match e.style {
                EdgeStyle::Arrow => ("#64748b", 2, "", "url(#arrow)"),
                EdgeStyle::Dashed => ("#94a3b8", 2, "6,4", "url(#arrow-dashed)"),
                EdgeStyle::Thick => ("#e2e8f0", 4, "", "url(#arrow-thick)"),
            };
            svg.push_str(&format!(
                r##"<path d="{d}" fill="none" stroke="{stroke_color}" stroke-width="{stroke_width}" {dash} marker-end="{marker}"/>"##,
                dash = if dash_array.is_empty() {
                    String::new()
                } else {
                    format!(r##"stroke-dasharray="{}""##, dash_array)
                },
            ));
        }

        if !e.label.is_empty() && e.points.len() >= 2 {
            let mid_idx = e.points.len() / 2;
            let (mx, my) = e.points[mid_idx];
            svg.push_str(&format!(
                r##"<text x="{mx:.1}" y="{my:.1}" text-anchor="middle" dominant-baseline="central" fill="#94a3b8" font-size="11">{}</text>"##,
                escape_xml(&e.label),
            ));
        }
    }

    for n in &layout.nodes {
        render_node(&mut svg, n);
    }

    svg.push_str("</svg>");
    svg
}

fn render_node(svg: &mut String, n: &LayoutNode) {
    svg.push_str(&format!(
        r##"<g transform="translate({:.1},{:.1})">"##,
        n.x, n.y,
    ));

    match n.shape {
        NodeShape::Rect => {
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="6" fill="#334155" stroke="#475569" stroke-width="2"/>"##,
                -n.width / 2.0, -n.height / 2.0, n.width, n.height,
            ));
        }
        NodeShape::Diamond => {
            let hw = n.width / 2.0;
            let hh = n.height / 2.0;
            svg.push_str(&format!(
                r##"<polygon points="{:.1},0 0,{:.1} {:.1},0 0,{:.1}" fill="#334155" stroke="#475569" stroke-width="2"/>"##,
                hw, -hh, -hw, hh,
            ));
        }
        NodeShape::Stadium => {
            let r = n.height.min(n.width) / 4.0;
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{r:.1}" ry="{r:.1}" fill="#334155" stroke="#475569" stroke-width="2"/>"##,
                -n.width / 2.0, -n.height / 2.0, n.width, n.height,
            ));
        }
        NodeShape::Hexagon => {
            let hw = n.width / 2.0;
            let hh = n.height / 2.0;
            let inset = n.width * 0.25;
            let il = hw - inset;
            let t = -hh;
            let b = hh;
            let nil = -il;
            let nhw = -hw;
            svg.push_str(&format!(
                r##"<polygon points="{il:.1},{t:.1} {hw:.1},0 {il:.1},{b:.1} {nil:.1},{b:.1} {nhw:.1},0 {nil:.1},{t:.1}" fill="#334155" stroke="#475569" stroke-width="2"/>"##,
            ));
        }
        NodeShape::Cylinder => {
            let hw = n.width / 2.0;
            let hh = n.height / 2.0;
            let elly = -hh + 6.0;
            let b = hh;
            let nhw = -hw;
            let er = 6.0;
            svg.push_str(&format!(
                r##"<path d="M{nhw:.1},{elly:.1} L{nhw:.1},{b:.1} A{hw:.1},{er:.1} 0 0,0 {hw:.1},{b:.1} L{hw:.1},{elly:.1} A{hw:.1},{er:.1} 0 0,1 {nhw:.1},{elly:.1} Z" fill="#334155" stroke="#475569" stroke-width="2"/>"##,
            ));
            svg.push_str(&format!(
                r##"<ellipse cx="0" cy="{elly:.1}" rx="{hw:.1}" ry="{er:.1}" fill="#1e293b" stroke="#475569" stroke-width="2"/>"##,
            ));
        }
        NodeShape::Circle => {
            let r = n.width.min(n.height) / 2.0 - 2.0;
            svg.push_str(&format!(
                r##"<circle cx="0" cy="0" r="{r:.1}" fill="#334155" stroke="#475569" stroke-width="2"/>"##,
            ));
        }
    }

    let display_text = if n.label.len() > 20 {
        format!("{}…", &n.label[..19])
    } else {
        n.label.clone()
    };

    svg.push_str(&format!(
        r##"<text x="0" y="0" text-anchor="middle" dominant-baseline="central" fill="#f1f5f9" font-size="12">{}</text>"##,
        escape_xml(&display_text),
    ));

    svg.push_str("</g>");
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
