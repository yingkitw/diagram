use crate::layout::{Layout, LayoutNode, LayoutSubgraph};
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

    for sg in &layout.subgraphs {
        render_subgraph(&mut svg, sg);
    }

    for e in &layout.edges {
        let d = if e.points.len() >= 2 {
            let start = e.points[0];
            let end = e.points[e.points.len() - 1];
            let dx = end.0 - start.0;
            let dy = end.1 - start.1;
            // Control points extend 40% along the dominant direction for a smooth curve
            let (c1, c2) = if dx.abs() > dy.abs() {
                ((start.0 + dx * 0.4, start.1), (end.0 - dx * 0.4, end.1))
            } else {
                ((start.0, start.1 + dy * 0.4), (end.0, end.1 - dy * 0.4))
            };
            format!(
                "M {:.1},{:.1} C {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                start.0, start.1, c1.0, c1.1, c2.0, c2.1, end.0, end.1
            )
        } else {
            String::new()
        };

        if !d.is_empty() {
            let (default_color, default_width, dash_array, marker) = match e.style {
                EdgeStyle::Arrow => ("#64748b", 2, "", "url(#arrow)"),
                EdgeStyle::Dashed => ("#94a3b8", 2, "6,4", "url(#arrow-dashed)"),
                EdgeStyle::Thick => ("#e2e8f0", 4, "", "url(#arrow-thick)"),
            };
            let stroke_color = e.stroke_color.as_deref().unwrap_or(default_color);
            let default_width_str = default_width.to_string();
            let stroke_width = e.stroke_width.as_deref().unwrap_or(&default_width_str);
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
    let fill = n.fill.as_deref().unwrap_or("#334155");
    let stroke = n.stroke.as_deref().unwrap_or("#475569");

    svg.push_str(&format!(
        r##"<g transform="translate({:.1},{:.1})">"##,
        n.x, n.y,
    ));

    match n.shape {
        NodeShape::Rect => {
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="6" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"##,
                -n.width / 2.0, -n.height / 2.0, n.width, n.height,
            ));
        }
        NodeShape::Diamond => {
            let hw = n.width / 2.0;
            let hh = n.height / 2.0;
            svg.push_str(&format!(
                r##"<polygon points="{:.1},0 0,{:.1} {:.1},0 0,{:.1}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"##,
                hw, -hh, -hw, hh,
            ));
        }
        NodeShape::Stadium => {
            let r = n.height.min(n.width) / 4.0;
            svg.push_str(&format!(
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{r:.1}" ry="{r:.1}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"##,
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
                r##"<polygon points="{il:.1},{t:.1} {hw:.1},0 {il:.1},{b:.1} {nil:.1},{b:.1} {nhw:.1},0 {nil:.1},{t:.1}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"##,
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
                r##"<path d="M{nhw:.1},{elly:.1} L{nhw:.1},{b:.1} A{hw:.1},{er:.1} 0 0,0 {hw:.1},{b:.1} L{hw:.1},{elly:.1} A{hw:.1},{er:.1} 0 0,1 {nhw:.1},{elly:.1} Z" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"##,
            ));
            svg.push_str(&format!(
                r##"<ellipse cx="0" cy="{elly:.1}" rx="{hw:.1}" ry="{er:.1}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"##,
            ));
        }
        NodeShape::Circle => {
            let r = n.width.min(n.height) / 2.0 - 2.0;
            svg.push_str(&format!(
                r##"<circle cx="0" cy="0" r="{r:.1}" fill="{fill}" stroke="{stroke}" stroke-width="2"/>"##,
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

fn render_subgraph(svg: &mut String, sg: &LayoutSubgraph) {
    let rx = sg.x - sg.width / 2.0;
    let ry = sg.y - sg.height / 2.0;
    svg.push_str(&format!(
        r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="8" fill="#1e293b" fill-opacity="0.6" stroke="#334155" stroke-width="2" stroke-dasharray="4,4"/>"##,
        rx, ry, sg.width, sg.height,
    ));
    svg.push_str(&format!(
        r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="central" fill="#94a3b8" font-size="11" font-style="italic">{}</text>"##,
        sg.x, ry + 12.0,
        escape_xml(&sg.label),
    ));
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
