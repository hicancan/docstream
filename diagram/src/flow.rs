//! Flowcharts and state diagrams: read into nodes and edges, laid out in layers (Sugiyama's method: cycles broken,
//! ranks by longest path, crossings reduced by barycentres, positions balanced), drawn as paths and words.
use crate::draw::{self, arrowhead, cross, dot, label, lines_size, rect, wrap, Diagram, Fill, Item, Pen};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape { Rect, Round, Stadium, Subroutine, Cylinder, Circle, Diamond, Hexagon, Lean, Asym, Start, End, Space }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Line { Solid, Dotted, Thick, Invisible }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Head { None, Arrow, Circle, Cross }

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dir { Down, Up, Right, Left }

/// A node. `home` is the innermost subgraph it was first written in: a node belongs to one subgraph, as in Mermaid.
/// `named` is whether it was given text of its own; a bare name that is also a subgraph's id names that subgraph.
struct Node { id: String, text: String, shape: Shape, home: Option<usize>, named: bool }

/// Where an edge begins or ends: at a node, or at the box of a subgraph.
#[derive(Debug, Clone, Copy, PartialEq)]
enum End { Node(usize), Cluster(usize) }

struct Edge { from: End, to: End, label: String, line: Line, head: Head, tail: Head }

/// A subgraph: a box around what it holds, inside its `parent`. What it holds is every node whose home is it or
/// a subgraph inside it.
struct Cluster { id: String, title: String, parent: Option<usize> }

#[derive(Default)]
struct Graph { nodes: Vec<Node>, edges: Vec<Edge>, clusters: Vec<Cluster>, index: HashMap<String, usize>, dir: Option<Dir>, open: Vec<usize> }

impl Graph {
    fn node(&mut self, id: &str, text: Option<String>, shape: Option<Shape>) -> usize {
        let at = match self.index.get(id) {
            Some(&at) => at,
            None => {
                self.nodes.push(Node { id: id.to_string(), text: id.to_string(), shape: Shape::Rect, home: None, named: false });
                self.index.insert(id.to_string(), self.nodes.len() - 1);
                self.nodes.len() - 1
            }
        };
        if let Some(text) = text { self.nodes[at].text = text; self.nodes[at].named = true; }
        if let Some(shape) = shape { self.nodes[at].shape = shape; }
        if self.nodes[at].home.is_none() { self.nodes[at].home = self.open.last().copied(); }
        at
    }

    fn edge(&mut self, from: usize, to: usize, label: String, line: Line, head: Head, tail: Head) {
        self.edges.push(Edge { from: End::Node(from), to: End::Node(to), label, line, head, tail });
    }

    fn open_cluster(&mut self, id: String, title: String) {
        self.clusters.push(Cluster { id, title, parent: self.open.last().copied() });
        self.open.push(self.clusters.len() - 1);
    }

    /// Whether subgraph `outer` is `inner` or holds it.
    fn holds(&self, outer: usize, inner: usize) -> bool {
        let mut at = Some(inner);
        while let Some(c) = at { if c == outer { return true; } at = self.clusters[c].parent; }
        false
    }

    /// Reading done: a bare name that is a subgraph's id is that subgraph. Written inside another subgraph, it puts
    /// the subgraph there; at an end of an edge, the edge goes to the subgraph's box. A subgraph holding nothing keeps
    /// a space of its own, so it is drawn as the empty box it was written as.
    fn settle(mut self) -> Graph {
        let ids: HashMap<String, usize> = self.clusters.iter().enumerate().map(|(i, c)| (c.id.clone(), i)).collect();
        let gone: Vec<Option<usize>> = self.nodes.iter().map(|n| if n.named { None } else { ids.get(&n.id).copied() }).collect();
        for (v, &c) in gone.iter().enumerate() {
            let (Some(c), Some(home)) = (c, self.nodes[v].home) else { continue };
            if self.clusters[c].parent.is_none() && !self.holds(c, home) { self.clusters[c].parent = Some(home); }
        }
        let mut new_index = vec![usize::MAX; self.nodes.len()];
        let mut kept = Vec::new();
        for (v, node) in std::mem::take(&mut self.nodes).into_iter().enumerate() {
            if gone[v].is_none() { new_index[v] = kept.len(); kept.push(node); }
        }
        self.nodes = kept;
        let map = |end: End| match end { End::Node(v) => match gone[v] { Some(c) => End::Cluster(c), None => End::Node(new_index[v]) }, other => other };
        for e in &mut self.edges { e.from = map(e.from); e.to = map(e.to); }
        for c in 0..self.clusters.len() {
            let holds_any = self.nodes.iter().any(|n| n.home.is_some_and(|h| self.holds(c, h)));
            let held_by_child = self.clusters.iter().any(|d| d.parent == Some(c));
            if !holds_any && !held_by_child {
                self.nodes.push(Node { id: String::new(), text: String::new(), shape: Shape::Space, home: Some(c), named: true });
            }
        }
        // A subgraph holding only empty subgraphs holds their spaces, so it is never without a node either.
        // Names were for reading; the nodes have been renumbered since.
        self.index.clear();
        self.open.clear();
        self
    }
}

fn direction(word: &str) -> Option<Dir> {
    match word.trim().to_ascii_uppercase().as_str() { "TD" | "TB" => Some(Dir::Down), "BT" => Some(Dir::Up), "LR" => Some(Dir::Right), "RL" => Some(Dir::Left), _ => None }
}

/// A flowchart (`graph` or `flowchart`), its header already taken off.
pub fn flowchart(header: &str, body: &[&str]) -> Option<Diagram> { read_flowchart(header, body).map(|g| layout(&g)) }

fn read_flowchart(header: &str, body: &[&str]) -> Option<Graph> {
    let mut g = Graph { dir: direction(header), ..Graph::default() };
    for raw in body {
        for statement in raw.split(';') {
            let s = statement.trim();
            if s.is_empty() { continue; }
            let word = s.split_whitespace().next().unwrap_or("");
            match word {
                "subgraph" => {
                    // `subgraph id [title]`, `subgraph id["title"]`, or a title alone, which is its id as well.
                    let rest = s["subgraph".len()..].trim();
                    let (id, title) = match (rest.find('['), rest.rfind(']')) {
                        (Some(a), Some(b)) if b > a && a > 0 => (rest[..a].trim().to_string(), rest[a + 1..b].trim().trim_matches('"').to_string()),
                        _ => { let t = rest.trim_matches('"').to_string(); (t.clone(), t) }
                    };
                    g.open_cluster(id, title);
                }
                "end" => { g.open.pop(); }
                "direction" => { if g.open.is_empty() { g.dir = direction(&s["direction".len()..]).or(g.dir); } }
                "classDef" | "class" | "style" | "linkStyle" | "click" => {}
                _ => chain(&mut g, s),
            }
        }
    }
    (!g.nodes.is_empty()).then(|| g.settle())
}

/// A state diagram (`stateDiagram`), its header already taken off: `[*]` is where it starts and ends.
pub fn states(body: &[&str]) -> Option<Diagram> { read_states(body).map(|g| layout(&g)) }

