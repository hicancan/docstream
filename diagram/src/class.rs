//! Mermaid class diagrams: classes with stereotypes, fields and methods, joined by UML relations on the shared
//! layered graph layout.
use crate::draw::Diagram;
use crate::flow::{direction, layout, Edge, End, Graph, Head, Line};
use std::collections::HashMap;

struct ClassInfo { title: String, stereotype: Option<String>, fields: Vec<String>, methods: Vec<String> }

pub fn class(body: &[&str]) -> Option<Diagram> {
    let mut g = Graph::default();
    let mut classes: HashMap<String, ClassInfo> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut open: Option<String> = None;
    let mut is_iface = false;

    let ensure = |id: &str, title: Option<String>, stereo: Option<String>, classes: &mut HashMap<String, ClassInfo>, order: &mut Vec<String>| {
        if !classes.contains_key(id) {
            order.push(id.to_string());
            classes.insert(id.to_string(), ClassInfo { title: title.clone().unwrap_or_else(|| id.to_string()), stereotype: stereo.clone(), fields: Vec::new(), methods: Vec::new() });
        } else {
            let c = classes.get_mut(id).unwrap();
            if let Some(t) = title { c.title = t; }
            if let Some(s) = stereo { c.stereotype = Some(s); }
        }
    };

    for raw in body {
        let s = raw.trim();
        if s.is_empty() { continue; }
        if let Some(cur) = open.clone() {
            if s == "}" { open = None; is_iface = false; continue; }
            let c = classes.get_mut(&cur).unwrap();
            if s.starts_with("<<") && s.ends_with(">>") {
                c.stereotype = Some(s.to_string());
            } else if s.contains('(') && s.contains(')') {
                c.methods.push(s.to_string());
            } else {
                c.fields.push(s.to_string());
            }
            if is_iface && c.stereotype.is_none() { c.stereotype = Some("<<interface>>".into()); }
            continue;
        }

        if let Some(rest) = s.strip_prefix("direction ") {
            g.dir = direction(rest).or(g.dir);
            continue;
        }
        if s.starts_with("namespace ") || s == "}" || s.starts_with("note ") || s.starts_with("style ") || s.starts_with("classDef ") || s.starts_with("link ") {
            continue;
        }
        if let Some(rest) = s.strip_prefix("class ").or_else(|| s.strip_prefix("interface ")) {
            let iface = s.starts_with("interface ");
            let (head, opens) = match rest.strip_suffix('{') { Some(h) => (h.trim(), true), None => (rest.trim(), false) };
            let (id, title) = class_name(head);
            ensure(&id, title, iface.then(|| "<<interface>>".into()), &mut classes, &mut order);
            if opens { open = Some(id); is_iface = iface; }
            continue;
        }

        if let Some(rel) = parse_relation(s) {
            ensure(&rel.a, None, None, &mut classes, &mut order);
            let na = g.node(&rel.a, None, None);
            // Association class link: `Enrollment .. "选课"` connects to the target of the named relation if present.
            let target_edge = if rel.b.starts_with('"') && rel.b.ends_with('"') {
                let want = rel.b.trim_matches('"');
                g.edges.iter().find(|e| e.label == want).map(|e| e.to)
            } else { None };
            let nb = match target_edge {
                Some(End::Node(v)) => v,
                _ => {
                    let clean_b = rel.b.trim_matches('"');
                    ensure(clean_b, None, None, &mut classes, &mut order);
                    g.node(clean_b, None, None)
                }
            };
            g.edges.push(Edge { from: End::Node(na), to: End::Node(nb), label: rel.label, from_label: rel.mult_a, to_label: rel.mult_b, line: rel.line, head: rel.head, tail: rel.tail });
            continue;
        }

        if let Some((owner, member)) = s.split_once(':') {
            let owner = owner.trim();
            let member = member.trim();
            if !owner.is_empty() && !owner.contains(' ') {
                ensure(owner, None, None, &mut classes, &mut order);
                let c = classes.get_mut(owner).unwrap();
                if member.contains('(') { c.methods.push(member.to_string()); } else if !member.is_empty() { c.fields.push(member.to_string()); }
            }
        }
    }

    for id in order {
        let info = classes.remove(&id).unwrap();
        let v = g.node(&id, Some(info.title.clone()), None);
        let mut header = Vec::new();
        if let Some(st) = info.stereotype { header.push(st); }
        header.push(info.title);
        let mut sections = vec![header];
        if !info.fields.is_empty() || !info.methods.is_empty() {
            sections.push(info.fields);
            if !info.methods.is_empty() { sections.push(info.methods); }
        }
        g.nodes[v].sections = sections;
    }

    (!g.nodes.is_empty()).then(|| layout(&g))
}

