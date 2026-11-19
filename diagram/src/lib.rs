//! Mermaid diagrams laid out on the device, the same on every client: flowcharts, state diagrams, sequence diagrams,
//! mind maps, Gantt charts, timelines, pie charts, quadrant charts, class diagrams, ER diagrams and XY charts, given
//! back as paths and words a client draws in its own font and colours, or as SVG.
mod class;
mod draw;
mod er;
mod flow;
mod gantt;
mod mind;
mod pie;
mod quadrant;
mod sequence;
mod timeline;
mod xychart;

pub use draw::{Diagram, Fill, Item};

/// The kinds of Mermaid diagram laid out here, as their first word is written.
pub const KINDS: [&str; 11] = [
    "flowchart", "sequenceDiagram", "stateDiagram", "mindmap", "gantt", "timeline", "pie", "quadrantChart",
    "classDiagram", "erDiagram", "xychart-beta",
];

/// A diagram from Mermaid source, or nothing when it is not a kind this lays out (the client shows the source).
pub fn diagram(source: &str) -> Option<Diagram> {
    let mut lines: Vec<&str> = Vec::new();
    let mut front = false;
    let mut front_title = None;
    // What it is called and what it shows, for someone who cannot see it: read here, so no kind takes them for content.
    let mut acc_title = None;
    let mut acc_descr: Option<Vec<String>> = None;
    let mut in_descr = false;
    for (i, raw) in source.lines().enumerate() {
        let line = raw.trim();
        if line == "---" && (i == 0 || front) { front = !front; continue; }
        if front { if let Some(t) = line.strip_prefix("title:") { front_title = Some(t.trim().trim_matches('"').to_string()); } continue; }
        if in_descr { if line == "}" { in_descr = false; } else if !line.is_empty() { acc_descr.get_or_insert_with(Vec::new).push(line.to_string()); } continue; }
        if let Some(t) = line.strip_prefix("accTitle") { acc_title = Some(t.trim_start_matches([':', ' ']).trim().to_string()).filter(|t| !t.is_empty()); continue; }
        if let Some(t) = line.strip_prefix("accDescr") {
            let t = t.trim_start_matches([':', ' ']).trim();
            if t == "{" { in_descr = true; } else if !t.is_empty() { acc_descr = Some(vec![t.to_string()]); }
            continue;
        }
        // Indentation is kept: a mind map's outline is written in it.
        let line = match raw.find("%%") { Some(at) => raw[..at].trim_end(), None => raw.trim_end() };
        if !line.trim().is_empty() { lines.push(line); }
    }
    let (header, raw) = lines.split_first()?;
    let header = header.trim();
    let kind = header.split_whitespace().next()?;
    let rest = header[kind.len()..].trim();
    let body: Vec<&str> = raw.iter().map(|l| l.trim()).collect();
    let body = &body[..];
    let mut d = match kind {
        "graph" | "flowchart" | "flowchart-elk" => flow::flowchart(rest, body),
        "stateDiagram" | "stateDiagram-v2" => flow::states(body),
        "sequenceDiagram" => sequence::sequence(body),
        "mindmap" => mind::mindmap(raw),
        "gantt" => gantt::gantt(body),
        "timeline" => timeline::timeline(body),
        "pie" => pie::pie(rest, body),
        "quadrantChart" => quadrant::quadrant(body),
        "classDiagram" => class::class(body),
        "erDiagram" => er::er(body),
        "xychart-beta" | "xychart" => xychart::xychart(body),
        _ => None,
    }?;
    // What it is called: the accessible title the source gives, else its title, else the first words drawn in it.
    let titled = body.iter().find_map(|l| l.strip_prefix("title ")).map(|t| t.trim().to_string())
        .or_else(|| rest.strip_prefix("title").map(|t| t.trim().to_string())).filter(|t| !t.is_empty());
    d.title = acc_title.or(front_title).or(titled).unwrap_or_else(|| {
        // A label broken into lines is one phrase again: its lines are written one under another at the same place.
        let mut phrases: Vec<(String, f32, f32)> = Vec::new();
        for item in &d.items {
            let Item::Text { text, x, y, .. } = item else { continue };
            if text.trim().is_empty() { continue; }
            match phrases.last_mut() {
                Some((phrase, px, py)) if (*px - x).abs() < 0.2 && (y - *py - draw::LINE).abs() < 0.2 => {
                    let spaced = phrase.ends_with(|c: char| c.is_ascii_alphanumeric()) && text.starts_with(|c: char| c.is_ascii_alphanumeric());
                    if spaced { phrase.push(' '); }
                    phrase.push_str(text);
                    *py = *y;
                }
                _ => phrases.push((text.clone(), *x, *y)),
            }
        }
        let mut words: Vec<&str> = Vec::new();
        for (phrase, _, _) in &phrases { if !words.contains(&phrase.as_str()) { words.push(phrase); } }
        let joined = words.iter().take(4).copied().collect::<Vec<_>>().join(" → ");
        if joined.chars().count() > 120 { joined.chars().take(119).chain(['…']).collect() } else { joined }
    });
    d.description = acc_descr.map(|lines| lines.join(" "));
    Some(d)
}

/// The diagram as JSON, for clients that draw it themselves.
pub fn json(source: &str) -> Option<String> { diagram(source).map(|d| serde_json::to_string(&d).expect("a diagram serializes")) }

/// The diagram as SVG in pixels of 14px text, in the colour of the text around it.
pub fn svg(source: &str) -> Option<String> { diagram(source).map(|d| draw::svg(&d)) }
