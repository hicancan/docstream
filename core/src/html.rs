//! The part of HTML a document may write, read as the same tree Markdown is read into.
//!
//! Some things Markdown says badly: a table whose cells span rows or columns, a subscript in a chemical formula.
//! Documents (and the OCR that made many of them) write those in HTML, so a small part of HTML is read as what it
//! means:
//!
//! - in text: `<sub>` `<sup>` `<b>` `<strong>` `<i>` `<em>` `<s>` `<del>` `<mark>` `<code>` `<kbd>` `<a href>` `<img src alt>`
//!   `<br>`, each read as the inline Markdown has for it;
//! - as a block: `<table>` with `<thead>` `<tbody>` `<tfoot>` `<tr>` `<th>` `<td>`, cells spanning with `rowspan` and
//!   `colspan`, each cell's content read as Markdown text (formulas in cells are common).
//!
//! Nothing else is markup: any other tag stays the text it is, and a block holding one is kept as written. Other
//! attributes say how a thing looks, not what it is, and are passed over; none of them can run script.
use crate::tree::{Align, Block, Cell, Inline};
use pulldown_cmark::{CowStr, Event, LinkType, Tag, TagEnd};
use std::ops::Range;

/// A tag as written: its name in lower case, whether it closes, its attributes, and how long it is.
pub(crate) struct Element { pub name: String, pub close: bool, attrs: Vec<(String, String)>, pub len: usize }

impl Element {
    fn attr(&self, name: &str) -> Option<&str> { self.attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str()) }
}

/// The tag starting `s`, if `s` starts with one.
pub(crate) fn element(s: &str) -> Option<Element> {
    let b = s.as_bytes();
    if b.first() != Some(&b'<') { return None; }
    let mut i = 1;
    let close = b.get(i) == Some(&b'/');
    if close { i += 1; }
    let start = i;
    if !b.get(i)?.is_ascii_alphabetic() { return None; }
    while b.get(i).is_some_and(|c| c.is_ascii_alphanumeric()) { i += 1; }
    let name = s[start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        let before = i;
        while b.get(i).is_some_and(|c| c.is_ascii_whitespace()) { i += 1; }
        match b.get(i)? {
            b'>' => return Some(Element { name, close, attrs, len: i + 1 }),
            b'/' if b.get(i + 1) == Some(&b'>') && !close => return Some(Element { name, close, attrs, len: i + 2 }),
            _ if close || i == before => return None,
            _ => {}
        }
        let at = i;
        while b.get(i).is_some_and(|c| !c.is_ascii_whitespace() && !b"/>=\"'".contains(c)) { i += 1; }
        if i == at { return None; }
        let key = s[at..i].to_ascii_lowercase();
        let mut j = i;
        while b.get(j).is_some_and(|c| c.is_ascii_whitespace()) { j += 1; }
        let mut value = String::new();
        if b.get(j) == Some(&b'=') {
            j += 1;
            while b.get(j).is_some_and(|c| c.is_ascii_whitespace()) { j += 1; }
            match b.get(j)? {
                &q @ (b'"' | b'\'') => { let end = s[j + 1..].find(q as char)? + j + 1; value = entities(&s[j + 1..end]); j = end + 1; }
                _ => { let at = j; while b.get(j).is_some_and(|c| !c.is_ascii_whitespace() && *c != b'>') { j += 1; } value = entities(&s[at..j]); }
            }
            i = j;
        }
        attrs.push((key, value));
    }
}

/// The few character references an attribute is written with.
fn entities(s: &str) -> String {
    if !s.contains('&') { return s.to_string(); }
    s.replace("&quot;", "\"").replace("&#39;", "'").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

/// The inline tags read as markup, and what each is.
fn pair(name: &str) -> Option<fn(&Element) -> Option<Tag<'static>>> {
    Some(match name {
        "b" | "strong" => |_| Some(Tag::Strong),
        "i" | "em" => |_| Some(Tag::Emphasis),
        "s" | "del" => |_| Some(Tag::Strikethrough),
        "mark" => |_| Some(Tag::Highlight),
        "sup" => |_| Some(Tag::Superscript),
        "sub" => |_| Some(Tag::Subscript),
        "a" => |e| e.attr("href").map(|href| Tag::Link { link_type: LinkType::Inline, dest_url: CowStr::from(href.to_string()), title: "".into(), id: "".into() }),
        _ => return None,
    })
}