fn read_states(body: &[&str]) -> Option<Graph> {
    let mut g = Graph { dir: Some(Dir::Down), ..Graph::default() };
    let mut scope = 0usize;
    let mut scopes = vec![0usize];
    for raw in body {
        let s = raw.trim();
        if s.is_empty() || s.starts_with("note") || s.starts_with("classDef") || s.starts_with("class ") { continue; }
        if let Some(rest) = s.strip_prefix("direction") { g.dir = direction(rest).or(g.dir); continue; }
        if s == "}" { g.open.pop(); scopes.pop(); continue; }
        if let Some(rest) = s.strip_prefix("state ") {
            let rest = rest.trim();
            if let Some(name) = rest.strip_suffix('{') {
                // A composite state is its box: edges to its name go to the box, what it holds is drawn inside.
                let (title, id) = alias(name.trim());
                g.open_cluster(id, title);
                scope += 1; scopes.push(scope);
                continue;
            }
            let (title, id) = alias(rest);
            let id = id.split("<<").next().unwrap_or("").trim().to_string();
            g.node(&id, Some(title), Some(Shape::Round));
            continue;
        }
        let here = *scopes.last().unwrap_or(&0);
        let point = |g: &mut Graph, name: &str, start: bool| -> usize {
            let name = name.trim();
            if name == "[*]" { let id = format!("[*]{}{here}", if start { "start" } else { "end" }); g.node(&id, Some(String::new()), Some(if start { Shape::Start } else { Shape::End })) }
            else { let at = g.node(name, None, None); if g.nodes[at].shape == Shape::Rect { g.nodes[at].shape = Shape::Round; } at }
        };
        if let Some(arrow) = s.find("-->") {
            let (from, rest) = (&s[..arrow], &s[arrow + 3..]);
            let (to, text) = match rest.find(':') { Some(c) => (&rest[..c], rest[c + 1..].trim().to_string()), None => (rest, String::new()) };
            let (a, b) = (point(&mut g, from, true), point(&mut g, to, false));
            g.edge(a, b, text, Line::Solid, Head::Arrow, Head::None);
        } else if let Some(c) = s.find(':') {
            let at = point(&mut g, &s[..c], true);
            let text = s[c + 1..].trim();
            let node = &mut g.nodes[at];
            node.text = if node.text == node.id { text.to_string() } else { format!("{}\n{}", node.text, text) };
        } else {
            point(&mut g, s, true);
        }
    }
    (!g.nodes.is_empty()).then(|| g.settle())
}

/// `"Long name" as id` or `id`.
fn alias(s: &str) -> (String, String) {
    match s.split_once(" as ") {
        Some((title, id)) => (title.trim().trim_matches('"').to_string(), id.trim().to_string()),
        None => (s.trim_matches('"').to_string(), s.trim_matches('"').to_string()),
    }
}

struct Cursor<'a> { s: &'a [char], at: usize }
impl Cursor<'_> {
    fn peek(&self) -> Option<char> { self.s.get(self.at).copied() }
    fn starts(&self, p: &str) -> bool { p.chars().enumerate().all(|(i, c)| self.s.get(self.at + i) == Some(&c)) }
    fn space(&mut self) { while self.peek().is_some_and(char::is_whitespace) { self.at += 1; } }
    fn rest(&self) -> String { self.s[self.at..].iter().collect() }
}

/// `A[text] --> B & C -- label --- D`: nodes joined by edges, as many as written.
fn chain(g: &mut Graph, s: &str) {
    let chars: Vec<char> = s.chars().collect();
    let mut c = Cursor { s: &chars, at: 0 };
    let Some(mut left) = group(g, &mut c) else { return };
    loop {
        c.space();
        let Some((line, head, tail, mut text)) = edge(&mut c) else { return };
        c.space();
        if c.peek() == Some('|') {
            c.at += 1;
            let start = c.at;
            while c.peek().is_some_and(|ch| ch != '|') { c.at += 1; }
            text = chars[start..c.at].iter().collect::<String>().trim().trim_matches('"').to_string();
            c.at += 1;
        }
        let Some(right) = group(g, &mut c) else { return };
        for &a in &left { for &b in &right { g.edge(a, b, text.clone(), line, head, tail); } }
        left = right;
    }
}

fn group(g: &mut Graph, c: &mut Cursor) -> Option<Vec<usize>> {
    let mut out = vec![node(g, c)?];
    loop {
        c.space();
        if c.peek() == Some('&') { c.at += 1; c.space(); out.push(node(g, c)?); } else { return Some(out); }
    }
}

const SHAPES: &[(&str, &str, Shape)] = &[
    ("([", "])", Shape::Stadium), ("[[", "]]", Shape::Subroutine), ("[(", ")]", Shape::Cylinder), ("((", "))", Shape::Circle), ("{{", "}}", Shape::Hexagon),
    ("[/", "/]", Shape::Lean), ("[\\", "\\]", Shape::Lean), ("[/", "\\]", Shape::Lean), ("[\\", "/]", Shape::Lean),
    ("[", "]", Shape::Rect), ("(", ")", Shape::Round), ("{", "}", Shape::Diamond), (">", "]", Shape::Asym),
];

