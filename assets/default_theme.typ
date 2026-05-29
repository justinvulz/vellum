// Inline link to another note in this vault. The app intercepts URLs
// with the `vellum://` scheme and opens the matching note. The link
// colour is applied by `template`'s `show link: set text(fill: ...)`
// rule below, so changing `link-color` on the template call retints
// every #line-note in the document.
//   #line-note("project-a")            -> "project-a"
//   #line-note("project-a", body: [A]) -> "A"
#let line-note(name, body: none) = link(
  "vellum://" + name,
  if body == none { name } else { body },
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

// `width`, `size`, `bg`, `text-color`, and `link-color` are all passed
// in by the app (see editor::preamble::wrap_for_render); the defaults
// match assets/default_config.toml so `typst compile` produces
// something coherent when the file is opened standalone.
#let template(
  doc,
  width: 600pt,
  size: 16pt,
  bg: rgb("#222831"),
  text-color: rgb("#dfd0b8"),
  link-color: rgb("#948979"),
) = {
  set page(
    fill: bg,
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
      // CJK fallbacks — kept in sync with the default cjk_families list
      // in assets/default_config.toml so plain and rendered blocks
      // resolve the same glyphs. Typst's font list is per-codepoint
      // fallback, so Latin still picks an earlier face.
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
    fill: text-color,
    size: size
  )

  // One rule covers every `#link(...)` in the document — including
  // `#line-note(...)`, which is defined above as a plain `link`.
  show link: set text(fill: link-color)

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