/// Whether a tag is one that text may use.
fn inline_tag(e: &Element) -> bool {
    match e.name.as_str() {
        "br" => !e.close,
        "img" => !e.close && e.attr("src").is_some(),
        "code" | "kbd" => true,
        name => pair(name).is_some_and(|tag| e.close || tag(e).is_some()),
    }
}

/// The tags among inline events read as what they mean: a tag that opens and the one that closes it, within the same
/// container, become that inline's start and end; `<br>` a break, `<img>` an image, `<code>` a code span. What does
/// not pair stays text.
pub(crate) fn inline(events: Vec<(Event<'_>, Range<usize>)>) -> Vec<(Event<'_>, Range<usize>)> {
    if !events.iter().any(|(e, _)| matches!(e, Event::InlineHtml(_))) { return events; }
    let mut out: Vec<(Event, Range<usize>)> = Vec::with_capacity(events.len());
    // Tags opened and not yet closed: the name, where the open tag stands in `out`, and the depth it was opened at.
    let mut open: Vec<(String, usize, usize)> = Vec::new();
    let mut depth = 0usize;
    let mut i = 0;
    while i < events.len() {
        let (event, range) = events[i].clone();
        i += 1;
        match &event {
            Event::Start(_) => depth += 1,
            Event::End(_) => { depth = depth.saturating_sub(1); open.retain(|&(_, _, d)| d <= depth); }
            Event::InlineHtml(raw) => if let Some(e) = element(raw).filter(|e| e.len == raw.trim_end().len()) {
                match (e.name.as_str(), e.close) {
                    ("br", false) => { out.push((Event::HardBreak, range)); continue; }
                    ("img", false) => if let Some(src) = e.attr("src") {
                        out.push((Event::Start(Tag::Image { link_type: LinkType::Inline, dest_url: CowStr::from(src.to_string()), title: "".into(), id: "".into() }), range.clone()));
                        if let Some(alt) = e.attr("alt").filter(|a| !a.is_empty()) { out.push((Event::Text(CowStr::from(alt.to_string())), range.clone())); }
                        out.push((Event::End(TagEnd::Image), range));
                        continue;
                    },
                    ("code" | "kbd", false) => {
                        // Its text, to the tag that closes it.
                        let mut text = String::new();
                        let mut j = i;
                        let closed = loop {
                            match events.get(j).map(|(e, _)| e) {
                                Some(Event::Text(t) | Event::Code(t)) => text.push_str(t),
                                Some(Event::SoftBreak) => text.push(' '),
                                Some(Event::InlineHtml(t)) if element(t).is_some_and(|c| c.close && c.name == e.name) => break true,
                                Some(Event::InlineHtml(t)) => text.push_str(t),
                                _ => break false,
                            }
                            j += 1;
                        };
                        if closed { out.push((Event::Code(CowStr::from(text)), range.start..events[j].1.end)); i = j + 1; continue; }
                    }
                    (name, false) => if pair(name).and_then(|tag| tag(&e)).is_some() { open.push((e.name.clone(), out.len(), depth)); },
                    (name, true) => if let Some(at) = open.iter().rposition(|(n, _, d)| n == name && *d == depth) {
                        let (_, start, _) = open[at];
                        open.truncate(at);
                        let tag = match &out[start].0 { Event::InlineHtml(raw) => element(raw).and_then(|o| pair(name).and_then(|tag| tag(&o))), _ => None };
                        if let Some(tag) = tag {
                            let end = tag.to_end();
                            out[start].0 = Event::Start(tag);
                            out.push((Event::End(end), range));
                            continue;
                        }
                    },
                }
            },
            _ => {}
        }
        out.push((event, range));
    }
    out
}

/// Whether raw HTML holds only what text may use, to be read as a paragraph.
pub(crate) fn only_inline(src: &str) -> bool {
    let mut any = false;
    let mut i = 0;
    while let Some(at) = src[i..].find('<') {
        i += at;
        match element(&src[i..]) {
            Some(e) if inline_tag(&e) => { any = true; i += e.len; }
            Some(_) => return false,
            None => i += 1,
        }
    }
    any
}

struct RawRow { head: bool, cells: Vec<RawCell> }
struct RawCell { header: bool, align: Align, rows: u32, cols: u32, source: Range<usize> }

/// An HTML table, read as a table: its cells with their spans and the column each starts in, each cell's content read
/// by `content`; and where what follows `</table>` begins, for text written straight after it (a caption on the next
/// line ends up in the same block of HTML, which a browser shows after the table). None when it does not start with
/// one table, or holds another tag outside its cells.
pub(crate) fn table(src: &str, content: impl Fn(&str) -> Vec<Inline>) -> Option<(Block, usize)> {
    let mut rows: Vec<RawRow> = Vec::new();
    let mut row: Option<RawRow> = None;
    let mut cell: Option<(RawCell, usize)> = None;
    let (mut seen, mut in_head) = (false, false);
    let mut rest = src.len();
    let b = src.as_bytes();
    let mut i = 0;
    let finish_cell = |cell: &mut Option<(RawCell, usize)>, row: &mut Option<RawRow>, at: usize| if let Some((mut c, start)) = cell.take() {
        c.source = start..at;
        if let Some(row) = row.as_mut() { row.cells.push(c); }
    };
    while i < b.len() {
        if src[i..].starts_with("<!--") { i += src[i..].find("-->").map_or(src.len() - i, |e| e + 3); continue; }
        if b[i] == b'<' {
            if let Some(e) = element(&src[i..]) {
                if matches!(e.name.as_str(), "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th") {
                    finish_cell(&mut cell, &mut row, i);
                    match (e.name.as_str(), e.close) {
                        ("table", false) => { if seen { return None; } seen = true; }
                        ("table", true) => { if let Some(r) = row.take() { rows.push(r); } rest = i + e.len; break; }
                        ("thead", close) => { if let Some(r) = row.take() { rows.push(r); } in_head = !close; }
                        ("tbody" | "tfoot", _) => { if let Some(r) = row.take() { rows.push(r); } in_head = false; }
                        ("tr", false) => { if let Some(r) = row.take() { rows.push(r); } row = Some(RawRow { head: in_head, cells: Vec::new() }); }
                        ("tr", true) => { if let Some(r) = row.take() { rows.push(r); } }
                        (name, false) => {
                            if !seen { return None; }
                            if row.is_none() { row = Some(RawRow { head: in_head, cells: Vec::new() }); }
                            let span = |key: &str| e.attr(key).and_then(|v| v.trim().parse::<u32>().ok());
                            let align = match e.attr("align").map(str::to_ascii_lowercase).as_deref() { Some("left") => Align::Left, Some("center") => Align::Center, Some("right") => Align::Right, _ => Align::None };
                            cell = Some((RawCell { header: name == "th", align, rows: span("rowspan").unwrap_or(1), cols: span("colspan").unwrap_or(1).clamp(1, 1000), source: 0..0 }, i + e.len));
                        }
                        _ => {}
                    }
                    i += e.len;
                    continue;
                }
                // Inside a cell it is the cell's content; outside one, no table.
                if cell.is_some() { i += e.len; continue; }
                return None;
            }
        }
        let ch = src[i..].chars().next()?;
        if cell.is_none() && !ch.is_whitespace() { return None; }
        i += ch.len_utf8();
    }
    // A table still being written, or written without its closing tags, ends where the text does.
    finish_cell(&mut cell, &mut row, src.len());
    if let Some(r) = row.take() { rows.push(r); }
    if !seen { return None; }
    // Without a <thead>, the leading rows of header cells alone are the head.
    if !rows.iter().any(|r| r.head) {
        for r in rows.iter_mut() { if !r.cells.is_empty() && r.cells.iter().all(|c| c.header) { r.head = true; } else { break; } }
    }
    let split = rows.iter().take_while(|r| r.head).count();
    let body = rows.split_off(split);
    let mut columns = 0;
    let mut place = |rows: Vec<RawRow>| -> Vec<Vec<Cell>> {
        // Which places earlier rows' cells still hold.
        let mut held: Vec<Vec<bool>> = vec![Vec::new(); rows.len()];
        let count = rows.len();
        rows.into_iter().enumerate().map(|(r, row)| {
            let mut column = 0usize;
            row.cells.into_iter().map(|c| {
                while held[r].get(column).copied().unwrap_or(false) { column += 1; }
                let left = (count - r) as u32;
                let row_span = if c.rows == 0 { left } else { c.rows.min(left) };
                let col_span = c.cols;
                for held_row in held.iter_mut().skip(r).take(row_span as usize) {
                    if held_row.len() < column + col_span as usize { held_row.resize(column + col_span as usize, false); }
                    for place in &mut held_row[column..column + col_span as usize] { *place = true; }
                }
                let cell = Cell { inlines: content(&src[c.source]), column: column as u32, row_span, col_span, header: c.header, align: c.align };
                column += col_span as usize;
                columns = columns.max(column as u32);
                cell
            }).collect()
        }).collect()
    };
    let head = place(rows);
    let body = place(body);
    Some((Block::Table { columns, head, body }, rest))
}