fn node(g: &mut Graph, c: &mut Cursor) -> Option<usize> {
    c.space();
    let start = c.at;
    while let Some(ch) = c.peek() {
        let joins = ch.is_alphanumeric() || ch == '_' || (!ch.is_ascii() && !ch.is_whitespace())
            || (ch == '-' && c.s.get(c.at + 1).is_some_and(|n| n.is_alphanumeric()) && c.at > start)
            || (ch == '.' && c.s.get(c.at + 1).is_some_and(|n| n.is_alphanumeric()) && c.at > start);
        if !joins { break; }
        c.at += 1;
    }
    if c.at == start { return None; }
    let id: String = c.s[start..c.at].iter().collect();
    let mut text = None;
    let mut shape = None;
    for (open, close, kind) in SHAPES {
        if c.starts(open) {
            let save = c.at;
            c.at += open.chars().count();
            let body_start = c.at;
            let quoted = c.peek() == Some('"');
            if quoted {
                c.at += 1;
                while c.peek().is_some_and(|ch| ch != '"') { c.at += 1; }
                c.at += 1;
            }
            while c.at < c.s.len() && !c.starts(close) { c.at += 1; }
            if c.at >= c.s.len() { c.at = save; continue; }
            let raw: String = c.s[body_start..c.at].iter().collect();
            c.at += close.chars().count();
            text = Some(raw.trim().trim_matches('"').to_string());
            shape = Some(*kind);
            break;
        }
    }
    if c.starts(":::") { c.at += 3; while c.peek().is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-') { c.at += 1; } }
    Some(g.node(&id, text, shape))
}

/// An edge: `-->`, `---`, `-.->`, `==>`, `--o`, `--x`, `<-->`, `~~~`, or one with its label inside (`-- text -->`).
fn edge(c: &mut Cursor) -> Option<(Line, Head, Head, String)> {
    let save = c.at;
    let mut tail = Head::None;
    match c.peek() {
        Some('<') => { tail = Head::Arrow; c.at += 1; }
        Some('o') if c.starts("o--") || c.starts("o==") => { tail = Head::Circle; c.at += 1; }
        Some('x') if c.starts("x--") || c.starts("x==") => { tail = Head::Cross; c.at += 1; }
        _ => {}
    }
    let body = |c: &mut Cursor| -> String { let start = c.at; while c.peek().is_some_and(|ch| matches!(ch, '-' | '=' | '.' | '~')) { c.at += 1; } c.s[start..c.at].iter().collect() };
    let head = |c: &mut Cursor| -> Head {
        match c.peek() {
            Some('>') => { c.at += 1; Head::Arrow }
            Some('o') if c.s.get(c.at + 1).is_none_or(|n| n.is_whitespace()) => { c.at += 1; Head::Circle }
            Some('x') if c.s.get(c.at + 1).is_none_or(|n| n.is_whitespace()) => { c.at += 1; Head::Cross }
            _ => Head::None,
        }
    };
    let style = |b: &str| if b.contains('~') { Line::Invisible } else if b.contains('.') { Line::Dotted } else if b.contains('=') { Line::Thick } else { Line::Solid };
    let first = body(c);
    if first.len() < 2 && !(first == "-" && c.peek() == Some('.')) { c.at = save; return None; }
    let h = head(c);
    if h != Head::None || first.len() >= 3 || first.contains('.') && first.len() >= 3 {
        return Some((style(&first), h, tail, String::new()));
    }
    // `-- text -->`: the label runs to the next stretch of dashes.
    let rest = c.rest();
    let ends = ["-->", "---", "==>", "===", ".->", "-.-", "--o", "--x"];
    let found = ends.iter().filter_map(|e| rest.find(e).map(|i| (i, *e))).min();
    let Some((i, _)) = found else { c.at = save; return None };
    let text = rest[..i].trim().trim_matches('"').to_string();
    c.at += rest[..i].chars().count();
    let second = body(c);
    let h = head(c);
    Some((style(&format!("{first}{second}")), h, tail, text))
}

struct Placed { lines: Vec<String>, w: f32, h: f32 }

fn measure(n: &Node) -> Placed {
    let lines = if matches!(n.shape, Shape::Start | Shape::End | Shape::Space) { Vec::new() } else { wrap(&n.text, 200.0) };
    let (tw, th) = lines_size(&lines);
    let (w, h) = match n.shape {
        Shape::Start | Shape::End => (18.0, 18.0),
        Shape::Space => (40.0, 8.0),
        Shape::Circle => { let d = tw.max(th) + 24.0; (d, d) }
        Shape::Diamond => { let w = tw + 1.6 * th + 24.0; (w, (th + 0.5 * tw + 18.0).min(w)) }
        Shape::Hexagon => (tw + th + 32.0, th + 18.0),
        Shape::Lean | Shape::Asym => (tw + th + 28.0, th + 18.0),
        Shape::Cylinder => (tw + 28.0, th + 30.0),
        Shape::Stadium => (tw + th + 24.0, th + 18.0),
        _ => (tw + 28.0, th + 18.0),
    };
    Placed { lines, w: w.max(if matches!(n.shape, Shape::Start | Shape::End) { 0.0 } else { 48.0 }), h }
}

/// A slot in a layer: a node, a point an edge passes through, the label an edge carries, or one side of a
/// subgraph's box on that layer. `home` is the innermost subgraph the slot is inside.
#[derive(Clone)]
struct Slot { node: Option<usize>, along: f32, across: f32, home: Option<usize>, side: Option<(usize, bool)> }

/// Room between a box and what it holds.
const PAD: f32 = 14.0;

fn layout(g: &Graph) -> Diagram { laid(g).diagram }

/// A diagram laid out, with where each node and each subgraph's box landed, for the tests that hold its shape.
struct Laid { diagram: Diagram, #[cfg_attr(not(test), allow(dead_code))] nodes: Vec<[f32; 4]>, #[cfg_attr(not(test), allow(dead_code))] boxes: Vec<[f32; 4]> }

/// The layers of one ordering: what each subgraph holds stays together between the box's sides on every layer it
/// spans, and subgraphs beside each other keep one order on every layer, so no box takes in what it does not hold.
struct Nest<'a> { slots: &'a [Slot], children: &'a [Vec<usize>], top: &'a [usize], first: &'a [usize], last: &'a [usize], sides: &'a [Vec<(usize, usize)>] }

impl Nest<'_> {
    fn layer(&self, r: usize, free: &[usize], key: &[f32], ckey: &[f32]) -> Vec<usize> {
        let mut out = Vec::with_capacity(free.len());
        self.block(None, r, free, key, ckey, &mut out);
        out
    }

    fn block(&self, c: Option<usize>, r: usize, free: &[usize], key: &[f32], ckey: &[f32], out: &mut Vec<usize>) {
        let mut items: Vec<(f32, usize, bool)> = free.iter().filter(|&&s| self.slots[s].home == c).map(|&s| (key[s], s, false)).collect();
        let kids = match c { Some(c) => &self.children[c][..], None => self.top };
        items.extend(kids.iter().filter(|&&d| self.first[d] <= r && r <= self.last[d]).map(|&d| (ckey[d], d, true)));
        items.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)).then(a.1.cmp(&b.1)));
        for (_, i, cluster) in items {
            if cluster {
                let (left, right) = self.sides[i][r - self.first[i]];
                out.push(left);
                self.block(Some(i), r, free, key, ckey, out);
                out.push(right);
            } else { out.push(i); }
        }
    }
}

