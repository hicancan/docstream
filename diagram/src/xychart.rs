//! XY charts: lines and bars over a numeric or categorical x-axis and a numeric y-axis.
use crate::draw::{dot, label, r, rect, width, wrap, Diagram, Fill, Item, Pen, LINE};
use crate::flow::{finish, Bounds};

enum SeriesKind { Line, Bar }
struct Series { kind: SeriesKind, name: Option<String>, values: Vec<f64> }

pub fn xychart(body: &[&str]) -> Option<Diagram> {
    let mut title = None;
    let mut x_title = None;
    let mut x_cats: Vec<String> = Vec::new();
    let mut x_range: Option<(f64, f64)> = None;
    let mut y_title = None;
    let mut y_range: Option<(f64, f64)> = None;
    let mut series: Vec<Series> = Vec::new();

    for raw in body {
        let line = raw.trim();
        let (word, rest) = line.split_once(char::is_whitespace).map(|(w, r)| (w, r.trim())).unwrap_or((line, ""));
        match word {
            "title" => { title = Some(unquote(rest)); }
            "x-axis" => {
                if let Some((q, tail)) = split_quoted(rest) {
                    x_title = Some(q);
                    if let Some(cats) = parse_bracket_strings(tail) { x_cats = cats; }
                    else if let Some(rng) = parse_range(tail) { x_range = Some(rng); }
                } else if let Some(cats) = parse_bracket_strings(rest) {
                    x_cats = cats;
                } else if let Some(rng) = parse_range(rest) {
                    x_range = Some(rng);
                } else if !rest.is_empty() {
                    x_title = Some(unquote(rest));
                }
            }
            "y-axis" => {
                if let Some((q, tail)) = split_quoted(rest) {
                    y_title = Some(q);
                    if let Some(rng) = parse_range(tail) { y_range = Some(rng); }
                } else if let Some(rng) = parse_range(rest) {
                    y_range = Some(rng);
                } else if !rest.is_empty() {
                    y_title = Some(unquote(rest));
                }
            }
            "line" | "bar" => {
                let kind = if word == "bar" { SeriesKind::Bar } else { SeriesKind::Line };
                let (name, arr) = match split_quoted(rest) {
                    Some((q, tail)) => (Some(q), tail),
                    None => (None, rest),
                };
                if let Some(values) = parse_bracket_numbers(arr) {
                    if !values.is_empty() { series.push(Series { kind, name, values }); }
                }
            }
            _ => {}
        }
    }
    if series.is_empty() { return None; }

    let count = series.iter().map(|s| s.values.len()).max().unwrap_or(0).max(x_cats.len());
    if count == 0 { return None; }
    let (mut ymin, mut ymax) = y_range.unwrap_or_else(|| {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for s in &series { for &v in &s.values { lo = lo.min(v); hi = hi.max(v); } }
        if lo > 0.0 && series.iter().any(|s| matches!(s.kind, SeriesKind::Bar)) { lo = 0.0; }
        if (hi - lo).abs() < 1e-9 { (lo - 1.0, hi + 1.0) } else { (lo, hi) }
    });
    if ymax <= ymin { ymax = ymin + 1.0; }

    let step = nice_step((ymax - ymin) / 5.0);
    ymin = (ymin / step).floor() * step;
    ymax = (ymax / step).ceil() * step;
    let mut y_ticks = Vec::new();
    let mut v = ymin;
    while v <= ymax + step * 1e-6 { y_ticks.push(v); v += step; }

    let y_label_w = y_ticks.iter().map(|&t| width(&fmt_num(t), 12.0)).fold(0.0_f32, f32::max);
    let left = 18.0 + y_label_w + if y_title.is_some() { 20.0 } else { 0.0 };
    let x_cats_display: Vec<String> = if !x_cats.is_empty() { x_cats.clone() }
        else if let Some((a, b)) = x_range {
            (0..count).map(|i| fmt_num(if count == 1 { a } else { a + (b - a) * (i as f64 / (count - 1) as f64) })).collect()
        } else { (1..=count).map(|i| i.to_string()).collect() };
    let max_cat_w = x_cats_display.iter().map(|s| width(s, 12.0)).fold(0.0_f32, f32::max);
    let slot_w = (max_cat_w + 22.0).max(60.0);
    let plot_w = (slot_w * count as f32).clamp(320.0, 720.0);
    let plot_h = 220.0_f32;

    let mut items = Vec::new();
    let mut b = Bounds::default();
    let mut top = 8.0_f32;
    if let Some(title) = &title {
        let lines = wrap(title, plot_w + left);
        label(&mut items, &lines, left + plot_w / 2.0, top + LINE * lines.len() as f32 / 2.0, true);
        top += LINE * lines.len() as f32 + 12.0;
    }
    if let Some(yt) = &y_title {
        items.push(Item::Text { x: r(left - 8.0), y: r(top - 2.0), text: yt.clone(), size: 12.0, bold: false });
        top += 12.0;
    }

    let bottom = top + plot_h;
    let right = left + plot_w;
    let y_of = |val: f64| -> f32 { bottom - ((val - ymin) / (ymax - ymin)) as f32 * plot_h };

    // Grid and y ticks.
    for &t in &y_ticks {
        let yy = y_of(t);
        items.push(Item::Path { d: Pen::new().m(left, yy).l(right, yy).done(), fill: Fill::None, stroke: 0.6, dash: true });
        let txt = fmt_num(t);
        let tw = width(&txt, 12.0);
        items.push(Item::Text { x: r(left - 8.0 - tw / 2.0), y: r(yy), text: txt, size: 12.0, bold: false });
    }
    // Axes.
    items.push(Item::Path { d: Pen::new().m(left, top).l(left, bottom).l(right, bottom).done(), fill: Fill::None, stroke: 1.4, dash: false });

    let has_bars = series.iter().any(|s| matches!(s.kind, SeriesKind::Bar));
    let x_of = |i: usize| -> f32 {
        if has_bars || !x_cats.is_empty() {
            left + (i as f32 + 0.5) * (plot_w / count as f32)
        } else if count == 1 {
            left + plot_w / 2.0
        } else {
            left + i as f32 * (plot_w / (count - 1) as f32)
        }
    };

    for (i, cat) in x_cats_display.iter().enumerate() {
        let xx = x_of(i);
        items.push(Item::Path { d: Pen::new().m(xx, bottom).l(xx, bottom + 4.0).done(), fill: Fill::None, stroke: 1.0, dash: false });
        items.push(Item::Text { x: r(xx), y: r(bottom + 14.0), text: cat.clone(), size: 12.0, bold: false });
    }

    // Bars first, then lines over them.
    let bar_count = series.iter().filter(|s| matches!(s.kind, SeriesKind::Bar)).count();
    let group_w = (plot_w / count as f32) * 0.65;
    let bar_w = if bar_count > 0 { (group_w / bar_count as f32).min(36.0) } else { 0.0 };
    let mut bar_idx = 0usize;
    let mut line_idx = 0usize;
    let base_y = y_of(ymin.max(0.0).min(ymax));

    for s in &series {
        match s.kind {
            SeriesKind::Bar => {
                let offset = (bar_idx as f32 - (bar_count as f32 - 1.0) / 2.0) * (bar_w + 2.0);
                for (i, &val) in s.values.iter().enumerate() {
                    let cx = x_of(i) + offset;
                    let yy = y_of(val);
                    let (y0, h) = if yy <= base_y { (yy, (base_y - yy).max(1.0)) } else { (base_y, (yy - base_y).max(1.0)) };
                    items.push(Item::Path { d: rect(cx - bar_w / 2.0, y0, bar_w, h, 2.0), fill: Fill::Soft, stroke: 1.4, dash: false });
                }
                bar_idx += 1;
            }
            SeriesKind::Line => {
                let mut pen = Pen::new();
                for (i, &val) in s.values.iter().enumerate() {
                    let (xx, yy) = (x_of(i), y_of(val));
                    pen = if i == 0 { pen.m(xx, yy) } else { pen.l(xx, yy) };
                }
                let dash = line_idx % 2 == 1;
                let stroke = if line_idx == 0 { 2.0 } else { 1.6 };
                items.push(Item::Path { d: pen.done(), fill: Fill::None, stroke, dash });
                for (i, &val) in s.values.iter().enumerate() {
                    let fill = if line_idx == 0 { Fill::Solid } else { Fill::Soft };
                    items.push(dot(x_of(i), y_of(val), 3.6, fill));
                }
                line_idx += 1;
            }
        }
    }

    let mut cur_y = bottom + 26.0;
    if let Some(xt) = &x_title {
        items.push(Item::Text { x: r(left + plot_w / 2.0), y: r(cur_y + 6.0), text: xt.clone(), size: 12.0, bold: false });
        cur_y += 18.0;
    }

    // Legend for named series.
    let named: Vec<&Series> = series.iter().filter(|s| s.name.is_some()).collect();
    if !named.is_empty() {
        cur_y += 6.0;
        let entries: Vec<(String, usize, bool)> = series.iter().enumerate()
            .filter_map(|(i, s)| s.name.as_ref().map(|n| (n.clone(), i, matches!(s.kind, SeriesKind::Bar))))
            .collect();
        let total_w: f32 = entries.iter().map(|(n, _, _)| 28.0 + width(n, 12.0) + 18.0).sum::<f32>() - 18.0;
        let mut lx = (left + plot_w / 2.0 - total_w / 2.0).max(8.0);
        let mut l_num = 0usize;
        for s in &series {
            let is_line = matches!(s.kind, SeriesKind::Line);
            if let Some(name) = &s.name {
                if is_line {
                    items.push(Item::Path { d: Pen::new().m(lx, cur_y).l(lx + 20.0, cur_y).done(), fill: Fill::None, stroke: if l_num == 0 { 2.0 } else { 1.6 }, dash: l_num % 2 == 1 });
                    items.push(dot(lx + 10.0, cur_y, 3.2, if l_num == 0 { Fill::Solid } else { Fill::Soft }));
                } else {
                    items.push(Item::Path { d: rect(lx, cur_y - 5.0, 16.0, 10.0, 2.0), fill: Fill::Soft, stroke: 1.4, dash: false });
                }
                let tw = width(name, 12.0);
                items.push(Item::Text { x: r(lx + 26.0 + tw / 2.0), y: r(cur_y), text: name.clone(), size: 12.0, bold: false });
                lx += 28.0 + tw + 18.0;
            }
            if is_line { l_num += 1; }
        }
        cur_y += 16.0;
    }

    b.add(0.0, 0.0);
    b.add(right + 16.0, cur_y);
    Some(finish(items, b))
}

