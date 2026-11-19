//! Quadrant charts: a square cut in four by its axes, each quarter named, and points placed by two shares from 0 to 1,
//! each named where its words cover nothing else.
use crate::draw::{dot, label, lines_size, rect, wrap, Diagram, Fill, Item, Pen};
use crate::flow::{finish, Bounds};

/// The side of the square.
const SIDE: f32 = 440.0;

pub fn quadrant(body: &[&str]) -> Option<Diagram> {
    let unquote = |s: &str| s.trim().trim_matches('"').trim().to_string();
    let axis = |s: &str| -> (String, Option<String>) {
        match s.split_once("-->") { Some((a, b)) => (unquote(a), Some(unquote(b)).filter(|b| !b.is_empty())), None => (unquote(s), None) }
    };
    let mut title = None;
    let mut x_axis: Option<(String, Option<String>)> = None;
    let mut y_axis: Option<(String, Option<String>)> = None;
    let mut quarters: [Option<String>; 4] = Default::default();
    let mut points: Vec<(String, f32, f32)> = Vec::new();
    for line in body {
        let line = line.trim();
        let (word, rest) = line.split_once(char::is_whitespace).map(|(w, r)| (w, r.trim())).unwrap_or((line, ""));
        match word {
            "title" => { title = Some(rest.to_string()); continue; }
            "x-axis" => { x_axis = Some(axis(rest)); continue; }
            "y-axis" => { y_axis = Some(axis(rest)); continue; }
            "quadrant-1" | "quadrant-2" | "quadrant-3" | "quadrant-4" => { quarters[(word.as_bytes()[9] - b'1') as usize] = Some(unquote(rest)); continue; }
            "classDef" | "accTitle" | "accDescr" => continue,
            _ => {}
        }
        // `Name: [x, y]`, the name perhaps quoted and followed by `:::class`, the point perhaps by its style.
        let Some(open) = line.find('[') else { continue };
        let Some(close) = line[open..].find(']').map(|c| open + c) else { continue };
        let name = line[..open].trim_end().trim_end_matches(':');
        let name = unquote(name.split(":::").next().unwrap_or(name));
        let shares: Vec<f32> = line[open + 1..close].split(',').filter_map(|v| v.trim().parse().ok()).collect();
        let [x, y] = shares[..] else { continue };
        if !name.is_empty() && x.is_finite() && y.is_finite() { points.push((name, x.clamp(0.0, 1.0), y.clamp(0.0, 1.0))); }
    }
    if points.is_empty() && quarters.iter().all(Option::is_none) && x_axis.is_none() && y_axis.is_none() { return None; }

    let mut items = Vec::new();
    let mut b = Bounds::default();
    let half = SIDE / 2.0;
    // Quarters as Mermaid numbers them: 1 top right, 2 top left, 3 bottom left, 4 bottom right.
    let corners = [(half, 0.0), (0.0, 0.0), (0.0, half), (half, half)];
    for &(x0, y0) in &corners { items.push(Item::Path { d: rect(x0, y0, half, half, 0.0), fill: Fill::Soft, stroke: 0.0, dash: false }); }
    items.push(Item::Path { d: rect(0.0, 0.0, SIDE, SIDE, 4.0), fill: Fill::None, stroke: 1.4, dash: false });
    items.push(Item::Path { d: Pen::new().m(half, 0.0).l(half, SIDE).m(0.0, half).l(SIDE, half).done(), fill: Fill::None, stroke: 1.0, dash: false });
    b.add(0.0, 0.0); b.add(SIDE, SIDE);

    // What is already written, so a point's name finds room among it.
    let mut taken: Vec<[f32; 4]> = Vec::new();
    for (i, quarter) in quarters.iter().enumerate() {
        let Some(text) = quarter else { continue };
        let lines = wrap(text, half - 28.0);
        let (w, h) = lines_size(&lines);
        let (x0, y0) = corners[i];
        let (cx, cy) = (x0 + half / 2.0, y0 + half / 2.0);
        label(&mut items, &lines, cx, cy, true);
        taken.push([cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0]);
    }
    for (_, x, y) in &points { let (px, py) = (x * SIDE, (1.0 - y) * SIDE); taken.push([px - 6.0, py - 6.0, px + 6.0, py + 6.0]); }
    for (name, x, y) in &points {
        let (px, py) = (x * SIDE, (1.0 - y) * SIDE);
        items.push(dot(px, py, 5.0, Fill::Solid));
        let lines = wrap(name, 180.0);
        let (w, h) = lines_size(&lines);
        // Around the point in eight places, below first: the first that covers nothing and stays in the square, else
        // the one that covers least.
        let (dx, dy) = (12.0 + w / 2.0, 10.0 + h / 2.0);
        let places = [(px, py + dy), (px, py - dy), (px + dx, py), (px - dx, py),
            (px - 8.0 + w / 2.0, py + dy), (px + 8.0 - w / 2.0, py + dy), (px - 8.0 + w / 2.0, py - dy), (px + 8.0 - w / 2.0, py - dy)];
        let covered = |cx: f32, cy: f32| -> f32 {
            let r = [cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0];
            let over: f32 = taken.iter().map(|t| (r[2].min(t[2]) - r[0].max(t[0])).max(0.0) * (r[3].min(t[3]) - r[1].max(t[1])).max(0.0)).sum();
            let outside = ((0.0 - r[0]).max(0.0) + (r[2] - SIDE).max(0.0)) * h + ((0.0 - r[1]).max(0.0) + (r[3] - SIDE).max(0.0)) * w;
            over + outside
        };
        let (cx, cy) = places.iter().copied().min_by(|a, b| covered(a.0, a.1).total_cmp(&covered(b.0, b.1))).unwrap_or(places[0]);
        label(&mut items, &lines, cx, cy, false);
        taken.push([cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0]);
        b.add(cx - w / 2.0, cy - h / 2.0); b.add(cx + w / 2.0, cy + h / 2.0);
    }
    // The axes named along their halves: low to high, left to right and bottom to top.
    if let Some((low, high)) = &x_axis {
        let spots: Vec<(&String, f32)> = match high { Some(high) => vec![(low, half / 2.0), (high, half + half / 2.0)], None => vec![(low, half)] };
        for (text, cx) in spots {
            let lines = wrap(text, half - 16.0);
            let (w, h) = lines_size(&lines);
            label(&mut items, &lines, cx, SIDE + 10.0 + h / 2.0, false);
            b.add(cx - w / 2.0, SIDE + 10.0 + h);
        }
    }
    if let Some((low, high)) = &y_axis {
        let spots: Vec<(&String, f32)> = match high { Some(high) => vec![(low, half + half / 2.0), (high, half / 2.0)], None => vec![(low, half)] };
        for (text, cy) in spots {
            let lines = wrap(text, 120.0);
            let (w, h) = lines_size(&lines);
            let cx = -12.0 - w / 2.0;
            label(&mut items, &lines, cx, cy, false);
            b.add(cx - w / 2.0, cy - h / 2.0);
        }
    }
    if let Some(title) = &title {
        let lines = wrap(title, SIDE);
        let (w, h) = lines_size(&lines);
        label(&mut items, &lines, half, -12.0 - h / 2.0, true);
        b.add(half - w / 2.0, -12.0 - h); b.add(half + w / 2.0, -12.0);
    }
    Some(finish(items, b))
}
