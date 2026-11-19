//! How each diagram in cases.md is laid out, kept beside it; `UPDATE_GOLDEN=1 cargo test` writes it anew after a
//! deliberate change, and the diagrams example draws them to look at.
use std::fs;
use std::path::Path;

fn cases() -> Vec<String> {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases.md")).unwrap().replace("\r\n", "\n");
    text.split("```mermaid").skip(1).map(|c| c.split("```").next().unwrap().trim().to_string()).collect()
}

#[test]
fn every_case_lays_out_as_kept() {
    let kept = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases.json");
    let got: Vec<_> = cases().iter().map(|source| docstream_diagram::diagram(source).expect("every case is a kind laid out")).collect();
    let got = serde_json::to_string_pretty(&got).unwrap() + "\n";
    if std::env::var_os("UPDATE_GOLDEN").is_some() { fs::write(&kept, &got).unwrap(); return; }
    assert_eq!(got, fs::read_to_string(&kept).unwrap().replace("\r\n", "\n"));
}

/// A model writes a diagram a character at a time; every unfinished piece of it lays out to something finite, or to
/// nothing while not even its kind is written.
#[test]
fn every_prefix_of_every_case_lays_out() {
    for source in cases() {
        for end in source.char_indices().map(|(i, _)| i).chain([source.len()]) {
            if let Some(d) = docstream_diagram::diagram(&source[..end]) {
                assert!(d.width.is_finite() && d.height.is_finite() && d.width >= 0.0, "{:?}", &source[..end]);
            }
        }
    }
}

#[test]
fn a_kind_not_laid_out_is_left_to_be_shown_as_written() {
    assert!(docstream_diagram::diagram("classDiagram\nclass Animal").is_none());
    assert!(docstream_diagram::diagram("journey\ntitle My day").is_none());
    assert!(docstream_diagram::svg("graph TD\nA-->B").unwrap().starts_with("<svg"));
}

/// Every kind said to be laid out has a case that is.
#[test]
fn every_kind_named_has_a_case() {
    for kind in docstream_diagram::KINDS {
        assert!(cases().iter().any(|c| c.starts_with(kind) || c.starts_with(&format!("graph")) && kind == "flowchart"), "no case of {kind}");
    }
}

/// A diagram is called what its source calls it, or by the first words drawn in it; never by its kind.
#[test]
fn every_diagram_is_called_something_meaningful() {
    for source in cases() {
        let d = docstream_diagram::diagram(&source).unwrap();
        let kind = source.split_whitespace().next().unwrap();
        assert!(!d.title.is_empty() && !d.title.starts_with(kind), "{source}");
    }
    let d = docstream_diagram::diagram("flowchart LR\n    accTitle: 两步\n    accDescr {\n        先这样\n        再那样\n    }\n    A --> B").unwrap();
    assert_eq!((d.title.as_str(), d.description.as_deref()), ("两步", Some("先这样 再那样")));
    assert_eq!(d.items.iter().filter(|i| matches!(i, docstream_diagram::Item::Text { .. })).count(), 2, "no node is made of the accessible lines");
    assert_eq!(docstream_diagram::diagram("flowchart LR\n    A[开始] --> B[结束]").unwrap().title, "开始 → 结束");
}
