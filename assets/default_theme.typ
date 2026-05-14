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

  // show math.equation.where(block: false): box


  doc
}