fn class_name(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if let (Some(a), Some(b)) = (s.find('['), s.rfind(']')) {
        if b > a {
            return (s[..a].trim().to_string(), Some(s[a + 1..b].trim().trim_matches('"').to_string()));
        }
    }
    (s.to_string(), None)
}

struct Relation { a: String, mult_a: String, line: Line, tail: Head, head: Head, mult_b: String, b: String, label: String }

fn parse_relation(s: &str) -> Option<Relation> {
    const OPS: &[(&str, Line, Head, Head)] = &[
        ("<|..", Line::Dotted, Head::Triangle, Head::None),
        ("..|>", Line::Dotted, Head::None, Head::Triangle),
        ("<|--", Line::Solid, Head::Triangle, Head::None),
        ("--|>", Line::Solid, Head::None, Head::Triangle),
        ("<..", Line::Dotted, Head::Open, Head::None),
        ("..>", Line::Dotted, Head::None, Head::Open),
        ("<--", Line::Solid, Head::Open, Head::None),
        ("-->", Line::Solid, Head::None, Head::Open),
        ("*--", Line::Solid, Head::FilledDiamond, Head::None),
        ("--*", Line::Solid, Head::None, Head::FilledDiamond),
        ("o--", Line::Solid, Head::Diamond, Head::None),
        ("--o", Line::Solid, Head::None, Head::Diamond),
        ("*..", Line::Dotted, Head::FilledDiamond, Head::None),
        ("..*", Line::Dotted, Head::None, Head::FilledDiamond),
        ("o..", Line::Dotted, Head::Diamond, Head::None),
        ("..o", Line::Dotted, Head::None, Head::Diamond),
        ("--", Line::Solid, Head::None, Head::None),
        ("..", Line::Dotted, Head::None, Head::None),
    ];
    let (main, label) = match s.split_once(':') { Some((m, l)) => (m.trim(), l.trim().to_string()), None => (s, String::new()) };
    let mut best: Option<(usize, &str, Line, Head, Head)> = None;
    for &(op, line, tail, head) in OPS {
        if let Some(pos) = main.find(op) {
            if best.is_none_or(|(bp, bo, ..)| pos < bp || (pos == bp && op.len() > bo.len())) {
                best = Some((pos, op, line, tail, head));
            }
        }
    }
    let (pos, op, line, tail, head) = best?;
    let left = main[..pos].trim();
    let right = main[pos + op.len()..].trim();
    let (a, mult_a) = take_end_quote(left);
    let (mult_b, b) = take_start_quote(right);
    if a.is_empty() || b.is_empty() { return None; }
    Some(Relation { a, mult_a, line, tail, head, mult_b, b, label })
}

fn take_end_quote(s: &str) -> (String, String) {
    let s = s.trim();
    if let Some(rest) = s.strip_suffix('"') {
        if let Some(q) = rest.rfind('"') {
            return (rest[..q].trim().to_string(), rest[q + 1..].to_string());
        }
    }
    (s.to_string(), String::new())
}

fn take_start_quote(s: &str) -> (String, String) {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(q) = rest.find('"') {
            let after = rest[q + 1..].trim();
            if !after.is_empty() { return (rest[..q].to_string(), after.to_string()); }
        }
    }
    (String::new(), s.to_string())
}
