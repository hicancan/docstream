//! Text as it streams in, read again each time into the whole tree, each top-level block keyed by what it holds.
use crate::read::{read_ranged, anchor_blocks};
use crate::tree::Block;
use serde::Serialize;

/// One top-level block: `key` changes only when the block does; `settled` once a blank line has come after it
/// and something has begun after that, when nothing more can join it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Keyed { pub key: String, pub settled: bool, pub block: Block }

/// The whole document as it now stands.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update { pub blocks: Vec<Keyed>, pub done: bool }

#[derive(Debug, Default)]
pub struct Stream { text: String, done: bool }

impl Stream {
    pub fn new() -> Self { Self::default() }
    /// Text that has arrived, added to what came before.
    pub fn push(&mut self, more: &str) -> Update { self.text.push_str(more); self.update() }
    /// Nothing more will come: the last block is read as written, and settles.
    pub fn finish(&mut self) -> Update { self.done = true; self.update() }
    pub fn text(&self) -> &str { &self.text }

    fn update(&self) -> Update {
        let ranged = read_ranged(&self.text);
        let mut blocks: Vec<(Block, bool)> = Vec::with_capacity(ranged.len());
        let Some(((_, last), settled)) = ranged.split_last() else { return Update { blocks: Vec::new(), done: self.done } };
        // A block is settled once a blank line has come after it: until then the next line may still join it
        // (a table's next row, a list's next item, a quote's lazy line).
        for (block, range) in settled {
            // From the block's last character on, whatever its width.
            let after = &self.text[self.text[..range.end].char_indices().next_back().map_or(0, |(at, _)| at)..];
            blocks.push((block.clone(), self.done || after.contains("\n\n")));
        }
        if self.done {
            for (block, _) in read_ranged(&self.text[last.start..]) { blocks.push((block, true)); }
        } else {
            // The block being written, its open marks closed for now so that it does not flicker.
            for (block, _) in read_ranged(&close_open_marks(&self.text[last.start..])) { blocks.push((block, false)); }
        }
        anchor_blocks(blocks.iter_mut().map(|(block, _)| block));
        Update { blocks: blocks.into_iter().map(|(block, settled)| keyed(block, settled)).collect(), done: self.done }
    }
}

fn keyed(block: Block, settled: bool) -> Keyed {
    let key = format!("{:016x}", fnv(serde_json::to_string(&block).unwrap_or_default().as_bytes()));
    Keyed { key, settled, block }
}

/// A stable hash, the same on every build: FNV-1a, 64 bits.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// The marks a block being written has opened and not yet closed (**, ~~, ==, `, $, $$, \\( and \\[), closed at its
/// end, the last opened first. Inside code, nothing is a mark; an open code fence is left to the reader, which reads it
/// to the end.
pub(crate) fn close_open_marks(text: &str) -> String {
    if text.trim_start().starts_with("```") || text.trim_start().starts_with("~~~") { return text.to_string(); }
    let mut open: Vec<&'static str> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &text[i..];
        let in_code = open.last() == Some(&"`");
        let in_math = matches!(open.last(), Some(&"$") | Some(&"$$") | Some(&"\\(") | Some(&"\\["));
        // An escaped backslash is no mark, nor the start of one.
        if !in_code && rest.starts_with("\\\\") { i += 2; continue; }
        let mark = if rest.starts_with('`') { Some("`") }
            else if in_code { None }
            else if rest.starts_with("\\(") || rest.starts_with("\\)") || rest.starts_with("\\[") || rest.starts_with("\\]") { Some(&rest[..2]) }
            else if rest.starts_with("$$") { Some("$$") }
            else if rest.starts_with('$') { Some("$") }
            else if in_math { None }
            else if rest.starts_with("**") { Some("**") }
            else if rest.starts_with("~~") { Some("~~") }
            else if rest.starts_with("==") { Some("==") }
            else { None };
        match mark {
            Some(mark) => {
                let closes = |opened: &str| if mark.starts_with('\\') { closer(opened) == mark } else { opened == mark };
                if open.last().is_some_and(|&opened| closes(opened)) { open.pop(); }
                else if mark == "\\)" || mark == "\\]" {}
                else if !(in_code || in_math) || mark == "`" && in_code { open.push(static_mark(mark)); }
                i += mark.len();
            }
            None => i += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    let mut closed = text.trim_end_matches([' ', '\t']).to_string();
    for mark in open.iter().rev() { closed.push_str(closer(mark)); }
    closed
}

/// What closes a mark: itself, but `\\)` for `\\(` and `\\]` for `\\[`; and for a closer, its opener.
fn closer(mark: &str) -> &'static str {
    match mark { "\\(" => "\\)", "\\[" => "\\]", "\\)" => "\\(", "\\]" => "\\[", other => static_mark(other) }
}

fn static_mark(mark: &str) -> &'static str {
    match mark { "`" => "`", "$$" => "$$", "$" => "$", "**" => "**", "~~" => "~~", "==" => "==", "\\(" => "\\(", "\\[" => "\\[", "\\)" => "\\)", _ => "\\]" }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_marks_close_in_reverse() {
        assert_eq!(close_open_marks("a **b ==c"), "a **b ==c==**");
        assert_eq!(close_open_marks("`co**de"), "`co**de`");
        assert_eq!(close_open_marks("$x^2 **"), "$x^2 **$");
        assert_eq!(close_open_marks("**done** and ==hi=="), "**done** and ==hi==");
        assert_eq!(close_open_marks("```rust\nfn"), "```rust\nfn");
        assert_eq!(close_open_marks("答案是 \\(x^2 **"), "答案是 \\(x^2 **\\)");
        assert_eq!(close_open_marks("\\(a\\) 与 \\[b"), "\\(a\\) 与 \\[b\\]");
        assert_eq!(close_open_marks("\\\\(不是公式"), "\\\\(不是公式");
    }
    #[test]
    fn the_block_being_written_shows_its_open_strong_already_strong() {
        let mut stream = Stream::new();
        let update = stream.push("先说结论：**三次都避开");
        assert_eq!(update.blocks.len(), 1);
        assert!(!update.blocks[0].settled);
        let json = serde_json::to_string(&update.blocks[0].block).unwrap();
        assert!(json.contains(r#""type":"strong""#), "{json}");
        let first = stream.push("了**。\n\n第二段").blocks[0].clone();
        assert!(first.settled);
        assert_eq!(stream.push("继续").blocks[0], first);
    }
}