fn laid(g: &Graph) -> Laid {
    let dir = g.dir.unwrap_or(Dir::Down);
    let vertical = matches!(dir, Dir::Down | Dir::Up);
    let placed: Vec<Placed> = g.nodes.iter().map(measure).collect();
    let n = g.nodes.len();
    let nc = g.clusters.len();
    // What each subgraph holds, and which subgraphs sit directly inside each.
    let held: Vec<Vec<usize>> = (0..nc).map(|c| (0..n).filter(|&v| g.nodes[v].home.is_some_and(|h| g.holds(c, h))).collect()).collect();
    let children: Vec<Vec<usize>> = (0..nc).map(|c| (0..nc).filter(|&d| g.clusters[d].parent == Some(c)).collect()).collect();
    let top: Vec<usize> = (0..nc).filter(|&c| g.clusters[c].parent.is_none()).collect();
    let depth: Vec<usize> = (0..nc).map(|c| { let mut d = 0; let mut at = g.clusters[c].parent; while let Some(p) = at { d += 1; at = g.clusters[p].parent; } d }).collect();
    let at_end = |end: End| -> Vec<usize> { match end { End::Node(v) => vec![v], End::Cluster(c) => held[c].clone() } };
    // The subgraph an edge's end is in, seen from outside it: a box is entered from the subgraph around it.
    let container = |end: End| match end { End::Node(v) => g.nodes[v].home, End::Cluster(c) => g.clusters[c].parent };
    let common = |a: Option<usize>, b: Option<usize>| -> Option<usize> {
        let mut at = a;
        while let Some(c) = at { if b.is_some_and(|b| g.holds(c, b)) { return Some(c); } at = g.clusters[c].parent; }
        None
    };
    // Edges laid out: not a loop, and not between a box and what it holds.
    let sound = |e: &Edge| match (e.from, e.to) {
        (End::Node(a), End::Node(b)) => a != b,
        (End::Node(v), End::Cluster(c)) | (End::Cluster(c), End::Node(v)) => !g.nodes[v].home.is_some_and(|h| g.holds(c, h)),
        (End::Cluster(a), End::Cluster(b)) => !g.holds(a, b) && !g.holds(b, a),
    };
    let real: Vec<usize> = (0..g.edges.len()).filter(|&e| sound(&g.edges[e])).collect();
    // An edge to a box holds everything in it below what the edge leaves; between two boxes, with room for both ends.
    let gap_of = |e: usize| if matches!((g.edges[e].from, g.edges[e].to), (End::Cluster(_), End::Cluster(_))) { 4 } else { 2 };
    let arcs: Vec<(usize, usize, usize)> = real.iter().flat_map(|&e| {
        let (sources, targets) = (at_end(g.edges[e].from), at_end(g.edges[e].to));
        sources.iter().flat_map(|&a| targets.iter().map(move |&b| (e, a, b))).collect::<Vec<_>>()
    }).collect();
    // Cycles broken by turning back the edges that close them.
    let mut reversed = vec![false; g.edges.len()];
    {
        let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
        for &(e, a, b) in &arcs { out[a].push((e, b)); }
        fn visit(v: usize, out: &[Vec<(usize, usize)>], state: &mut [u8], reversed: &mut [bool]) {
            state[v] = 1;
            for &(e, w) in &out[v] {
                if state[w] == 1 { reversed[e] = true; } else if state[w] == 0 { visit(w, out, state, reversed); }
            }
            state[v] = 2;
        }
        let mut state = vec![0u8; n];
        for v in 0..n { if state[v] == 0 { visit(v, &out, &mut state, &mut reversed); } }
    }
    let arc = |&(e, a, b): &(usize, usize, usize)| if reversed[e] { (b, a, gap_of(e)) } else { (a, b, gap_of(e)) };
    // Ranks by longest path, two apart, so an edge's label can sit in the rank between.
    let mut rank = vec![0usize; n];
    for _ in 0..n.max(1) {
        let mut changed = false;
        for x in &arcs { let (a, b, gap) = arc(x); if rank[b] < rank[a] + gap { rank[b] = rank[a] + gap; changed = true; } }
        if !changed { break; }
    }
    // A node nothing leads into sits just above what it leads to.
    for v in 0..n {
        if arcs.iter().any(|x| arc(x).1 == v) { continue; }
        if let Some(min) = arcs.iter().map(arc).filter(|&(a, _, _)| a == v).map(|(_, b, gap)| rank[b].saturating_sub(gap)).min() { rank[v] = min.max(rank[v]); }
    }
    let first: Vec<usize> = (0..nc).map(|c| held[c].iter().map(|&v| rank[v]).min().unwrap_or(0)).collect();
    let last: Vec<usize> = (0..nc).map(|c| held[c].iter().map(|&v| rank[v]).max().unwrap_or(0)).collect();
    let mut slots: Vec<Slot> = (0..n).map(|v| {
        let (w, h) = (placed[v].w, placed[v].h);
        // A node with a loop back into itself keeps room beside it for the loop.
        let looped = g.edges.iter().any(|e| e.from == End::Node(v) && e.to == End::Node(v));
        let (w, h) = if looped { if vertical { (w + 80.0, h) } else { (w, h + 80.0) } } else { (w, h) };
        Slot { node: Some(v), along: if vertical { h } else { w }, across: if vertical { w } else { h }, home: g.nodes[v].home, side: None }
    }).collect();
    let mut layer_of: Vec<usize> = rank.clone();
    // Each edge as a chain of slots, one in every rank it crosses; an end at a box is a point just outside it.
    let mut chains: Vec<Vec<usize>> = vec![Vec::new(); g.edges.len()];
    let mut boxed: Vec<(Option<usize>, Option<usize>)> = vec![(None, None); g.edges.len()];
    let mut label_slot: Vec<Option<usize>> = vec![None; g.edges.len()];
    let mut label_lines: Vec<Vec<String>> = vec![Vec::new(); g.edges.len()];
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for &e in &real {
        let (from, to) = if reversed[e] { (g.edges[e].to, g.edges[e].from) } else { (g.edges[e].from, g.edges[e].to) };
        let home = common(container(from), container(to));
        let point_at = |r: usize, slots: &mut Vec<Slot>, layer_of: &mut Vec<usize>| -> usize {
            slots.push(Slot { node: None, along: 0.0, across: 0.0, home, side: None });
            layer_of.push(r);
            slots.len() - 1
        };
        let a = match from { End::Node(v) => v, End::Cluster(c) => point_at(last[c] + 1, &mut slots, &mut layer_of) };
        let b = match to { End::Node(v) => v, End::Cluster(c) => point_at(first[c].saturating_sub(1), &mut slots, &mut layer_of) };
        // A cycle the ranks could not settle leaves its edge out rather than drawing it backwards.
        if layer_of[b] <= layer_of[a] { continue; }
        // The point outside a box is drawn towards what the box holds nearest it.
        if let End::Cluster(c) = from { pairs.extend(held[c].iter().filter(|&&m| rank[m] == last[c]).map(|&m| (m, a))); }
        if let End::Cluster(c) = to { pairs.extend(held[c].iter().filter(|&&m| rank[m] == first[c]).map(|&m| (b, m))); }
        boxed[e] = (match from { End::Cluster(c) => Some(c), _ => None }, match to { End::Cluster(c) => Some(c), _ => None });
        let mid = (layer_of[a] + layer_of[b]) / 2;
        let mut chain = vec![a];
        for r in layer_of[a] + 1..layer_of[b] {
            let (along, across) = if r == mid && !g.edges[e].label.is_empty() {
                let lines = wrap(&g.edges[e].label, 160.0);
                let (w, h) = lines_size(&lines);
                label_lines[e] = lines;
                label_slot[e] = Some(slots.len());
                if vertical { (h + 4.0, w + 8.0) } else { (w + 12.0, h + 2.0) }
            } else { (0.0, 0.0) };
            slots.push(Slot { node: None, along, across, home, side: None });
            layer_of.push(r);
            chain.push(slots.len() - 1);
        }
        chain.push(b);
        pairs.extend(chain.windows(2).map(|w| (w[0], w[1])));
        chains[e] = chain;
    }
    // Each box has a left and a right side on every layer it spans, tied layer to layer so they stand in line.
    let mut sides: Vec<Vec<(usize, usize)>> = vec![Vec::new(); nc];
    for c in 0..nc {
        for r in first[c]..=last[c] {
            for left in [true, false] {
                slots.push(Slot { node: None, along: 0.0, across: 0.0, home: Some(c), side: Some((c, left)) });
                layer_of.push(r);
            }
            sides[c].push((slots.len() - 2, slots.len() - 1));
        }
        pairs.extend(sides[c].windows(2).flat_map(|w| [(w[0].0, w[1].0), (w[0].1, w[1].1)]));
    }
    let mut up: Vec<Vec<usize>> = vec![Vec::new(); slots.len()];
    let mut down: Vec<Vec<usize>> = vec![Vec::new(); slots.len()];
    for &(a, b) in &pairs { down[a].push(b); up[b].push(a); }
    let ranks = layer_of.iter().max().copied().unwrap_or(0) + 1;
    let mut free: Vec<Vec<usize>> = vec![Vec::new(); ranks];
    for (s, &r) in layer_of.iter().enumerate() { if slots[s].side.is_none() { free[r].push(s); } }
    // The slots inside each box, whose average place orders boxes beside each other.
    let inside: Vec<Vec<usize>> = (0..nc).map(|c| (0..slots.len()).filter(|&s| slots[s].side.is_none() && slots[s].home.is_some_and(|h| g.holds(c, h))).collect()).collect();
    let nest = Nest { slots: &slots, children: &children, top: &top, first: &first, last: &last, sides: &sides };
    let cluster_keys = |pos: &[f32]| -> Vec<f32> { (0..nc).map(|c| if inside[c].is_empty() { 0.0 } else { inside[c].iter().map(|&s| pos[s]).sum::<f32>() / inside[c].len() as f32 }).collect() };
    // First order: as written, nodes before the points edges pass through.
    let written: Vec<f32> = (0..slots.len()).map(|s| s as f32).collect();
    let mut layers: Vec<Vec<usize>> = (0..ranks).map(|r| nest.layer(r, &free[r], &written, &cluster_keys(&written))).collect();
    let crossings = |layers: &[Vec<usize>]| -> usize {
        let mut total = 0;
        let mut pos = vec![0usize; slots.len()];
        for layer in layers { for (i, &s) in layer.iter().enumerate() { pos[s] = i; } }
        for layer in layers {
            let edges: Vec<(usize, usize)> = layer.iter().flat_map(|&s| down[s].iter().map(move |&t| (s, t))).map(|(s, t)| (pos[s], pos[t])).collect();
            for i in 0..edges.len() { for j in i + 1..edges.len() { if (edges[i].0 < edges[j].0 && edges[i].1 > edges[j].1) || (edges[i].0 > edges[j].0 && edges[i].1 < edges[j].1) { total += 1; } } }
        }
        total
    };
    let mut best = layers.clone();
    let mut best_count = crossings(&layers);
    // A slot's place in its layer, as a share of the way across it: the sides of boxes take no share, so places
    // compare between layers that hold different numbers of slots.
    let share = |layer: &[usize], pos: &mut [f32]| {
        let held: Vec<usize> = layer.iter().copied().filter(|&s| slots[s].side.is_none()).collect();
        let k = held.len().max(1) as f32;
        for (i, &s) in held.iter().enumerate() { pos[s] = (i as f32 + 0.5) / k; }
    };
    for round in 0..12 {
        if best_count == 0 { break; }
        let mut pos = vec![0f32; slots.len()];
        for layer in &layers { share(layer, &mut pos); }
        // One order of boxes for the whole sweep, so boxes beside each other keep it on every layer.
        let ckey = cluster_keys(&pos);
        let order: Vec<usize> = if round % 2 == 0 { (0..ranks).collect() } else { (0..ranks).rev().collect() };
        for r in order {
            let near = if round % 2 == 0 { &up } else { &down };
            let key: Vec<f32> = (0..slots.len()).map(|s| {
                let ns = &near[s];
                if ns.is_empty() || layer_of[s] != r { pos[s] } else { ns.iter().map(|&t| pos[t]).sum::<f32>() / ns.len() as f32 }
            }).collect();
            layers[r] = nest.layer(r, &free[r], &key, &ckey);
            share(&layers[r], &mut pos);
        }
        let count = crossings(&layers);
        if count < best_count { best_count = count; best = layers.clone(); }
    }
    let mut layers = best;
    // Then each slot tried at every place its subgraph allows on its layer, before or after each thing beside it,
    // kept where it crosses least: the barycentres cannot see that an edge into a box crosses less from one side.
    let between = |upper: &[usize], lower: &[usize]| -> usize {
        let mut at = vec![usize::MAX; slots.len()];
        for (i, &s) in lower.iter().enumerate() { at[s] = i; }
        let edges: Vec<(usize, usize)> = upper.iter().enumerate().flat_map(|(i, &s)| down[s].iter().filter(|&&t| at[t] != usize::MAX).map(move |&t| (i, t))).map(|(i, t)| (i, at[t])).collect();
        let mut total = 0;
        for i in 0..edges.len() { for j in i + 1..edges.len() { if (edges[i].0 < edges[j].0 && edges[i].1 > edges[j].1) || (edges[i].0 > edges[j].0 && edges[i].1 < edges[j].1) { total += 1; } } }
        total
    };
    let cost = |layers: &[Vec<usize>], r: usize, layer: &[usize]| -> usize {
        (if r > 0 { between(&layers[r - 1], layer) } else { 0 }) + (if r + 1 < ranks { between(layer, &layers[r + 1]) } else { 0 })
    };
    for _ in 0..4 {
        let mut improved = false;
        for r in 0..ranks {
            let movable: Vec<usize> = layers[r].iter().copied().filter(|&s| slots[s].side.is_none()).collect();
            for s in movable {
                let current = layers[r].clone();
                let mut least = cost(&layers, r, &current);
                let mut without = current.clone();
                without.retain(|&t| t != s);
                let find = |t: usize| without.iter().position(|&u| u == t).unwrap();
                let (lo, hi) = match slots[s].home {
                    Some(h) => { let (left, right) = sides[h][r - first[h]]; (find(left) + 1, find(right)) }
                    None => (0, without.len()),
                };
                // The places between the things in the subgraph: a box inside it moves as a whole.
                let mut places = vec![lo];
                let mut j = lo;
                while j < hi {
                    j = match slots[without[j]].side { Some((d, true)) => find(sides[d][r - first[d]].1) + 1, _ => j + 1 };
                    places.push(j);
                }
                let mut chosen = None;
                for p in places {
                    let mut trial = without.clone();
                    trial.insert(p, s);
                    if trial == current { continue; }
                    let c = cost(&layers, r, &trial);
                    if c < least { least = c; chosen = Some(trial); }
                }
                if let Some(trial) = chosen { layers[r] = trial; improved = true; }
            }
        }
        if !improved { break; }
    }
    // Positions across: each slot drawn towards what it joins, never closer to its neighbour than they both allow.
    let title_lines: Vec<Vec<String>> = g.clusters.iter().map(|c| wrap(&c.title, 240.0)).collect();
    let title_size: Vec<(f32, f32)> = title_lines.iter().map(|l| if l.iter().all(|t| t.is_empty()) { (0.0, 0.0) } else { lines_size(l) }).collect();
    // Across, a box beside its contents holds its title too when the title cannot sit above them.
    let pad_in = |c: usize| PAD + if vertical || title_size[c].1 == 0.0 { 0.0 } else { title_size[c].1 + 4.0 };
    let gap = |a: usize, b: usize| match (slots[a].node.is_some() || slots[a].across > 0.0, slots[b].node.is_some() || slots[b].across > 0.0) { (true, true) => 28.0, (false, false) => 10.0, _ => 16.0 };
    let sep = |a: usize, b: usize| -> f32 {
        let space = match (slots[a].side, slots[b].side) {
            (Some((c, true)), Some((d, false))) if c == d => pad_in(c) + PAD,
            (Some((c, true)), _) => pad_in(c),
            (_, Some((_, false))) => PAD,
            (Some((_, false)), Some((_, true))) => 16.0,
            (Some((_, false)), _) | (_, Some((_, true))) => 12.0,
            _ => gap(a, b),
        };
        (slots[a].across + slots[b].across) / 2.0 + space
    };
    let mut x = vec![0f32; slots.len()];
    for layer in &layers { let mut at = 0.0; for (i, &s) in layer.iter().enumerate() { if i > 0 { at += sep(layer[i - 1], s); } x[s] = at; } }
    for round in 0..24 {
        let order: Vec<usize> = if round % 2 == 0 { (0..ranks).collect() } else { (0..ranks).rev().collect() };
        for r in order {
            let layer = &layers[r];
            let desired: Vec<f32> = layer.iter().enumerate().map(|(i, &s)| {
                // A side of a box hugs what is next to it inside the box.
                match slots[s].side {
                    Some((_, true)) => return x[layer[i + 1]] - sep(s, layer[i + 1]),
                    Some((_, false)) => return x[layer[i - 1]] + sep(layer[i - 1], s),
                    None => {}
                }
                let mut sum = 0.0; let mut weight = 0.0;
                for &t in up[s].iter().chain(down[s].iter()) {
                    let w = match (slots[s].node.is_some(), slots[t].node.is_some()) { (false, false) => 8.0, (true, true) => 1.0, _ => 2.0 };
                    sum += x[t] * w; weight += w;
                }
                if weight > 0.0 { sum / weight } else { x[s] }
            }).collect();
            let k = layer.len();
            let mut left = desired.clone();
            for i in 1..k { left[i] = left[i].max(left[i - 1] + sep(layer[i - 1], layer[i])); }
            let mut right = desired.clone();
            for i in (0..k.saturating_sub(1)).rev() { right[i] = right[i].min(right[i + 1] - sep(layer[i], layer[i + 1])); }
            for i in 0..k { x[layer[i]] = (left[i] + right[i]) / 2.0; }
        }
    }
    // Each side of a box in one line on every layer, and the box as wide as its title: moving only rightwards
    // until nothing is closer than it may be, which ends because the boxes keep one order on every layer.
    // Then each box drawn in tight around what it holds on all its layers, the inner before the outer.
    let mut index_in = vec![0usize; slots.len()];
    for layer in &layers { for (i, &s) in layer.iter().enumerate() { index_in[s] = i; } }
    let mut inner_first: Vec<usize> = (0..nc).collect();
    inner_first.sort_by_key(|&c| std::cmp::Reverse(depth[c]));
    for &c in &inner_first {
        let lx = sides[c].iter().map(|&(l, _)| { let layer = &layers[layer_of[l]]; let next = layer[index_in[l] + 1]; x[next] - sep(l, next) }).fold(f32::MAX, f32::min);
        let rx = sides[c].iter().map(|&(_, r)| { let layer = &layers[layer_of[r]]; let prev = layer[index_in[r] - 1]; x[prev] + sep(prev, r) }).fold(f32::MIN, f32::max);
        for &(l, r) in &sides[c] { x[l] = lx; x[r] = rx.max(lx); }
    }
    let min_width: Vec<f32> = (0..nc).map(|c| if vertical { title_size[c].0 + 20.0 } else { 0.0 }).collect();
    for _ in 0..10_000 {
        let mut changed = false;
        for c in 0..nc {
            let lx = sides[c].iter().map(|s| x[s.0]).fold(f32::MIN, f32::max);
            let rx = sides[c].iter().map(|s| x[s.1]).fold(f32::MIN, f32::max).max(lx + min_width[c]);
            for &(l, r) in &sides[c] {
                if x[l] < lx { x[l] = lx; changed = true; }
                if x[r] < rx { x[r] = rx; changed = true; }
            }
        }
        for layer in &layers {
            for i in 1..layer.len() {
                let need = x[layer[i - 1]] + sep(layer[i - 1], layer[i]);
                if x[layer[i]] < need - 0.01 { x[layer[i]] = need; changed = true; }
            }
        }
        if !changed { break; }
    }
    // Room between ranks for the boxes that begin below or end above: padding, and a title on top; boxes that begin
    // or end on one rank, one inside another, each take their own.
    let mut by_depth: Vec<usize> = (0..nc).collect();
    by_depth.sort_by_key(|&c| std::cmp::Reverse(depth[c]));
    let mut over = vec![0f32; nc];
    let mut under = vec![0f32; nc];
    for &c in &by_depth {
        let title = if vertical && title_size[c].1 > 0.0 { title_size[c].1 + 4.0 } else { 0.0 };
        over[c] = PAD + title + children[c].iter().filter(|&&d| first[d] == first[c]).map(|&d| over[d]).fold(0.0, f32::max);
        under[c] = PAD + children[c].iter().filter(|&&d| last[d] == last[c]).map(|&d| under[d]).fold(0.0, f32::max);
    }
    let mut room_over = vec![0f32; ranks];
    let mut room_under = vec![0f32; ranks];
    for c in 0..nc { room_over[first[c]] = room_over[first[c]].max(over[c]); room_under[last[c]] = room_under[last[c]].max(under[c]); }
    let depth_of: Vec<f32> = layers.iter().map(|layer| layer.iter().map(|&s| slots[s].along).fold(0.0, f32::max)).collect();
    let mut rank_top = vec![0f32; ranks];
    let mut total_along = 0.0;
    // Sideways, a box is as long as its title: what it lacks is added below its last rank.
    for _ in 0..=nc {
        let mut at = room_over[0];
        for r in 0..ranks {
            if r > 0 { at += 20.0 + room_under[r - 1] + room_over[r]; }
            rank_top[r] = at;
            at += depth_of[r];
        }
        total_along = at + room_under[ranks - 1];
        let mut more = false;
        if !vertical {
            for c in 0..nc {
                let span = rank_top[last[c]] + depth_of[last[c]] + under[c] - (rank_top[first[c]] - over[c]);
                let need = title_size[c].0 + 20.0;
                if span < need - 0.01 { room_under[last[c]] += need - span; under[c] += need - span; more = true; }
            }
        }
        if !more { break; }
    }
    let y: Vec<f32> = (0..slots.len()).map(|s| rank_top[layer_of[s]] + depth_of[layer_of[s]] / 2.0).collect();
    // Into the page: flipped for upward and leftward, turned for sideways.
    let point = |across: f32, along: f32| -> (f32, f32) {
        let along = if matches!(dir, Dir::Up | Dir::Left) { total_along - along } else { along };
        if vertical { (across, along) } else { (along, across) }
    };
    // Each box, across from its left side to its right, along from above its first rank to below its last.
    let span_of = |c: usize| -> (f32, f32, f32, f32) {
        let (l, r) = sides[c][0];
        (x[l], x[r], rank_top[first[c]] - over[c], rank_top[last[c]] + depth_of[last[c]] + under[c])
    };
    let page = |(a0, a1, b0, b1): (f32, f32, f32, f32)| -> [f32; 4] {
        let (p, q) = (point(a0, b0), point(a1, b1));
        [p.0.min(q.0), p.1.min(q.1), p.0.max(q.0), p.1.max(q.1)]
    };
    let mut items = Vec::new();
    let mut bounds = Bounds::default();
    let boxes: Vec<[f32; 4]> = (0..nc).map(|c| page(span_of(c))).collect();
    // Boxes first, the outer behind the inner, behind what they hold.
    let mut by_nesting: Vec<usize> = (0..nc).collect();
    by_nesting.sort_by_key(|&c| depth[c]);
    for &c in &by_nesting {
        let [x0, y0, x1, y1] = boxes[c];
        items.push(Item::Path { d: rect(x0, y0, x1 - x0, y1 - y0, 8.0), fill: Fill::Soft, stroke: 1.0, dash: true });
        // The title in the corner of the room kept for it, clear of the edges that come in through the middle.
        let (tw, th) = title_size[c];
        let (a0, _, b0, _) = span_of(c);
        let (tx, ty) = if vertical { point(a0 + 10.0 + tw / 2.0, b0 + 8.0 + th / 2.0) } else { point(a0 + 8.0 + th / 2.0, b0 + 10.0 + tw / 2.0) };
        label(&mut items, &title_lines[c], tx, ty, true);
        bounds.add(x0, y0); bounds.add(x1, y1);
    }
    // Edges, each a curve from slot to slot that leaves and arrives square to the ranks, from and to a box's edge
    // when it ends at a box.
    for &e in &real {
        let edge = &g.edges[e];
        let chain = &chains[e];
        if chain.is_empty() || edge.line == Line::Invisible { continue; }
        // Stations along the edge: across, where it arrives, where it leaves, and whether it merely passes.
        let mut st: Vec<(f32, f32, f32, bool)> = Vec::new();
        let at_box = |c: usize, s: usize| { let (a0, a1, _, _) = span_of(c); x[s].clamp(a0 + 8.0, (a1 - 8.0).max(a0 + 8.0)) };
        if let Some(c) = boxed[e].0 { let b1 = span_of(c).3; st.push((at_box(c, chain[0]), b1, b1, false)); }
        for &s in chain { st.push((x[s], y[s] - slots[s].along / 2.0, y[s] + slots[s].along / 2.0, slots[s].node.is_none() && slots[s].along == 0.0)); }
        if let Some(c) = boxed[e].1 { let b0 = span_of(c).2; st.push((at_box(c, chain[chain.len() - 1]), b0, b0, false)); }
        // Through a point an edge merely passes, it runs along the line from the station before to the one after.
        let slope = |i: usize| -> f32 {
            if i == 0 || i + 1 == st.len() || !st[i].3 { return 0.0; }
            (st[i + 1].0 - st[i - 1].0) / ((st[i + 1].1 + st[i + 1].2) / 2.0 - (st[i - 1].1 + st[i - 1].2) / 2.0).max(1.0)
        };
        let mut d = Pen::new();
        let mut last_seg = (0.0, 0.0, 0.0, 0.0);
        for i in 0..st.len() - 1 {
            let (ax, ay) = (st[i].0, st[i].2);
            let (bx, by) = (st[i + 1].0, st[i + 1].1);
            let half = (by - ay) / 2.0;
            let (p0, p1, p2, p3) = (point(ax, ay), point(ax + slope(i) * half, ay + half), point(bx - slope(i + 1) * half, by - half), point(bx, by));
            if i == 0 || st[i].2 > st[i].1 { d = d.m(p0.0, p0.1); }
            d = d.c(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1);
            last_seg = (p2.0, p2.1, p3.0, p3.1);
        }
        let width = match edge.line { Line::Thick => 2.6, _ => 1.4 };
        items.push(Item::Path { d: d.done(), fill: Fill::None, stroke: width, dash: edge.line == Line::Dotted });
        // Heads at either end, as written: an edge turned back for layout still points where it was written to.
        let (head_end, tail_end) = if reversed[e] { (edge.tail, edge.head) } else { (edge.head, edge.tail) };
        let start = (point(st[0].0, st[0].2), point(st[0].0, (st[0].2 + st[1].1) / 2.0));
        ends_mark(&mut items, head_end, last_seg.2, last_seg.3, last_seg.2 - last_seg.0, last_seg.3 - last_seg.1);
        ends_mark(&mut items, tail_end, start.0 .0, start.0 .1, start.0 .0 - start.1 .0, start.0 .1 - start.1 .1);
        if let Some(s) = label_slot[e] { let (lx, ly) = point(x[s], y[s]); label(&mut items, &label_lines[e], lx, ly, false); }
    }
    // Loops back into the node they leave.
    for edge in g.edges.iter() {
        let End::Node(v) = edge.from else { continue };
        if edge.to != End::Node(v) { continue; }
        let (cx, cy) = point(x[v], y[v]);
        let (w, h) = (placed[v].w, placed[v].h);
        let (sx, sy, ex, ey, c1, c2) = if vertical {
            (cx + w / 2.0, cy - h / 4.0, cx + w / 2.0, cy + h / 4.0, (cx + w / 2.0 + 36.0, cy - h / 2.0 - 8.0), (cx + w / 2.0 + 36.0, cy + h / 2.0 + 8.0))
        } else {
            (cx - w / 4.0, cy + h / 2.0, cx + w / 4.0, cy + h / 2.0, (cx - w / 2.0 - 8.0, cy + h / 2.0 + 36.0), (cx + w / 2.0 + 8.0, cy + h / 2.0 + 36.0))
        };
        items.push(Item::Path { d: Pen::new().m(sx, sy).c(c1.0, c1.1, c2.0, c2.1, ex, ey).done(), fill: Fill::None, stroke: 1.4, dash: edge.line == Line::Dotted });
        ends_mark(&mut items, edge.head, ex, ey, ex - c2.0, ey - c2.1);
        let lines = wrap(&edge.label, 160.0);
        let (lw, _) = lines_size(&lines);
        let (lx, ly) = if vertical { (cx + w / 2.0 + 34.0 + lw / 2.0, cy) } else { (cx, cy + h / 2.0 + 40.0) };
        label(&mut items, &lines, lx, ly, false);
        bounds.add(lx + lw / 2.0 + 4.0, ly); bounds.add(c1.0, c1.1); bounds.add(c2.0, c2.1);
    }
    // Nodes over the edges; a space keeps an empty box open and draws nothing.
    let mut nodes = Vec::with_capacity(n);
    for v in 0..n {
        let (cx, cy) = point(x[v], y[v]);
        let p = &placed[v];
        nodes.push([cx - p.w / 2.0, cy - p.h / 2.0, cx + p.w / 2.0, cy + p.h / 2.0]);
        if g.nodes[v].shape == Shape::Space { continue; }
        shape(&mut items, g.nodes[v].shape, cx, cy, p.w, p.h);
        label(&mut items, &p.lines, cx, cy + if g.nodes[v].shape == Shape::Cylinder { 5.0 } else { 0.0 }, false);
        bounds.add(cx - p.w / 2.0, cy - p.h / 2.0); bounds.add(cx + p.w / 2.0, cy + p.h / 2.0);
    }
    for s in label_slot.iter().flatten() { let (lx, ly) = point(x[*s], y[*s]); let (w, h) = if vertical { (slots[*s].across, slots[*s].along) } else { (slots[*s].along, slots[*s].across) }; bounds.add(lx - w / 2.0, ly - h / 2.0); bounds.add(lx + w / 2.0, ly + h / 2.0); }
    Laid { diagram: finish(items, bounds), nodes, boxes }
}

