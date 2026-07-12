//! Mermaid gantt chart parse, layout, and SVG render (MVP).

use crate::renderer::Theme;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttTask {
    pub name: String,
    pub id: Option<String>,
    /// Start as days since Unix epoch (UTC date).
    pub start: i64,
    /// End exclusive as days since Unix epoch.
    pub end: i64,
    pub section: String,
    pub crit: bool,
    pub done: bool,
    pub active: bool,
    /// Point-in-time marker (`milestone` tag); rendered as a diamond.
    pub milestone: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttDiagram {
    pub title: String,
    pub date_format: String,
    pub tasks: Vec<GanttTask>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {}: {}", l, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn is_gantt(source: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .is_some_and(|l| l == "gantt")
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` date.
pub fn parse_ymd(s: &str) -> Option<i64> {
    let mut parts = s.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m as i32, d as i32))
}

fn days_from_civil(y: i32, m: i32, d: i32) -> i64 {
    // Howard Hinnant civil_from_days algorithm (inverse).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as u32;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy as u32;
    (era as i64 * 146097 + doe as i64) - 719468
}

fn format_ymd(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64 + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn parse_duration_days(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('d') {
        return n.parse().ok();
    }
    if let Some(n) = s.strip_suffix('h') {
        let h: i64 = n.parse().ok()?;
        return Some((h + 23) / 24); // round up hours to days
    }
    None
}

pub fn parse(source: &str) -> Result<GanttDiagram, ParseError> {
    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("%%"))
        .collect();

    if lines.is_empty() || lines[0].1 != "gantt" {
        return Err(ParseError {
            message: "expected gantt header".into(),
            line: lines.first().map(|(n, _)| *n),
        });
    }

    let mut title = String::new();
    let mut date_format = "YYYY-MM-DD".to_string();
    let mut section = String::from("Default");
    let mut tasks: Vec<GanttTask> = Vec::new();
    let mut id_end: HashMap<String, i64> = HashMap::new();
    let mut cursor_start: Option<i64> = None;

    for (line_num, text) in lines.iter().skip(1) {
        if let Some(rest) = text.strip_prefix("title ") {
            title = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = text.strip_prefix("dateFormat ") {
            date_format = rest.trim().to_string();
            if date_format != "YYYY-MM-DD" {
                return Err(ParseError {
                    message: format!("unsupported dateFormat '{date_format}' (only YYYY-MM-DD)"),
                    line: Some(*line_num),
                });
            }
            continue;
        }
        if let Some(rest) = text.strip_prefix("section ") {
            section = rest.trim().to_string();
            continue;
        }

        // Task line: `Name : tags..., start/after, duration/end`
        let Some((name, meta)) = text.split_once(':') else {
            return Err(ParseError {
                message: format!("unrecognized gantt line: {text}"),
                line: Some(*line_num),
            });
        };
        let name = name.trim().to_string();
        let parts: Vec<&str> = meta
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();

        let mut crit = false;
        let mut done = false;
        let mut active = false;
        let mut milestone = false;
        let mut id: Option<String> = None;
        let mut start: Option<i64> = None;
        let mut end: Option<i64> = None;
        let mut after_id: Option<String> = None;
        let mut duration: Option<i64> = None;

        for p in &parts {
            match *p {
                "crit" => crit = true,
                "done" => done = true,
                "active" => active = true,
                "milestone" => {
                    milestone = true;
                    if duration.is_none() {
                        duration = Some(0);
                    }
                }
                _ if p.starts_with("after ") => {
                    after_id = Some(p.strip_prefix("after ").unwrap().trim().to_string());
                }
                _ if parse_ymd(p).is_some() => {
                    let d = parse_ymd(p).unwrap();
                    if start.is_none() {
                        start = Some(d);
                    } else {
                        end = Some(d);
                    }
                }
                _ if parse_duration_days(p).is_some() => {
                    duration = parse_duration_days(p);
                }
                _ if p
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                    && id.is_none()
                    && start.is_none()
                    && after_id.is_none() =>
                {
                    id = Some((*p).to_string());
                }
                _ => {
                    return Err(ParseError {
                        message: format!("unrecognized task token '{p}'"),
                        line: Some(*line_num),
                    });
                }
            }
        }

        let start = if let Some(a) = after_id {
            id_end.get(&a).copied().ok_or_else(|| ParseError {
                message: format!("unknown task id '{a}' in after clause"),
                line: Some(*line_num),
            })?
        } else if let Some(s) = start {
            s
        } else if let Some(c) = cursor_start {
            c
        } else {
            return Err(ParseError {
                message: "task missing start date".into(),
                line: Some(*line_num),
            });
        };

        let end = if let Some(e) = end {
            e
        } else if let Some(dur) = duration {
            if milestone {
                start + dur.max(0)
            } else {
                start + dur.max(1)
            }
        } else if milestone {
            start
        } else {
            return Err(ParseError {
                message: "task missing end date or duration".into(),
                line: Some(*line_num),
            });
        };

        // Milestones are points; `after` dependents start on the milestone day.
        let after_anchor = if milestone { start } else { end };
        if let Some(ref tid) = id {
            id_end.insert(tid.clone(), after_anchor);
        }
        cursor_start = Some(after_anchor);

        tasks.push(GanttTask {
            name,
            id,
            start,
            end: if milestone { start } else { end },
            section: section.clone(),
            crit,
            done,
            active,
            milestone,
        });
    }

    Ok(GanttDiagram {
        title,
        date_format,
        tasks,
    })
}

impl GanttDiagram {
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("gantt\n");
        if !self.title.is_empty() {
            out.push_str(&format!("    title {}\n", self.title));
        }
        out.push_str(&format!("    dateFormat {}\n", self.date_format));
        let mut current_section = String::new();
        for t in &self.tasks {
            if t.section != current_section {
                current_section = t.section.clone();
                out.push_str(&format!("    section {}\n", t.section));
            }
            let mut meta = Vec::new();
            if t.crit {
                meta.push("crit".to_string());
            }
            if t.active {
                meta.push("active".to_string());
            }
            if t.done {
                meta.push("done".to_string());
            }
            if t.milestone {
                meta.push("milestone".to_string());
            }
            if let Some(id) = &t.id {
                meta.push(id.clone());
            }
            meta.push(format_ymd(t.start));
            if t.milestone {
                meta.push("0d".to_string());
            } else {
                let dur = (t.end - t.start).max(1);
                meta.push(format!("{dur}d"));
            }
            out.push_str(&format!("    {} : {}\n", t.name, meta.join(", ")));
        }
        out
    }
}

const LABEL_W: f64 = 160.0;
const ROW_H: f64 = 28.0;
const MARGIN: f64 = 40.0;
const DAY_W: f64 = 14.0;
const HEADER_H: f64 = 50.0;

struct LaidTask {
    name: String,
    x: f64,
    y: f64,
    w: f64,
    crit: bool,
    done: bool,
    active: bool,
    milestone: bool,
}

struct GanttLayout {
    width: f64,
    height: f64,
    title: String,
    tasks: Vec<LaidTask>,
    min_day: i64,
    max_day: i64,
    section_ys: Vec<(String, f64)>,
}

fn layout(diagram: &GanttDiagram) -> GanttLayout {
    if diagram.tasks.is_empty() {
        return GanttLayout {
            width: 400.0,
            height: 120.0,
            title: diagram.title.clone(),
            tasks: Vec::new(),
            min_day: 0,
            max_day: 1,
            section_ys: Vec::new(),
        };
    }
    let min_day = diagram.tasks.iter().map(|t| t.start).min().unwrap();
    let max_day = diagram.tasks.iter().map(|t| t.end).max().unwrap().max(min_day + 1);

    let mut tasks = Vec::new();
    let mut section_ys = Vec::new();
    let mut y = MARGIN + HEADER_H;
    let mut current = String::new();
    for t in &diagram.tasks {
        if t.section != current {
            current = t.section.clone();
            section_ys.push((current.clone(), y));
            y += ROW_H;
        }
        let x = MARGIN + LABEL_W + (t.start - min_day) as f64 * DAY_W;
        let w = if t.milestone {
            0.0
        } else {
            ((t.end - t.start) as f64 * DAY_W).max(DAY_W)
        };
        tasks.push(LaidTask {
            name: t.name.clone(),
            x,
            y,
            w,
            crit: t.crit,
            done: t.done,
            active: t.active,
            milestone: t.milestone,
        });
        y += ROW_H;
    }

    let width = MARGIN * 2.0 + LABEL_W + (max_day - min_day) as f64 * DAY_W + 20.0;
    GanttLayout {
        width,
        height: y + MARGIN,
        title: diagram.title.clone(),
        tasks,
        min_day,
        max_day,
        section_ys,
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn render_svg(diagram: &GanttDiagram, theme: Theme) -> String {
    let laid = layout(diagram);
    let bg = match theme {
        Theme::Dark => "#1a1a2e",
        Theme::Light => "#ffffff",
    };
    let text = match theme {
        Theme::Dark => "#f1f5f9",
        Theme::Light => "#1e293b",
    };
    let muted = match theme {
        Theme::Dark => "#94a3b8",
        Theme::Light => "#64748b",
    };
    let grid = match theme {
        Theme::Dark => "#334155",
        Theme::Light => "#e2e8f0",
    };
    let bar = match theme {
        Theme::Dark => "#3b82f6",
        Theme::Light => "#2563eb",
    };
    let crit = match theme {
        Theme::Dark => "#ef4444",
        Theme::Light => "#dc2626",
    };
    let done = match theme {
        Theme::Dark => "#64748b",
        Theme::Light => "#94a3b8",
    };
    let active = match theme {
        Theme::Dark => "#22c55e",
        Theme::Light => "#16a34a",
    };

    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"##,
        w = laid.width as i32,
        h = laid.height as i32,
    );
    svg.push_str(&format!(
        r#"<rect width="100%" height="100%" fill="{bg}"/>"#
    ));

    if !laid.title.is_empty() {
        svg.push_str(&format!(
            r##"<text x="{x}" y="{y}" fill="{c}" font-size="16" font-weight="600" font-family="sans-serif">{t}</text>"##,
            x = MARGIN,
            y = MARGIN,
            c = text,
            t = esc(&laid.title),
        ));
    }

    // Vertical day grid + tick labels every 7 days
    let chart_left = MARGIN + LABEL_W;
    let chart_top = MARGIN + HEADER_H - 10.0;
    let chart_bottom = laid.height - MARGIN;
    for day in laid.min_day..=laid.max_day {
        let x = chart_left + (day - laid.min_day) as f64 * DAY_W;
        svg.push_str(&format!(
            r##"<line x1="{x}" y1="{y1}" x2="{x}" y2="{y2}" stroke="{g}" stroke-width="1"/>"##,
            y1 = chart_top,
            y2 = chart_bottom,
            g = grid,
        ));
        if (day - laid.min_day) % 7 == 0 {
            svg.push_str(&format!(
                r##"<text x="{x}" y="{y}" fill="{c}" font-size="10" font-family="sans-serif">{d}</text>"##,
                y = chart_top - 4.0,
                c = muted,
                d = format_ymd(day),
            ));
        }
    }

    for (section, y) in &laid.section_ys {
        svg.push_str(&format!(
            r##"<text x="{x}" y="{y}" fill="{c}" font-size="12" font-weight="600" font-family="sans-serif">{t}</text>"##,
            x = MARGIN,
            y = y + 16.0,
            c = text,
            t = esc(section),
        ));
    }

    for t in &laid.tasks {
        let fill = if t.crit {
            crit
        } else if t.done {
            done
        } else if t.active {
            active
        } else {
            bar
        };
        svg.push_str(&format!(
            r##"<text x="{x}" y="{y}" fill="{c}" font-size="12" font-family="sans-serif">{t}</text>"##,
            x = MARGIN,
            y = t.y + 18.0,
            c = muted,
            t = esc(&t.name),
        ));
        if t.milestone {
            let cx = t.x;
            let cy = t.y + 13.0;
            let s = 8.0;
            svg.push_str(&format!(
                r##"<polygon points="{x1},{y0} {x2},{y1} {x1},{y2} {x0},{y1}" fill="{fill}" stroke="{fill}"/>"##,
                x0 = cx - s,
                x1 = cx,
                x2 = cx + s,
                y0 = cy - s,
                y1 = cy,
                y2 = cy + s,
                fill = fill,
            ));
        } else {
            svg.push_str(&format!(
                r##"<rect x="{x}" y="{y}" width="{w}" height="18" rx="3" fill="{fill}"/>"##,
                x = t.x,
                y = t.y + 4.0,
                w = t.w,
                fill = fill,
            ));
        }
    }

    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"gantt
    title Project Plan
    dateFormat YYYY-MM-DD
    section Design
    Research :a1, 2024-01-01, 7d
    Spec :after a1, 5d
    section Build
    Implement :2024-01-15, 10d
"#;

    #[test]
    fn parse_ymd_roundtrip() {
        let d = parse_ymd("2024-01-01").unwrap();
        assert_eq!(format_ymd(d), "2024-01-01");
    }

    #[test]
    fn parse_basic() {
        let g = parse(SAMPLE).unwrap();
        assert_eq!(g.title, "Project Plan");
        assert_eq!(g.tasks.len(), 3);
        assert_eq!(g.tasks[0].id.as_deref(), Some("a1"));
        assert_eq!(g.tasks[1].start, g.tasks[0].end);
        assert_eq!(g.tasks[2].section, "Build");
    }

    #[test]
    fn roundtrip() {
        let g = parse(SAMPLE).unwrap();
        let out = g.to_mermaid();
        let g2 = parse(&out).unwrap();
        assert_eq!(g2.tasks.len(), g.tasks.len());
        assert_eq!(g2.tasks[0].name, g.tasks[0].name);
    }

    #[test]
    fn render_ok() {
        let g = parse(SAMPLE).unwrap();
        let svg = render_svg(&g, Theme::Dark);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Research"));
        assert!(svg.contains("Project Plan"));
    }

    #[test]
    fn is_gantt_detects() {
        assert!(is_gantt(SAMPLE));
        assert!(!is_gantt("graph TD\n A-->B\n"));
    }

    #[test]
    fn parse_milestone() {
        let src = r#"gantt
    title Ship
    dateFormat YYYY-MM-DD
    section Build
    Implement :a1, 2024-01-01, 5d
    Launch :milestone, m1, 2024-01-10, 0d
    Followup :after m1, 3d
"#;
        let g = parse(src).unwrap();
        assert!(g.tasks[1].milestone);
        assert_eq!(g.tasks[1].start, g.tasks[1].end);
        assert_eq!(g.tasks[2].start, g.tasks[1].start);
        let svg = render_svg(&g, Theme::Light);
        assert!(svg.contains("<polygon"));
        let out = g.to_mermaid();
        assert!(out.contains("milestone"));
        let g2 = parse(&out).unwrap();
        assert!(g2.tasks[1].milestone);
        assert_eq!(g2.tasks[1].name, "Launch");
    }
}
