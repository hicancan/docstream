//! The fixture every client reads, drawn as HTML.
use docstream_html::html;

#[test]
fn draws_every_kind_of_block_with_formulas_and_diagrams_as_svg() {
    let page = html("## 标题 One\n\n## 标题 One\n\n> [!TIP]\n> 小心 **加粗**\n\n- [x] 完成\n- 未完\n\n行内 $x^2$\n\n$$\\frac{a}{b}$$\n\n```mermaid\ngraph TD\nA-->B\n```\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|:-|-:|\n| 1 | 2 |\n\n[x](javascript:alert(1)) ![图](../media/a.png)");
    assert!(page.contains("<h2 id=\"标题-one\">") && page.contains("<h2 id=\"标题-one-1\">"));
    assert!(page.contains("markdown-alert-tip") && page.contains("<strong>加粗</strong>"));
    assert!(page.contains("<input type=\"checkbox\" disabled checked>"));
    assert!(page.contains("<span class=\"math math-inline\"><svg") && page.contains("<div class=\"math math-display\"><svg"));
    assert!(page.contains("<figure class=\"diagram\"><svg"));
    assert!(page.contains("<pre data-language=\"rust\"><code>fn main() {}</code></pre>"));
    assert!(page.contains("style=\"text-align:right\""));
    assert!(page.contains("<a href=\"#\">") && page.contains("<img src=\"../media/a.png\" alt=\"图\" loading=\"lazy\">"));
}

#[test]
fn raw_html_is_shown_as_text() {
    assert!(html("<script>alert(1)</script>").contains("&lt;script&gt;"));
}

#[test]
fn the_html_a_document_may_write_is_drawn_as_what_it_means_and_nothing_else_runs() {
    let page = html("H<sub>2</sub>O <a href=\"javascript:alert(1)\">x</a> <img src=\"a.png\" onerror=\"alert(1)\">\n\n<table><tr><th rowspan=\"2\">头</th><td colspan=\"2\">$x$</td></tr><tr><td>a</td><td>b</td></tr></table>");
    assert!(page.contains("H<sub>2</sub>O") && page.contains("<a href=\"#\">x</a>") && page.contains("<img src=\"a.png\" alt=\"\" loading=\"lazy\">"));
    assert!(!page.contains("onerror"));
    assert!(!page.contains("<thead>") && page.contains("<tbody><tr><th rowspan=\"2\">头</th><td colspan=\"2\"><span class=\"math math-inline\"><svg"));
}