fn ends_mark(items: &mut Vec<Item>, head: Head, x: f32, y: f32, dx: f32, dy: f32) {
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    match head {
        Head::None => {}
        Head::Arrow => items.push(arrowhead(x, y, dx, dy)),
        Head::Circle => items.push(dot(x - dx / len * 4.0, y - dy / len * 4.0, 4.0, Fill::Solid)),
        Head::Cross => items.push(cross(x - dx / len * 6.0, y - dy / len * 6.0)),
    }
}

fn shape(items: &mut Vec<Item>, shape: Shape, cx: f32, cy: f32, w: f32, h: f32) {
    let (x0, y0, x1, y1) = (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
    let path = |d: String| Item::Path { d, fill: Fill::Soft, stroke: 1.4, dash: false };
    match shape {
        Shape::Rect => items.push(path(rect(x0, y0, w, h, 3.0))),
        Shape::Round => items.push(path(rect(x0, y0, w, h, 10.0))),
        Shape::Stadium => items.push(path(rect(x0, y0, w, h, h / 2.0))),
        Shape::Subroutine => {
            items.push(path(rect(x0, y0, w, h, 0.0)));
            items.push(Item::Path { d: Pen::new().m(x0 + 8.0, y0).l(x0 + 8.0, y1).m(x1 - 8.0, y0).l(x1 - 8.0, y1).done(), fill: Fill::None, stroke: 1.4, dash: false });
        }
        Shape::Cylinder => {
            let ry = 7.0;
            items.push(path(Pen::new().m(x0, y0 + ry).a(w / 2.0, ry, true, x1, y0 + ry).l(x1, y1 - ry).a(w / 2.0, ry, true, x0, y1 - ry).z()));
            items.push(Item::Path { d: Pen::new().m(x0, y0 + ry).a(w / 2.0, ry, false, x1, y0 + ry).done(), fill: Fill::None, stroke: 1.4, dash: false });
        }
        Shape::Circle => items.push(dot(cx, cy, w / 2.0, Fill::Soft)),
        Shape::Diamond => items.push(path(Pen::new().m(cx, y0).l(x1, cy).l(cx, y1).l(x0, cy).z())),
        Shape::Hexagon => { let k = h / 2.0; items.push(path(Pen::new().m(x0 + k, y0).l(x1 - k, y0).l(x1, cy).l(x1 - k, y1).l(x0 + k, y1).l(x0, cy).z())); }
        Shape::Lean => { let k = h / 2.0; items.push(path(Pen::new().m(x0 + k, y0).l(x1, y0).l(x1 - k, y1).l(x0, y1).z())); }
        Shape::Asym => { let k = h / 2.0; items.push(path(Pen::new().m(x0, y0).l(x1, y0).l(x1, y1).l(x0, y1).l(x0 + k, cy).z())); }
        Shape::Start => items.push(dot(cx, cy, 8.0, Fill::Solid)),
        Shape::End => { items.push(dot(cx, cy, 9.0, Fill::None)); items.push(dot(cx, cy, 5.5, Fill::Solid)); }
        Shape::Space => {}
    }
}

#[derive(Default)]
pub(crate) struct Bounds { pub x0: f32, pub y0: f32, pub x1: f32, pub y1: f32, any: bool }
impl Bounds {
    pub fn add(&mut self, x: f32, y: f32) {
        if !self.any { *self = Bounds { x0: x, y0: y, x1: x, y1: y, any: true }; return; }
        self.x0 = self.x0.min(x); self.y0 = self.y0.min(y); self.x1 = self.x1.max(x); self.y1 = self.y1.max(y);
    }
}

/// The diagram as drawn, with a small margin around what it holds; its items keep their places, the view moves.
pub(crate) fn finish(items: Vec<Item>, b: Bounds) -> Diagram {
    let margin = 4.0;
    Diagram { x: draw::r(b.x0 - margin), y: draw::r(b.y0 - margin), width: draw::r(b.x1 - b.x0 + 2.0 * margin), height: draw::r(b.y1 - b.y0 + 2.0 * margin), items, title: String::new(), description: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every flowchart and state diagram in the golden cases, and the ones below written to test boxes.
    fn sources() -> Vec<String> {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/cases.md")).unwrap().replace("\r\n", "\n");
        text.split("```mermaid").skip(1).map(|c| c.split("```").next().unwrap().trim().to_string())
            .filter(|c| c.starts_with("flowchart") || c.starts_with("graph") || c.starts_with("stateDiagram")).collect()
    }

    fn read(source: &str) -> Graph {
        let lines: Vec<&str> = source.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let (header, body) = lines.split_first().unwrap();
        if header.starts_with("stateDiagram") { read_states(body).unwrap() } else { read_flowchart(header.split_once(' ').map_or("", |h| h.1), body).unwrap() }
    }

    fn overlap(a: [f32; 4], b: [f32; 4]) -> bool { a[0] < b[2] - 0.5 && b[0] < a[2] - 0.5 && a[1] < b[3] - 0.5 && b[1] < a[3] - 0.5 }
    fn within(a: [f32; 4], b: [f32; 4]) -> bool { a[0] >= b[0] - 0.5 && a[1] >= b[1] - 0.5 && a[2] <= b[2] + 0.5 && a[3] <= b[3] + 0.5 }

    /// A box holds what it holds and nothing else, and boxes meet only when one is inside the other.
    #[test]
    fn every_box_holds_its_own_and_no_other() {
        for source in sources() {
            let g = read(&source);
            let laid = laid(&g);
            for c in 0..g.clusters.len() {
                for v in 0..g.nodes.len() {
                    let mine = g.nodes[v].home.is_some_and(|h| g.holds(c, h));
                    if mine { assert!(within(laid.nodes[v], laid.boxes[c]), "{source}\n{} outside {}", g.nodes[v].id, g.clusters[c].title); }
                    else { assert!(!overlap(laid.nodes[v], laid.boxes[c]), "{source}\n{} inside {}", g.nodes[v].id, g.clusters[c].title); }
                }
                for d in 0..g.clusters.len() {
                    if c == d || g.holds(c, d) || g.holds(d, c) { continue; }
                    assert!(!overlap(laid.boxes[c], laid.boxes[d]), "{source}\n{} meets {}", g.clusters[c].title, g.clusters[d].title);
                }
                if let Some(p) = g.clusters[c].parent { assert!(within(laid.boxes[c], laid.boxes[p]), "{source}\n{} outside {}", g.clusters[c].title, g.clusters[p].title); }
            }
        }
    }

    /// A node belongs to the first subgraph that names it; a bare subgraph id names the subgraph, nesting it or
    /// ending an edge at its box; a composite state is its box.
    #[test]
    fn subgraphs_are_read_as_mermaid_reads_them() {
        let g = read("flowchart TB\nsubgraph a [甲]\nx --> y\nend\nsubgraph b\ny --> z\nend\nsubgraph c\nb\nend\na --> b");
        let home = |id: &str| g.nodes.iter().find(|n| n.id == id).unwrap().home.map(|h| g.clusters[h].id.clone());
        assert_eq!((home("x"), home("y"), home("z")), (Some("a".into()), Some("a".into()), Some("b".into())));
        assert_eq!(g.clusters[0].title, "甲");
        assert_eq!(g.clusters[1].parent, Some(2));
        assert!(g.nodes.iter().all(|n| n.id != "a" && n.id != "b"));
        assert!(g.edges.iter().any(|e| e.from == End::Cluster(0) && e.to == End::Cluster(1)));
        let s = read("stateDiagram-v2\n[*] --> 工作\nstate 工作 {\n[*] --> 写\n写 --> 改\n}\n工作 --> [*]");
        assert_eq!(s.clusters.len(), 1);
        assert!(s.nodes.iter().all(|n| n.id != "工作"));
        assert_eq!(s.edges.iter().filter(|e| matches!(e.to, End::Cluster(0)) || matches!(e.from, End::Cluster(0))).count(), 2);
    }
}
