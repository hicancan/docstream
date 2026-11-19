use docstream_core::{read, Block, Inline, Stream};

fn maths(blocks: &[Block]) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for block in blocks {
        match block {
            Block::Math { tex } => out.push((tex.clone(), true)),
            Block::Paragraph { inlines } => for inline in inlines { if let Inline::Math { tex, display } = inline { out.push((tex.clone(), *display)); } },
            Block::Quote { blocks } | Block::Alert { blocks, .. } => out.extend(maths(blocks)),
            _ => {}
        }
    }
    out
}

/// Formulas as models and LaTeX write them read as formulas, whichever way they are marked.
#[test]
fn every_way_of_marking_math_is_math() {
    let text = "行内 \\(x^2\\) 与 $y^2$。\n\n\\[\n\\frac{a}{b}\n\\]\n\n```math\na^2+b^2=c^2\n```\n\n> 引用里 \\(\\xi \\in (a,b)\\)\n";
    assert_eq!(maths(&read(text)), [
        ("x^2".into(), false), ("y^2".into(), false), ("\\frac{a}{b}".into(), true), ("a^2+b^2=c^2".into(), true), ("\\xi \\in (a,b)".into(), false),
    ]);
}

/// What is not math stays as written: code, an escaped backslash, and a bracket that never closes.
#[test]
fn what_is_not_math_stays_as_written() {
    let text = "`\\(code\\)` 和 \\\\(不是\\\\) 和 \\(未闭合\n\n```tex\n\\(x\\)\n```\n";
    let blocks = read(text);
    assert!(maths(&blocks).is_empty(), "{blocks:?}");
    let json = serde_json::to_string(&blocks).unwrap();
    assert!(json.contains(r#""text":"\\(code\\)""#), "{json}");
    assert!(json.contains(r#"\\(x\\)"#), "{json}");
}

/// Streamed a character at a time, a formula written with brackets shows as a formula while it is written, and the
/// end is the same as reading the whole at once.
#[test]
fn brackets_stream_as_they_are_read() {
    let text = "先看 \\(E = mc^2\\)，再看\n\n\\[\n\\int_0^1 x\\,dx\n\\]\n\n结束。\n";
    let mut stream = Stream::new();
    let mut shown_as_math = false;
    for ch in text.chars() {
        let update = stream.push(&ch.to_string());
        let blocks: Vec<Block> = update.blocks.into_iter().map(|keyed| keyed.block).collect();
        if stream.text().ends_with("mc") { shown_as_math = maths(&blocks).iter().any(|(tex, _)| tex.starts_with("E = mc")); }
    }
    assert!(shown_as_math);
    let ended: Vec<Block> = stream.finish().blocks.into_iter().map(|keyed| keyed.block).collect();
    assert_eq!(ended, read(text));
}

/// A block of math is TeX to its closing `$$`: a line of `=` in it is no heading, a `- ` no list, in a quote as well.
#[test]
fn a_block_of_math_is_read_to_its_close() {
    let text = "推导：\n\n$$\nAB=\\begin{pmatrix}a\\\\b\\end{pmatrix}\n=\n- c\n$$\n\n> $$ x\n> = y $$\n\n收尾 $$ 不是块\n";
    let blocks = read(text);
    assert_eq!(maths(&blocks), [
        ("AB=\\begin{pmatrix}a\\\\b\\end{pmatrix}\n=\n- c".into(), true), ("x\n= y".into(), true),
    ]);
    assert!(blocks.iter().all(|b| !matches!(b, Block::Heading { .. } | Block::List { .. })), "{blocks:?}");
    let json = serde_json::to_string(&blocks).unwrap();
    assert!(json.contains("收尾 $$ 不是块"), "{json}");
}

/// A `$$` that never closes, or meets a blank line first, opens nothing.
#[test]
fn an_unclosed_block_of_math_opens_nothing() {
    let blocks = read("$$\na\n\n正文还在。\n");
    assert!(maths(&blocks).is_empty(), "{blocks:?}");
}
