use docstream_core::{read, Block, Stream};

fn anchors(blocks: &[Block], output: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Heading { anchor, .. } => output.push(anchor.clone()),
            Block::Quote { blocks } | Block::Alert { blocks, .. } => anchors(blocks, output),
            Block::List { items, .. } => for item in items { anchors(&item.blocks, output); },
            _ => {},
        }
    }
}

#[test]
fn heading_addresses_belong_to_the_parsed_document() {
    let text = "# Same\n\n## **Same**\n\n### Same-1\n\n> ### Same\n\n### C++ 的 `sizeof` 与 $x^2$\n\n### ![图示](figure.png)\n";
    let whole = read(text);
    let mut ids = Vec::new();
    anchors(&whole, &mut ids);
    assert_eq!(ids, ["same", "same-1", "same-1-1", "same-2", "c-的-sizeof-与-x2", "图示"]);
    let mut stream = Stream::new();
    for ch in text.chars() { stream.push(&ch.to_string()); }
    let ended = stream.finish().blocks.into_iter().map(|keyed| keyed.block).collect::<Vec<_>>();
    assert_eq!(ended, whole);
}
