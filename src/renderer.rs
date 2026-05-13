use crate::layout::{Layout, LayoutNode};
use crate::diagram::NodeShape;

pub fn render_svg(layout: &Layout) -> String {
    let mut svg = String::new();

    svg.push_str(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"##,
        w = layout.width as i32,
        h = layout.height as i32,
    ));

    svg.push_str(
        r##"<defs><marker id="arrow" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">"##
    );
    svg.push_str(r##"<polygon points="0 0, 10 3.5, 0 7" fill="#64748b"/></marker></defs>"##);
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
            svg.push_str(&format!(
                r##"<path d="{d}" fill="none" stroke="#64748b" stroke-width="2" marker-end="url(#arrow)"/>"##,
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
