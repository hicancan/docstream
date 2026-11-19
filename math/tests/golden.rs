//! How each case in cases.tex is typeset, kept beside it as layout without outlines; `UPDATE_GOLDEN=1 cargo test`
//! writes it anew after a deliberate change, and the formulas example draws them to look at.
use std::fs;
use std::path::Path;

fn cases() -> Vec<String> {
    let text = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases.tex")).unwrap();
    text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('%')).map(String::from).collect()
}

#[test]
fn every_case_typesets_as_kept() {
    let kept = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases.json");
    let got: Vec<serde_json::Value> = cases().iter().flat_map(|tex| [false, true].map(|display| {
        let f = docstream_math::typeset(tex, display);
        serde_json::json!({ "tex": tex, "display": display, "width": f.width, "height": f.height, "depth": f.depth, "items": f.items })
    })).collect();
    let got = serde_json::to_string_pretty(&got).unwrap() + "\n";
    if std::env::var_os("UPDATE_GOLDEN").is_some() { fs::write(&kept, &got).unwrap(); return; }
    assert_eq!(got, fs::read_to_string(&kept).unwrap().replace("\r\n", "\n"));
}

/// A model writes TeX a character at a time; every unfinished piece of it still typesets to something finite.
#[test]
fn every_prefix_of_every_case_typesets() {
    for tex in cases() {
        for end in tex.char_indices().map(|(i, _)| i).chain([tex.len()]) {
            for display in [false, true] {
                let f = docstream_math::typeset(&tex[..end], display);
                assert!(f.width.is_finite() && f.height.is_finite() && f.depth.is_finite(), "{:?}", &tex[..end]);
                for item in &f.items { if let docstream_math::Item::Glyph { id, .. } = item { assert!(f.glyphs.contains_key(id)); } }
            }
        }
    }
}

#[test]
fn what_is_not_understood_is_shown_as_written() {
    let f = docstream_math::typeset(r"\notacommand{x}", false);
    assert!(!f.items.is_empty());
    assert_eq!(docstream_math::unknown(r"\frac{\notacommand{x}}{\alpha}"), [r"\notacommand"]);
    let svg = docstream_math::svg(r"a<b", true);
    assert!(svg.starts_with("<svg") && svg.contains("<title>a&lt;b</title>"));
}

/// Commands of LaTeX, amsmath and plain TeX that NJUPT's notes write are typeset, none kept as written.
#[test]
fn the_commands_notes_write_are_understood() {
    for tex in [r"\pmb{\alpha}", r"{\bf x}", r"{\rm d}", r"{\it t}", r"{\sf A}", r"{\tt a}", r"{\cal L}", r"\tag{1}", r"\tag*{(a)}",
        r"\textcircled{9}", r"\textcircled{12}", r"\textcircled{-}", r"\mathrel{\mathop:}", r"\mathbin{\#}", r"\framebox{2}", r"\xlongequal{c}",
        r"\preccurlyeq", r"\succcurlyeq", r"\bowtie", r"\Longleftrightarrow", r"\bigcirc", r"\bigtriangledown", r"\bigstar", r"\lessgtr",
        r"\lessdot", r"\gtrdot", r"\dag", r"\nmid", r"\circledast", r"\circledcirc", r"\triangleright", r"\ncong", r"\surd", r"\lll",
        r"\imath", r"\jmath", r"\lbrack x \rbrack", r"\yen", r"\AA", r"\dotsc", r"\hdots", r"\varDelta", r"\varPhi", r"\Vec{a}",
        r"\small x", r"\texttt{int}", r"\textsf{A}",
        r"\nrightarrow", r"\nRightarrow", r"\gtrsim", r"\lesssim", r"\ngtr", r"\rightharpoondown", r"\intercal", r"{a \atop b}",
        r"\oiint", r"\triangleq", r"\enspace", r"\hspace{2em}", r"\updownarrow", r"\Uparrow", r"\Downarrow", r"\rightleftarrows",
        r"\curvearrowright", r"\twoheadrightarrow", r"\Rsh", r"\Vdash", r"\sqsubseteq", r"\varkappa", r"\bigsqcup", r"\overleftrightarrow{AB}",
        r"\boxminus", r"\hspace{1.5cm}", r"\hspace*{3pt}"] {
        assert!(docstream_math::unknown(tex).is_empty(), "{tex}: {:?}", docstream_math::unknown(tex));
    }
}

/// Commands that are not math are kept as written, so that a note written wrongly shows it and is put right.
#[test]
fn commands_that_are_not_math_are_kept_as_written() {
    for tex in [r"\Nu", r"\textmu", r"\multicolumn{2}{c}{x}", r"\hspace{wide}"] {
        assert!(!docstream_math::unknown(tex).is_empty(), "{tex}");
    }
}
