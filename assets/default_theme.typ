// Inline link to another note in this vault. The app intercepts URLs
// with the `vellum://` scheme and opens the matching note.
//   #line-note("project-a")            -> "project-a"
//   #line-note("project-a", body: [A]) -> "A"
#let line-note(name, body: none) = link(
  "vellum://" + name,
  text(fill: rgb("#4a9eff"))[#if body == none { name } else { body }],
)

#let al(itm) = {
  return n => grid(
    columns: (0em, auto),
    align: bottom,
    hide[一], numbering(itm, n)
  )
}

#let listal = {
  grid(
    columns: (0em, auto),
    align: bottom,
    hide[一], [•]
  )
}

// Width and size are passed in by the app (see style.rs); defaults here
// just keep `typst compile` workable when the file is opened standalone.
#let template(doc, width: 600pt, size: 16pt) = {
  set page(
    fill: rgb("#0d0d0d"),
    width: width,
    height: auto,
    margin: 4pt,
    )


  set text(
    font: (
      "Inter",
      "Noto Sans",
      "DejaVu Sans",
      "Liberation Sans",
      "Ubuntu",
      "Helvetica",
      "Arial",
      // CJK fallbacks — kept in sync with style::CJK_FAMILIES so plain
      // and rendered blocks resolve the same glyphs. Typst's font list
      // is per-codepoint fallback, so Latin still picks an earlier face.
      "Noto Sans SC",
      "Noto Sans TC",
      "Noto Sans JP",
      "Noto Sans KR",
      "Source Han Sans SC",
      "Source Han Sans TC",
      "Source Han Sans",
      "PingFang SC",
      "PingFang TC",
      "Hiragino Sans",
      "Microsoft YaHei",
      "Microsoft JhengHei",
      "SimSun",
      "WenQuanYi Micro Hei",
      "WenQuanYi Zen Hei",
    ),
    top-edge: "ascender",
    bottom-edge: "descender",
    lang: "en",
    fill: rgb("#d4d4d4"),
    size: size
  )

  set heading(numbering: "1.")
  // set heading(numbering: "あ.")

  show heading: it =>[
    #text(weight: "bold")[#it]
    // #v(0.65em)
  ]
  
  // show heading.where(level: 1): it => {
  //   counter(math.equation).update(0)
  //   text(weight: "bold")[#it]
  //   v(0.65em)
  // }

  // set par(leading: 0.8em)
  show math.equation: set text(weight: "extralight")
  // show math.equation.where(block: true): e => [
  //   #block(width: 100%, inset: 0.3em)[
  //     #set align(center)
  //     #set par(leading: 0.65em)
  //     #e
  //   ]
  // ]

  // set math.equation(numbering: "(1.1)")

  // show: equate.with(breakable: true, sub-numbering: true,number-mode: "label")

  show ref: it => {
    let eq = math.equation
    let el = it.element

    if el != none and el.func() == eq {
    // Override equation references.
      numbering(
        el.numbering,
        ..counter(eq).at(el.location())
      )
    } else {
      // Other references as usual.
      it
    }
  }

  
  set list(marker: listal)

  set enum(numbering: al("1."))

  set math.cases(gap: 1em)

  // Centre tables and grids horizontally on the page. Wraps each
  // element in `align(center, …)` rather than touching its internal
  // cell alignment, so per-cell `align:` still works as written.
  show table: it => align(center, it)
  show grid: it => align(center, it)

  // show math.equation.where(block: false): box


  doc
}
