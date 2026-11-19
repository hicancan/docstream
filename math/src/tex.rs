//! TeX math, read into the formula it writes: the commands people and models use, as far as KaTeX covers them.
//! What is not understood is kept as the text it was written as, never dropped.

/// How an atom spaces from its neighbours, as TeX classes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class { Ord, Op, Bin, Rel, Open, Close, Punct, Inner }

/// Which face a letter is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face { Italic, Upright, Bold, BoldItalic, DoubleStruck, Script, Fraktur, SansSerif, Mono }

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A symbol: a letter, digit, operator or relation.
    Symbol { text: String, class: Class, face: Face },
    /// Words: \text{...}, an operator name like sin; drawn upright, CJK by the client's own font.
    Text { text: String, class: Class },
    Group(Vec<Node>),
    Scripts { base: Box<Node>, sup: Option<Box<Node>>, sub: Option<Box<Node>> },
    Frac { num: Box<Node>, den: Box<Node>, rule: bool, style: Option<StyleChange> },
    Sqrt { body: Box<Node>, index: Option<Box<Node>> },
    /// \left( ... \right), its delimiters grown to what they hold.
    Fenced { open: Option<char>, close: Option<char>, body: Vec<Node> },
    /// A delimiter made larger by \big and its kin, to a height in ems.
    Sized { delimiter: char, class: Class, size: f32 },
    /// A large operator (\sum, \int): limits above and below in display, beside it otherwise.
    LargeOp { symbol: char, limits: bool },
    /// An operator written as a word with limits (\lim, \max).
    NamedOp { name: String, limits: bool },
    Accent { base: Box<Node>, accent: char, stretch: bool },
    Over { base: Box<Node>, under: bool },
    /// Rows of cells: matrices, cases, aligned equations.
    Array { rows: Vec<Vec<Node>>, align: Vec<char>, open: Option<char>, close: Option<char>, gap: f32 },
    /// Something set over and/or under a base, centred (\overset, \underset, \stackrel, \substack's rows).
    Stack { base: Box<Node>, over: Option<Box<Node>>, under: Option<Box<Node>>, class: Class },
    /// A brace grown over or under what it spans; a script after it is set above or below (\overbrace, \underbrace).
    Brace { body: Box<Node>, under: bool },
    /// An arrow grown to hold the words over and under it (\xrightarrow).
    Arrow { symbol: char, over: Option<Box<Node>>, under: Option<Box<Node>> },
    /// What it holds, in a frame.
    Boxed(Box<Node>),
    /// The room what it holds would take, empty.
    Phantom(Box<Node>),
    /// Space in ems.
    Space(f32),
    Style(StyleChange, Box<Node>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleChange { Display, Text, Script, ScriptScript }

pub fn parse(tex: &str) -> Node { let mut p = Parser { chars: tex.chars().collect(), at: 0 }; Node::Group(p.list(None)) }

struct Parser { chars: Vec<char>, at: usize }

const GREEK: &[(&str, char)] = &[
    ("alpha", 'α'), ("beta", 'β'), ("gamma", 'γ'), ("delta", 'δ'), ("epsilon", 'ϵ'), ("varepsilon", 'ε'), ("zeta", 'ζ'), ("eta", 'η'),
    ("theta", 'θ'), ("vartheta", 'ϑ'), ("iota", 'ι'), ("kappa", 'κ'), ("lambda", 'λ'), ("mu", 'μ'), ("nu", 'ν'), ("xi", 'ξ'), ("omicron", 'ο'),
    ("pi", 'π'), ("varpi", 'ϖ'), ("rho", 'ρ'), ("varrho", 'ϱ'), ("sigma", 'σ'), ("varsigma", 'ς'), ("tau", 'τ'), ("upsilon", 'υ'), ("phi", 'ϕ'),
    ("varphi", 'φ'), ("chi", 'χ'), ("psi", 'ψ'), ("omega", 'ω'),
    ("Gamma", 'Γ'), ("Delta", 'Δ'), ("Theta", 'Θ'), ("Lambda", 'Λ'), ("Xi", 'Ξ'), ("Pi", 'Π'), ("Sigma", 'Σ'), ("Upsilon", 'Υ'), ("Phi", 'Φ'), ("Psi", 'Ψ'), ("Omega", 'Ω'),
];
const SYMBOLS: &[(&str, char, Class)] = &[
    ("infty", '∞', Class::Ord), ("partial", '∂', Class::Ord), ("nabla", '∇', Class::Ord), ("emptyset", '∅', Class::Ord), ("varnothing", '∅', Class::Ord),
    ("forall", '∀', Class::Ord), ("exists", '∃', Class::Ord), ("nexists", '∄', Class::Ord), ("neg", '¬', Class::Ord), ("lnot", '¬', Class::Ord), ("angle", '∠', Class::Ord),
    ("triangle", '△', Class::Ord), ("ell", 'ℓ', Class::Ord), ("hbar", 'ℏ', Class::Ord), ("Re", 'ℜ', Class::Ord), ("Im", 'ℑ', Class::Ord), ("aleph", 'ℵ', Class::Ord),
    ("prime", '′', Class::Ord), ("degree", '°', Class::Ord), ("top", '⊤', Class::Ord), ("bot", '⊥', Class::Ord), ("clubsuit", '♣', Class::Ord), ("star", '⋆', Class::Bin),
    ("cdots", '⋯', Class::Inner), ("ldots", '…', Class::Inner), ("dots", '…', Class::Inner), ("vdots", '⋮', Class::Ord), ("ddots", '⋱', Class::Inner),
    ("pm", '±', Class::Bin), ("mp", '∓', Class::Bin), ("times", '×', Class::Bin), ("div", '÷', Class::Bin), ("cdot", '⋅', Class::Bin), ("ast", '∗', Class::Bin),
    ("circ", '∘', Class::Bin), ("bullet", '∙', Class::Bin), ("oplus", '⊕', Class::Bin), ("otimes", '⊗', Class::Bin), ("odot", '⊙', Class::Bin), ("cup", '∪', Class::Bin),
    ("cap", '∩', Class::Bin), ("wedge", '∧', Class::Bin), ("land", '∧', Class::Bin), ("vee", '∨', Class::Bin), ("lor", '∨', Class::Bin), ("setminus", '∖', Class::Bin),
    ("leq", '≤', Class::Rel), ("le", '≤', Class::Rel), ("geq", '≥', Class::Rel), ("ge", '≥', Class::Rel), ("neq", '≠', Class::Rel), ("ne", '≠', Class::Rel),
    ("approx", '≈', Class::Rel), ("equiv", '≡', Class::Rel), ("sim", '∼', Class::Rel), ("simeq", '≃', Class::Rel), ("cong", '≅', Class::Rel), ("propto", '∝', Class::Rel),
    ("ll", '≪', Class::Rel), ("gg", '≫', Class::Rel), ("in", '∈', Class::Rel), ("notin", '∉', Class::Rel), ("ni", '∋', Class::Rel), ("subset", '⊂', Class::Rel),
    ("supset", '⊃', Class::Rel), ("subseteq", '⊆', Class::Rel), ("supseteq", '⊇', Class::Rel), ("perp", '⊥', Class::Rel), ("parallel", '∥', Class::Rel), ("mid", '∣', Class::Rel),
    ("to", '→', Class::Rel), ("rightarrow", '→', Class::Rel), ("leftarrow", '←', Class::Rel), ("gets", '←', Class::Rel), ("leftrightarrow", '↔', Class::Rel),
    ("Rightarrow", '⇒', Class::Rel), ("Leftarrow", '⇐', Class::Rel), ("Leftrightarrow", '⇔', Class::Rel), ("iff", '⟺', Class::Rel), ("implies", '⟹', Class::Rel),
    ("mapsto", '↦', Class::Rel), ("longrightarrow", '⟶', Class::Rel), ("uparrow", '↑', Class::Rel), ("downarrow", '↓', Class::Rel), ("vdash", '⊢', Class::Rel),
    ("models", '⊨', Class::Rel), ("coloneqq", '≔', Class::Rel), ("doteq", '≐', Class::Rel), ("prec", '≺', Class::Rel), ("succ", '≻', Class::Rel),
    ("colon", ':', Class::Punct), ("lbrace", '{', Class::Open), ("rbrace", '}', Class::Close), ("langle", '⟨', Class::Open), ("rangle", '⟩', Class::Close),
    ("lfloor", '⌊', Class::Open), ("rfloor", '⌋', Class::Close), ("lceil", '⌈', Class::Open), ("rceil", '⌉', Class::Close), ("vert", '|', Class::Ord), ("Vert", '‖', Class::Ord),
    ("Longrightarrow", '⟹', Class::Rel), ("longleftarrow", '⟵', Class::Rel), ("Longleftarrow", '⟸', Class::Rel), ("impliedby", '⟸', Class::Rel),
    ("longleftrightarrow", '⟷', Class::Rel), ("hookrightarrow", '↪', Class::Rel), ("rightleftharpoons", '⇌', Class::Rel), ("nearrow", '↗', Class::Rel), ("searrow", '↘', Class::Rel),
    ("leqslant", '⩽', Class::Rel), ("geqslant", '⩾', Class::Rel), ("lt", '<', Class::Rel), ("gt", '>', Class::Rel), ("nleq", '≰', Class::Rel), ("ngeq", '≱', Class::Rel),
    ("subsetneq", '⊊', Class::Rel), ("supsetneq", '⊋', Class::Rel), ("nsubseteq", '⊈', Class::Rel), ("asymp", '≍', Class::Rel), ("preceq", '⪯', Class::Rel), ("succeq", '⪰', Class::Rel),
    ("ominus", '⊖', Class::Bin), ("oslash", '⊘', Class::Bin), ("dagger", '†', Class::Bin), ("ddagger", '‡', Class::Bin), ("amalg", '⨿', Class::Bin), ("uplus", '⊎', Class::Bin),
    ("sqcup", '⊔', Class::Bin), ("sqcap", '⊓', Class::Bin), ("diamond", '⋄', Class::Bin), ("wr", '≀', Class::Bin),
    ("therefore", '∴', Class::Rel), ("because", '∵', Class::Rel), ("checkmark", '✓', Class::Ord), ("square", '□', Class::Ord), ("blacksquare", '■', Class::Ord),
    ("lozenge", '◊', Class::Ord), ("flat", '♭', Class::Ord), ("sharp", '♯', Class::Ord), ("natural", '♮', Class::Ord), ("complement", '∁', Class::Ord), ("wp", '℘', Class::Ord), ("mho", '℧', Class::Ord),
    ("lvert", '|', Class::Open), ("rvert", '|', Class::Close), ("lVert", '‖', Class::Open), ("rVert", '‖', Class::Close),
    ("preccurlyeq", '≼', Class::Rel), ("succcurlyeq", '≽', Class::Rel), ("bowtie", '⋈', Class::Rel), ("Longleftrightarrow", '⟺', Class::Rel),
    ("lessgtr", '≶', Class::Rel), ("gtrless", '≷', Class::Rel), ("lessdot", '⋖', Class::Rel), ("gtrdot", '⋗', Class::Rel), ("lll", '⋘', Class::Rel), ("ggg", '⋙', Class::Rel),
    ("nmid", '∤', Class::Rel), ("nparallel", '∦', Class::Rel), ("ncong", '≇', Class::Rel), ("triangleright", '▷', Class::Bin), ("triangleleft", '◁', Class::Bin),
    ("bigcirc", '◯', Class::Bin), ("bigtriangledown", '▽', Class::Bin), ("bigtriangleup", '△', Class::Bin), ("circledast", '⊛', Class::Bin), ("circledcirc", '⊚', Class::Bin),
    ("circleddash", '⊝', Class::Bin), ("bigstar", '★', Class::Ord), ("dag", '†', Class::Ord), ("ddag", '‡', Class::Ord), ("surd", '√', Class::Ord), ("yen", '¥', Class::Ord),
    ("AA", 'Å', Class::Ord), ("imath", 'ı', Class::Ord), ("jmath", 'ȷ', Class::Ord), ("lbrack", '[', Class::Open), ("rbrack", ']', Class::Close),
    ("dotsc", '…', Class::Inner), ("dotso", '…', Class::Inner), ("dotsb", '⋯', Class::Inner), ("dotsm", '⋯', Class::Inner), ("dotsi", '⋯', Class::Inner), ("hdots", '…', Class::Inner),
    ("nrightarrow", '↛', Class::Rel), ("nRightarrow", '⇏', Class::Rel), ("nleftarrow", '↚', Class::Rel), ("nLeftarrow", '⇍', Class::Rel), ("nleftrightarrow", '↮', Class::Rel), ("nLeftrightarrow", '⇎', Class::Rel),
    ("gtrsim", '≳', Class::Rel), ("lesssim", '≲', Class::Rel), ("ngtr", '≯', Class::Rel), ("nless", '≮', Class::Rel), ("npreccurlyeq", '⋠', Class::Rel), ("nsucccurlyeq", '⋡', Class::Rel),
    ("rightharpoonup", '⇀', Class::Rel), ("rightharpoondown", '⇁', Class::Rel), ("leftharpoonup", '↼', Class::Rel), ("leftharpoondown", '↽', Class::Rel),
    ("updownarrow", '↕', Class::Rel), ("Updownarrow", '⇕', Class::Rel), ("Uparrow", '⇑', Class::Rel), ("Downarrow", '⇓', Class::Rel),
    ("rightleftarrows", '⇄', Class::Rel), ("leftrightarrows", '⇆', Class::Rel), ("curvearrowright", '↷', Class::Rel), ("curvearrowleft", '↶', Class::Rel),
    ("twoheadrightarrow", '↠', Class::Rel), ("twoheadleftarrow", '↞', Class::Rel), ("swarrow", '↙', Class::Rel), ("nwarrow", '↖', Class::Rel), ("Rsh", '↱', Class::Rel), ("Lsh", '↰', Class::Rel),
    ("triangleq", '≜', Class::Rel), ("approxeq", '≊', Class::Rel), ("fallingdotseq", '≒', Class::Rel), ("risingdotseq", '≓', Class::Rel), ("eqcirc", '≖', Class::Rel),
    ("Vdash", '⊩', Class::Rel), ("dashv", '⊣', Class::Rel), ("sqsubseteq", '⊑', Class::Rel), ("sqsupseteq", '⊒', Class::Rel), ("sqsubset", '⊏', Class::Rel), ("sqsupset", '⊐', Class::Rel),
    ("between", '≬', Class::Rel), ("smile", '⌣', Class::Rel), ("frown", '⌢', Class::Rel),
    ("intercal", '⊤', Class::Bin), ("vartriangleleft", '⊲', Class::Rel), ("vartriangleright", '⊳', Class::Rel), ("vartriangle", '△', Class::Rel),
    ("blacktriangle", '▲', Class::Ord), ("blacktriangledown", '▼', Class::Ord), ("blacktriangleleft", '◀', Class::Ord), ("blacktriangleright", '▶', Class::Ord), ("blacklozenge", '♦', Class::Ord),
    ("boxtimes", '⊠', Class::Bin), ("boxplus", '⊞', Class::Bin), ("boxdot", '⊡', Class::Bin), ("diamondsuit", '♢', Class::Ord), ("heartsuit", '♡', Class::Ord), ("spadesuit", '♠', Class::Ord),
    ("varkappa", 'ϰ', Class::Ord), ("Nu", 'Ν', Class::Ord), ("Chi", 'Χ', Class::Ord), ("textmu", 'µ', Class::Ord), ("pounds", '£', Class::Ord),
    ("textless", '<', Class::Ord), ("textgreater", '>', Class::Ord), ("textregistered", '®', Class::Ord),
    ("backslash", '\\', Class::Ord), ("%", '%', Class::Ord), ("$", '$', Class::Ord), ("#", '#', Class::Ord), ("&", '&', Class::Ord), ("_", '_', Class::Ord),
];
const LARGE: &[(&str, char, bool)] = &[
    ("sum", '∑', true), ("prod", '∏', true), ("coprod", '∐', true), ("bigcup", '⋃', true), ("bigcap", '⋂', true), ("bigoplus", '⨁', true), ("bigotimes", '⨂', true),
    ("bigsqcup", '⨆', true), ("bigvee", '⋁', true), ("bigwedge", '⋀', true), ("int", '∫', false), ("intop", '∫', true), ("iint", '∬', false), ("iiint", '∭', false), ("oint", '∮', false), ("oiint", '∯', false),
];
const NAMES: &[(&str, bool)] = &[
    ("lim", true), ("limsup", true), ("liminf", true), ("varliminf", true), ("varlimsup", true), ("varprojlim", true), ("varinjlim", true),
    ("max", true), ("min", true), ("sup", true), ("inf", true), ("det", true), ("gcd", true), ("Pr", true),
    ("sin", false), ("cos", false), ("tan", false), ("cot", false), ("sec", false), ("csc", false), ("arcsin", false), ("arccos", false), ("arctan", false),
    ("sinh", false), ("cosh", false), ("tanh", false), ("log", false), ("ln", false), ("lg", false), ("exp", false), ("arg", false), ("deg", false), ("dim", false),
    ("ker", false), ("hom", false), ("mod", false), ("argmax", true), ("argmin", true), ("Tr", false), ("tr", false), ("rank", false), ("sgn", false), ("lcm", false),
];
const ACCENTS: &[(&str, char, bool)] = &[
    ("hat", '\u{302}', false), ("widehat", '\u{302}', true), ("check", '\u{30C}', false), ("tilde", '\u{303}', false), ("widetilde", '\u{303}', true),
    ("bar", '\u{304}', false), ("vec", '\u{20D7}', false), ("dot", '\u{307}', false), ("ddot", '\u{308}', false), ("acute", '\u{301}', false), ("grave", '\u{300}', false),
    ("breve", '\u{306}', false), ("mathring", '\u{30A}', false), ("overrightarrow", '→', true), ("overleftarrow", '←', true), ("overleftrightarrow", '↔', true), ("Vec", '\u{20D7}', false),
];

fn class_of(c: char) -> Class {
    match c {
        '+' | '-' | '*' | '−' => Class::Bin,
        '=' | '<' | '>' | ':' => Class::Rel,
        '(' | '[' => Class::Open,
        ')' | ']' => Class::Close,
        ',' | ';' => Class::Punct,
        '!' | '?' => Class::Close,
        _ => Class::Ord,
    }
}

impl Parser {
    fn peek(&self) -> Option<char> { self.chars.get(self.at).copied() }
    fn skip_space(&mut self) { while self.peek().is_some_and(char::is_whitespace) { self.at += 1; } }

    /// Atoms up to the end, a closing brace, `\right`, `\end` or an array's `&` / `\\` (when `stop` asks for them).
    fn list(&mut self, stop: Option<&[&str]>) -> Vec<Node> {
        let mut out = Vec::new();
        loop {
            self.skip_space();
            let Some(c) = self.peek() else { break };
            if c == '}' { break; }
            if let Some(stops) = stop {
                if stops.iter().any(|s| self.ahead(s)) { break; }
            }
            if self.ahead("\\atop") {
                self.at += 5;
                let den = self.list(stop);
                return vec![Node::Frac { num: Box::new(group(out)), den: Box::new(group(den)), rule: false, style: None }];
            }
            if c == '^' || c == '_' {
                self.at += 1;
                let arg = self.argument();
                let base = match out.pop() { Some(Node::Scripts { base, sup, sub }) => Node::Scripts { base, sup, sub }, Some(node) => Node::Scripts { base: Box::new(node), sup: None, sub: None }, None => Node::Scripts { base: Box::new(Node::Group(vec![])), sup: None, sub: None } };
                if let Node::Scripts { base, mut sup, mut sub } = base {
                    if c == '^' { sup = Some(Box::new(arg)); } else { sub = Some(Box::new(arg)); }
                    out.push(Node::Scripts { base, sup, sub });
                }
                continue;
            }
            if c == '\'' { self.at += 1; out.push(Node::Symbol { text: "′".into(), class: Class::Ord, face: Face::Upright }); continue; }
            if let Some(node) = self.atom() { out.push(node); }
        }
        // A prime written after a symbol is a superscript of it.
        let mut merged: Vec<Node> = Vec::new();
        for node in out {
            if let (Node::Symbol { text, .. }, Some(last)) = (&node, merged.last_mut()) {
                if text == "′" { let base = std::mem::replace(last, Node::Group(vec![])); *last = Node::Scripts { base: Box::new(base), sup: Some(Box::new(node.clone())), sub: None }; continue; }
            }
            merged.push(node);
        }
        merged
    }

    fn ahead(&self, s: &str) -> bool { s.chars().enumerate().all(|(i, c)| self.chars.get(self.at + i) == Some(&c)) && !(s.starts_with('\\') && s.len() > 1 && s.chars().nth(1).is_some_and(char::is_alphabetic) && self.chars.get(self.at + s.chars().count()).is_some_and(|c| c.is_ascii_alphabetic())) }

    /// One argument: a braced group or a single atom.
    fn argument(&mut self) -> Node {
        self.skip_space();
        match self.peek() {
            Some('{') => { self.at += 1; let list = self.list(None); if self.peek() == Some('}') { self.at += 1; } group(list) }
            Some(_) => self.atom().unwrap_or(Node::Group(vec![])),
            None => Node::Group(vec![]),
        }
    }
    /// Raw text in braces: \text{...}, an environment's name.
    fn braced_text(&mut self) -> String {
        self.skip_space();
        if self.peek() != Some('{') { return self.peek().map(|c| { self.at += 1; c.to_string() }).unwrap_or_default(); }
        self.at += 1;
        let mut depth = 1; let mut text = String::new();
        while let Some(c) = self.peek() {
            self.at += 1;
            match c { '{' => depth += 1, '}' => { depth -= 1; if depth == 0 { break; } } '\\' => { if let Some(next) = self.peek() { self.at += 1; text.push(next); continue; } } _ => {} }
            text.push(c);
        }
        text
    }
    fn optional(&mut self) -> Option<Node> {
        self.skip_space();
        if self.peek() != Some('[') { return None; }
        self.at += 1;
        let list = self.list(Some(&["]"]));
        if self.peek() == Some(']') { self.at += 1; }
        Some(group(list))
    }
    fn command(&mut self) -> String {
        let mut name = String::new();
        if let Some(c) = self.peek() {
            if !c.is_ascii_alphabetic() { self.at += 1; return c.to_string(); }
        }
        while let Some(c) = self.peek() { if !c.is_ascii_alphabetic() { break; } name.push(c); self.at += 1; }
        name
    }
    fn delimiter(&mut self) -> Option<char> {
        self.skip_space();
        let c = self.peek()?;
        self.at += 1;
        if c == '.' { return None; }
        if c != '\\' { return Some(c); }
        let name = self.command();
        match name.as_str() {
            "{" | "lbrace" => Some('{'), "}" | "rbrace" => Some('}'), "|" | "Vert" => Some('‖'), "vert" => Some('|'),
            other => SYMBOLS.iter().find(|(n, _, _)| *n == other).map(|(_, c, _)| *c),
        }
    }

    fn atom(&mut self) -> Option<Node> {
        let c = self.peek()?;
        if c == '{' { return Some(self.argument()); }
        if c != '\\' {
            self.at += 1;
            if c == '~' { return Some(Node::Space(0.333)); }
            let face = if c.is_alphabetic() && c.is_ascii() || ('α'..='ω').contains(&c) { Face::Italic } else { Face::Upright };
            let text = if c == '-' { '−'.to_string() } else { c.to_string() };
            return Some(Node::Symbol { text, class: class_of(c), face });
        }
        self.at += 1;
        let name = self.command();
        Some(match name.as_str() {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let style = match name.as_str() { "dfrac" | "cfrac" => Some(StyleChange::Display), "tfrac" => Some(StyleChange::Text), _ => None };
                Node::Frac { num: Box::new(self.argument()), den: Box::new(self.argument()), rule: true, style }
            }
            "binom" | "dbinom" | "tbinom" => { let num = self.argument(); let den = self.argument(); Node::Fenced { open: Some('('), close: Some(')'), body: vec![Node::Frac { num: Box::new(num), den: Box::new(den), rule: false, style: None }] } }
            "sqrt" => { let index = self.optional(); Node::Sqrt { body: Box::new(self.argument()), index: index.map(Box::new) } }
            "left" => {
                let open = self.delimiter();
                let body = self.list(Some(&["\\right"]));
                let close = if self.ahead("\\right") { self.at += 6; self.delimiter() } else { None };
                Node::Fenced { open, close, body }
            }
            "right" => Node::Group(vec![]),
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "bigr" | "Bigl" | "Bigr" | "biggl" | "biggr" | "Biggl" | "Biggr" => {
                let size = match name.trim_end_matches(['l', 'r']) { "big" => 1.2, "Big" => 1.8, "bigg" => 2.4, _ => 3.0 };
                let delimiter = self.delimiter().unwrap_or('.');
                let class = if name.ends_with('l') { Class::Open } else if name.ends_with('r') { Class::Close } else { class_of(delimiter) };
                Node::Sized { delimiter, class, size }
            }
            "text" | "textrm" | "textnormal" | "mbox" | "textit" | "textbf" | "texttt" | "textsf" | "textup" | "textsl" | "emph" | "textmd" => Node::Text { text: self.braced_text(), class: Class::Ord },
            "ref" => Node::Text { text: format!("({})", self.braced_text()), class: Class::Ord },
            "multicolumn" => { self.braced_text(); self.braced_text(); self.argument() }
            // A letter or number in a circle, as Unicode has them; anything longer in a frame.
            "textcircled" => {
                let inside = self.braced_text();
                let mut chars = inside.chars();
                let circled = match (chars.next(), chars.next()) {
                    (Some(c @ '1'..='9'), None) => char::from_u32(0x2460 + c as u32 - '1' as u32),
                    (Some('0'), None) => Some('⓪'),
                    (Some(c @ 'A'..='Z'), None) => char::from_u32(0x24B6 + c as u32 - 'A' as u32),
                    (Some(c @ 'a'..='z'), None) => char::from_u32(0x24D0 + c as u32 - 'a' as u32),
                    _ => inside.parse::<u32>().ok().filter(|n| (10..=20).contains(n)).and_then(|n| char::from_u32(0x2469 + n - 10)),
                };
                match circled { Some(c) => Node::Symbol { text: c.to_string(), class: Class::Ord, face: Face::Upright }, None => Node::Boxed(Box::new(Node::Text { text: inside, class: Class::Ord })) }
            }
            // An equation's number, set apart at its end: (n), or as written with \tag*.
            "tag" => {
                let starred = self.peek() == Some('*');
                if starred { self.at += 1; }
                let number = self.braced_text();
                Node::Group(vec![Node::Space(2.0), Node::Text { text: if starred { number } else { format!("({number})") }, class: Class::Ord }])
            }
            // What is written made an atom of another class: \mathrel{\mathop:} is a relation.
            "mathop" | "mathrel" | "mathbin" | "mathord" | "mathopen" | "mathclose" | "mathpunct" | "mathinner" => {
                let class = match name.as_str() { "mathop" => Class::Op, "mathrel" => Class::Rel, "mathbin" => Class::Bin, "mathopen" => Class::Open, "mathclose" => Class::Close, "mathpunct" => Class::Punct, "mathinner" => Class::Inner, _ => Class::Ord };
                Node::Stack { base: Box::new(self.argument()), over: None, under: None, class }
            }
            "framebox" => { self.optional(); self.optional(); Node::Boxed(Box::new(self.argument())) }
            "xlongequal" => { let under = self.optional(); let over = self.argument(); Node::Arrow { symbol: '=', over: Some(Box::new(over)), under: under.map(Box::new) } }
            // The sizes of words, which a formula here keeps at the size of the text around it.
            "tiny" | "scriptsize" | "footnotesize" | "small" | "normalsize" | "large" | "Large" | "LARGE" | "huge" | "Huge" => Node::Group(vec![]),
            // The font switches of plain TeX: what follows them in the group, in that face.
            "bf" | "rm" | "it" | "mit" | "sf" | "tt" | "cal" | "em" | "sc" | "scshape" | "normalfont" => {
                let face = match name.as_str() { "bf" => Face::Bold, "it" | "mit" | "em" => Face::Italic, "sf" => Face::SansSerif, "tt" => Face::Mono, "cal" => Face::Script, _ => Face::Upright };
                restyle(group(self.list(None)), face)
            }
            "operatorname" => Node::Text { text: self.braced_text(), class: Class::Op },
            "mathrm" | "mathit" | "mathnormal" | "mathbf" | "mathbb" | "mathcal" | "mathscr" | "mathfrak" | "mathsf" | "mathtt" | "boldsymbol" | "bm" | "pmb" | "mathbfit" | "mathbfcal" | "mathsfit" => {
                let face = match name.as_str() { "mathrm" => Face::Upright, "mathit" | "mathnormal" => Face::Italic, "mathbf" => Face::Bold, "boldsymbol" | "bm" | "pmb" | "mathbfit" => Face::BoldItalic, "mathbb" => Face::DoubleStruck, "mathcal" | "mathscr" | "mathbfcal" => Face::Script, "mathfrak" => Face::Fraktur, "mathsf" | "mathsfit" => Face::SansSerif, _ => Face::Mono };
                restyle(self.argument(), face)
            }
            "overset" | "stackrel" => { let over = self.argument(); let base = self.argument(); Node::Stack { base: Box::new(base), over: Some(Box::new(over)), under: None, class: Class::Rel } }
            "underset" => { let under = self.argument(); let base = self.argument(); Node::Stack { base: Box::new(base), over: None, under: Some(Box::new(under)), class: Class::Rel } }
            "overbrace" | "underbrace" => Node::Brace { body: Box::new(self.argument()), under: name == "underbrace" },
            "xrightarrow" | "xleftarrow" | "xRightarrow" | "xLeftarrow" | "xleftrightarrow" | "xmapsto" => {
                let under = self.optional();
                let over = self.argument();
                let symbol = match name.as_str() { "xrightarrow" => '→', "xleftarrow" => '←', "xRightarrow" => '⇒', "xLeftarrow" => '⇐', "xleftrightarrow" => '↔', _ => '↦' };
                Node::Arrow { symbol, over: Some(Box::new(over)), under: under.map(Box::new) }
            }
            "substack" => {
                self.skip_space();
                if self.peek() == Some('{') { self.at += 1; }
                let mut rows = Vec::new();
                loop {
                    rows.push(vec![group(self.list(Some(&["\\\\"])))]);
                    if self.ahead("\\\\") { self.at += 2; } else { if self.peek() == Some('}') { self.at += 1; } break; }
                }
                Node::Array { rows, align: vec!['c'], open: None, close: None, gap: 0.0 }
            }
            "boxed" | "fbox" => Node::Boxed(Box::new(self.argument())),
            "phantom" | "hphantom" | "vphantom" => Node::Phantom(Box::new(self.argument())),
            "color" => { self.braced_text(); Node::Group(vec![]) }
            "textcolor" | "colorbox" => { self.braced_text(); self.argument() }
            "cancel" | "bcancel" | "xcancel" | "smash" | "mathstrut" | "displaylimits" => if name == "mathstrut" || name == "displaylimits" { Node::Group(vec![]) } else { self.argument() },
            "bmod" => Node::Text { text: "mod".into(), class: Class::Bin },
            "pmod" | "pod" => {
                let inner = self.argument();
                let mut list = vec![Node::Space(if name == "pmod" { 1.0 } else { 0.444 }), Node::Symbol { text: "(".into(), class: Class::Open, face: Face::Upright }];
                if name == "pmod" { list.push(Node::Text { text: "mod".into(), class: Class::Ord }); list.push(Node::Space(0.333)); }
                list.push(inner);
                list.push(Node::Symbol { text: ")".into(), class: Class::Close, face: Face::Upright });
                Node::Group(list)
            }
            "not" => {
                // A negated relation, as Unicode writes it; anything else struck by a slash.
                let next = self.atom().unwrap_or(Node::Group(vec![]));
                let negated = match &next { Node::Symbol { text, .. } => match text.as_str() { "=" => Some('≠'), "∈" => Some('∉'), "⊂" => Some('⊄'), "⊃" => Some('⊅'), "⊆" => Some('⊈'), "⊇" => Some('⊉'), "≡" => Some('≢'), "<" => Some('≮'), ">" => Some('≯'), "≤" => Some('≰'), "≥" => Some('≱'), "∼" => Some('≁'), "≈" => Some('≉'), "∣" => Some('∤'), "∥" => Some('∦'), "∃" => Some('∄'), _ => None }, _ => None };
                match negated { Some(c) => Node::Symbol { text: c.to_string(), class: Class::Rel, face: Face::Upright }, None => Node::Group(vec![Node::Symbol { text: "/".into(), class: Class::Ord, face: Face::Upright }, Node::Space(-0.5), next]) }
            }
            "middle" => { let d = self.delimiter().unwrap_or('|'); Node::Symbol { text: d.to_string(), class: Class::Rel, face: Face::Upright } }
            "hline" | "hdashline" | "displaystyle_" => Node::Group(vec![]),
            "overline" => Node::Over { base: Box::new(self.argument()), under: false },
            "underline" => Node::Over { base: Box::new(self.argument()), under: true },
            "displaystyle" => Node::Style(StyleChange::Display, Box::new(group(self.list(None)))),
            "textstyle" => Node::Style(StyleChange::Text, Box::new(group(self.list(None)))),
            "scriptstyle" => Node::Style(StyleChange::Script, Box::new(group(self.list(None)))),
            "scriptscriptstyle" => Node::Style(StyleChange::ScriptScript, Box::new(group(self.list(None)))),
            "," | "thinspace" => Node::Space(3.0 / 18.0), ":" | ">" | "medspace" => Node::Space(4.0 / 18.0), ";" | "thickspace" => Node::Space(5.0 / 18.0),
            "!" | "negthinspace" => Node::Space(-3.0 / 18.0), "negmedspace" => Node::Space(-4.0 / 18.0), "negthickspace" => Node::Space(-5.0 / 18.0),
            " " => Node::Space(0.25), "enspace" | "enskip" => Node::Space(0.5), "quad" => Node::Space(1.0), "qquad" => Node::Space(2.0),
            "hspace" => { if self.peek() == Some('*') { self.at += 1; } Node::Space(parse_em(&self.braced_text())) }
            "{" => Node::Symbol { text: "{".into(), class: Class::Open, face: Face::Upright }, "}" => Node::Symbol { text: "}".into(), class: Class::Close, face: Face::Upright },
            "|" => Node::Symbol { text: "‖".into(), class: Class::Ord, face: Face::Upright },
            "\\" => Node::Space(0.0),
            "begin" => self.environment(),
            "limits" | "nolimits" | "nonumber" | "notag" | "label" | "bgroup" | "egroup" => { if name == "label" { self.braced_text(); } Node::Group(vec![]) }
            other => {
                if let Some((_, c)) = GREEK.iter().find(|(n, _)| *n == other) { Node::Symbol { text: c.to_string(), class: Class::Ord, face: if c.is_lowercase() { Face::Italic } else { Face::Upright } } }
                // amsmath's slanted capitals: \varDelta is an italic Δ.
                else if let Some((_, c)) = other.strip_prefix("var").and_then(|n| GREEK.iter().find(|(g, c)| *g == n && c.is_uppercase())) { Node::Symbol { text: c.to_string(), class: Class::Ord, face: Face::Italic } }
                else if let Some((_, c, class)) = SYMBOLS.iter().find(|(n, _, _)| *n == other) { Node::Symbol { text: c.to_string(), class: *class, face: Face::Upright } }
                else if let Some((_, c, limits)) = LARGE.iter().find(|(n, _, _)| *n == other) { Node::LargeOp { symbol: *c, limits: *limits } }
                else if let Some((n, limits)) = NAMES.iter().find(|(n, _)| *n == other) { Node::NamedOp { name: (*n).into(), limits: *limits } }
                else if let Some((_, accent, stretch)) = ACCENTS.iter().find(|(n, _, _)| *n == other) { Node::Accent { base: Box::new(self.argument()), accent: *accent, stretch: *stretch } }
                // Not understood: kept as written, so that nothing is lost.
                else { Node::Text { text: format!("\\{other}"), class: Class::Ord } }
            }
        })
    }

    fn environment(&mut self) -> Node {
        let name = self.braced_text();
        let (open, close, align, gap) = match name.as_str() {
            "pmatrix" => (Some('('), Some(')'), 'c', 1.0), "bmatrix" => (Some('['), Some(']'), 'c', 1.0), "Bmatrix" => (Some('{'), Some('}'), 'c', 1.0),
            "vmatrix" => (Some('|'), Some('|'), 'c', 1.0), "Vmatrix" => (Some('‖'), Some('‖'), 'c', 1.0), "cases" => (Some('{'), None, 'l', 1.0),
            "aligned" | "align" | "align*" | "split" | "alignat" | "alignedat" => (None, None, 'r', 0.0), "matrix" | "smallmatrix" => (None, None, 'c', 1.0),
            "gathered" | "gather" | "gather*" | "equation" | "equation*" => (None, None, 'c', 0.0), "rcases" => (None, Some('}'), 'l', 1.0), "dcases" => (Some('{'), None, 'l', 1.0), "array" => { self.braced_text(); (None, None, 'c', 1.0) }
            _ => (None, None, 'c', 1.0),
        };
        let mut rows = vec![vec![]];
        loop {
            let cell = self.list(Some(&["&", "\\\\", "\\end"]));
            rows.last_mut().unwrap().push(group(cell));
            if self.ahead("&") { self.at += 1; }
            else if self.ahead("\\\\") { self.at += 2; rows.push(vec![]); }
            else { if self.ahead("\\end") { self.at += 4; self.braced_text(); } break; }
        }
        if rows.last().is_some_and(|row| row.len() == 1 && matches!(&row[0], Node::Group(list) if list.is_empty())) { rows.pop(); }
        // Aligned equations alternate right and left at their &.
        let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
        let align = (0..columns).map(|i| if align == 'r' { if i % 2 == 0 { 'r' } else { 'l' } } else if matches!(name.as_str(), "cases" | "dcases" | "rcases") && i > 0 { 'l' } else { align }).collect();
        Node::Array { rows, align, open, close, gap }
    }
}

fn group(mut list: Vec<Node>) -> Node { if list.len() == 1 { list.pop().unwrap() } else { Node::Group(list) } }

fn restyle(node: Node, to: Face) -> Node {
    match node {
        // Bold symbols keep the slant they had: an italic letter is bold italic, an upright one bold.
        Node::Symbol { text, class, face } => Node::Symbol { text, class, face: match (face, to) { (Face::Upright, Face::BoldItalic) => Face::Bold, (Face::Italic | Face::Upright, _) => to, _ => face } },
        Node::Group(list) => Node::Group(list.into_iter().map(|n| restyle(n, to)).collect()),
        Node::Scripts { base, sup, sub } => Node::Scripts { base: Box::new(restyle(*base, to)), sup, sub },
        other => other,
    }
}

fn parse_em(text: &str) -> f32 {
    let text = text.trim();
    let split = text.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != '+').unwrap_or(text.len());
    let val: f32 = text[..split].parse().unwrap_or(0.5);
    match text[split..].trim() {
        "pt" => (val / 10.0).max(-2.0),
        "mu" => val / 18.0,
        "cm" => val * 2.8,
        "mm" => val * 0.28,
        _ => val,
    }
}
