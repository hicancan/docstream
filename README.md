# docstream

A document kernel. Markdown, as people and models write it, is read into one document tree that every client draws
natively: a web page, an Android app, a Windows app, a blog's build. The kernel knows nothing of who draws the tree
or how; there is one reading of every document, everywhere.

**What is read.** CommonMark; GitHub's tables, task lists, strikethrough and alerts (`> [!TIP]`); Typora's
`==highlight==`, `^superscript^` and `~subscript~`; math as `$inline$`, `\(inline\)`, `$$display$$`, `\[display\]` or
a fence written `math`, kept as TeX; fenced code with its language. Emphasis follows the CJK-friendly rule, so
`**加粗**，` is bold. Raw HTML is kept as text, never as markup. Each heading carries its address in `anchor`, derived
from its parsed text and unique within the document. Readers, indexes and HTML use that address directly.

**As it streams.** `Stream.push(text)` gives the whole tree as it now stands, every top-level block with a key that
changes only when the block does, and marked settled once a blank line and something after it have come, when
nothing more can join it: a client redraws only what changed. The block still being written is read as if its open
marks (`**`, `` ` ``, `==`, `$`) were closed, so it does not flicker between plain and styled. Fed a character at a
time, a settled block never changes and the end equals reading the whole at once; the tests hold both.

**Math.** TeX is typeset on the device by TeX's own rules (The TeXbook, appendix G), every measure read from the
OpenType MATH table of New Computer Modern Math: fractions, radicals, scripts and limits, stretchy delimiters built
from the font's variants and assemblies, accents, matrices, cases, aligned equations. A formula comes back as glyph
outlines, rules and words (CJK in `\text{}`, drawn by the client's own font), or as SVG in the colour of the text
around it. What is not understood is shown as written; an unfinished formula, streamed a character at a time, still
typesets.

**Diagrams.** Mermaid flowcharts, state diagrams, sequence diagrams, mind maps, Gantt charts, timelines, pies and
quadrant charts are laid out on the device: a flowchart in layers by Sugiyama's method (cycles broken, ranks by
longest path, crossings reduced by barycentres and then by moving each node where it crosses least, positions
balanced), each edge's label given its own place in the rank between. Subgraphs are read as Mermaid reads them (a node
belongs to the first that names it; a subgraph has an id, nests in another and may be an edge's end; a composite state
is its box) and laid out as boxes: what a box holds stays together between its sides on every layer, so a box never
takes in what it does not hold and two boxes meet only when one is inside the other. A sequence diagram has its notes
and frames (loop, alt, opt, par); a mind map its root in the middle and its branches to either side; a Gantt chart its
ticks on whole units of the calendar (`tickInterval` and `weekday` read), with the year once the axis runs into
another; a quadrant chart its points named where their words cover nothing else. Labels may break lines with `<br>`
and be set bold with `<b>`, as Mermaid allows. A diagram comes back as paths and words, or as SVG in the colour of the
text around it, with what it is called for someone who cannot see it: its `accTitle`, else its title, else the first
words drawn in it (and `accDescr` as its description). A kind not laid out is left for the client to show as written.

## Layout

| Part | What |
| --- | --- |
| `core` | The reader and the tree (`docstream-core`), with golden fixtures in `core/tests/fixtures` |
| `math` | The typesetter (`docstream-math`): `tex` reads TeX, `layout` sets it; cases in `math/tests/cases.tex` |
| `diagram` | The diagram layout (`docstream-diagram`); cases in `diagram/tests/cases.md` |
| `html` | The tree drawn as HTML with formulas and diagrams as SVG (`docstream-html`), for pages built ahead of time |
| `wasm` | WebAssembly binding; `js` wraps it as the `docstream` package with TypeScript types |
| `native` | UniFFI binding for the native clients: Kotlin for Android (`top.hicancan.docstream`), C# for Windows |

## Build

```powershell
pwsh ./build.ps1          # Rust tests, the JS package and its tests, then the native libraries for Android and Windows
```

`UPDATE_GOLDEN=1 cargo test` rewrites the kept trees and layouts after a deliberate change;
`cargo run -p docstream-math --example formulas -- formulas.html` (and `-p docstream-diagram --example diagrams`) draws
every case on one page to look at.

Licensed under AGPL-3.0-only.
