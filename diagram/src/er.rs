//! Mermaid ER diagrams: entities with typed attributes and key markers, joined by cardinality relations on the
//! shared layered graph layout.
use crate::draw::Diagram;
use crate::flow::{direction, layout, Edge, End, Graph, Head, Line};
use std::collections::HashMap;

struct EntityInfo { title: String, attrs: Vec<String> }

pub fn er(body: &[&str]) -> Option<Diagram> {
    let mut g = Graph::default();
    let mut entities: HashMap<String, EntityInfo> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut open: Option<String> = None;

    let ensure = |id: &str, title: Option<String>, entities: &mut HashMap<String, EntityInfo>, order: &mut Vec<String>| {
        if !entities.contains_key(id) {
            order.push(id.to_string());
            entities.insert(id.to_string(), EntityInfo { title: title.clone().unwrap_or_else(|| id.to_string()), attrs: Vec::new() });
        } else if let Some(t) = title {
            entities.get_mut(id).unwrap().title = t;
        }
    };

    for raw in body {
        let s = raw.trim();
        if s.is_empty() { continue; }
        if let Some(cur) = open.clone() {
            if s == "}" { open = None; continue; }
            if let Some(line) = format_attr(s) {
                entities.get_mut(&cur).unwrap().attrs.push(line);
            }
            continue;
        }
        if let Some(rest) = s.strip_prefix("direction ") { g.dir = direction(rest).or(g.dir); continue; }
        if let Some(head) = s.strip_suffix('{') {
            if !head.contains("--") && !head.contains("..") {
                let (id, title) = entity_name(head.trim());
                if !id.is_empty() {
                    ensure(&id, title, &mut entities, &mut order);
                    open = Some(id);
                    continue;
                }
            }
        }
        if let Some((a, tail, line, head, b, label)) = parse_relation(s) {
            let (id_a, title_a) = entity_name(&a);
            let (id_b, title_b) = entity_name(&b);
            ensure(&id_a, title_a, &mut entities, &mut order);
            ensure(&id_b, title_b, &mut entities, &mut order);
            let na = g.node(&id_a, None, None);
            let nb = g.node(&id_b, None, None);
            g.edges.push(Edge { from: End::Node(na), to: End::Node(nb), label, from_label: String::new(), to_label: String::new(), line, head, tail });
        }
    }

    for id in order {
        let info = entities.remove(&id).unwrap();
        let v = g.node(&id, Some(info.title.clone()), None);
        let mut sections = vec![vec![info.title]];
        if !info.attrs.is_empty() { sections.push(info.attrs); }
        g.nodes[v].sections = sections;
    }

    (!g.nodes.is_empty()).then(|| layout(&g))
}

fn entity_name(s: &str) -> (String, Option<String>) {
    let s = s.trim();
    if let (Some(a), Some(b)) = (s.find('['), s.rfind(']')) {
        if b > a {
            return (s[..a].trim().to_string(), Some(s[a + 1..b].trim().trim_matches('"').to_string()));
        }
    }
    (s.trim_matches('"').to_string(), None)
}

fn format_attr(s: &str) -> Option<String> {
    let mut rest = s.trim();
    let comment = if let Some(q0) = rest.find('"') {
        if let Some(q1) = rest[q0 + 1..].find('"') {
            let c = rest[q0 + 1..q0 + 1 + q1].trim().to_string();
            rest = rest[..q0].trim();
            Some(c)
        } else { None }
    } else { None };
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.is_empty() { return None; }
    let mut out = parts.join(" ");
    if let Some(c) = comment { out.push_str(&format!(" ({c})")); }
    Some(out)
}

fn parse_relation(s: &str) -> Option<(String, Head, Line, Head, String, String)> {
    let (main, label) = match s.split_once(':') { Some((m, l)) => (m.trim(), l.trim().trim_matches('"').to_string()), None => (s, String::new()) };
    let (sep_at, line) = match (main.find("--"), main.find("..")) {
        (Some(a), Some(b)) => if a <= b { (a, Line::Solid) } else { (b, Line::Dotted) },
        (Some(a), None) => (a, Line::Solid),
        (None, Some(b)) => (b, Line::Dotted),
        (None, None) => return None,
    };
    let left = main[..sep_at].trim();
    let right = main[sep_at + 2..].trim();
    let (a, tail) = parse_left_card(left)?;
    let (head, b) = parse_right_card(right)?;
    Some((a, tail, line, head, b, label))
}

fn parse_left_card(s: &str) -> Option<(String, Head)> {
    for (pat, h) in [("||", Head::ErOne), ("|o", Head::ErZeroOne), ("o|", Head::ErZeroOne), ("}|", Head::ErMany), ("|{", Head::ErMany), ("}o", Head::ErZeroMany), ("o{", Head::ErZeroMany)] {
        if let Some(rest) = s.strip_suffix(pat) { return Some((rest.trim().to_string(), h)); }
    }
    None
}

fn parse_right_card(s: &str) -> Option<(Head, String)> {
    for (pat, h) in [("||", Head::ErOne), ("o|", Head::ErZeroOne), ("|o", Head::ErZeroOne), ("|{", Head::ErMany), ("}|", Head::ErMany), ("o{", Head::ErZeroMany), ("}o", Head::ErZeroMany)] {
        if let Some(rest) = s.strip_prefix(pat) { return Some((h, rest.trim().to_string())); }
    }
    None
}