fn unquote(s: &str) -> String { s.trim().trim_matches('"').trim_matches('\'').to_string() }

fn split_quoted(s: &str) -> Option<(String, &str)> {
    let s = s.trim();
    let q = s.chars().next()?;
    if q != '"' && q != '\'' { return None; }
    let end = s[1..].find(q)? + 1;
    Some((s[1..end].to_string(), s[end + 1..].trim()))
}

fn parse_bracket_strings(s: &str) -> Option<Vec<String>> {
    let a = s.find('[')?; let b = s.rfind(']')?;
    if b <= a { return None; }
    Some(s[a + 1..b].split(',').map(unquote).filter(|x| !x.is_empty()).collect())
}

fn parse_bracket_numbers(s: &str) -> Option<Vec<f64>> {
    let a = s.find('[')?; let b = s.rfind(']')?;
    if b <= a { return None; }
    s[a + 1..b].split(',').map(|x| x.trim().parse::<f64>().ok()).collect()
}

fn parse_range(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once("-->")?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

fn nice_step(raw: f64) -> f64 {
    let exp = 10.0_f64.powf(raw.abs().max(1e-9).log10().floor());
    let f = raw / exp;
    let nice = if f <= 1.0 { 1.0 } else if f <= 2.0 { 2.0 } else if f <= 5.0 { 5.0 } else { 10.0 };
    nice * exp
}

fn fmt_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 { format!("{}", v.round() as i64) } else { format!("{:.1}", v) }
}
