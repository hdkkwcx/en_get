// // 代码
// #import "@preview/codly:1.0.0": *
// show: codly-init.with()
// show: codly-init.with()
// codly(stroke: 1pt + blue)
// codly(display-icon: true)
#import "@preview/mitex:0.2.5": mi, mitex
#import "@preview/showybox:2.0.4": showybox
#import "@preview/physica:0.9.4": *
#import "@preview/cuti:0.3.0": show-cn-fakebold

#let rm-bracket(it) = {
  show "{": ""
  show "}": ""
  it
}

#let beamer-block(value) =  {
  block(
    fill: luma(240), 
    inset: 0.6em,
    stroke: (left: 0.25em))[#value]
}

#let set-supplement(it) = {
  it
}

#let num-include-chapter(num) = {
  numbering("1.1", counter(heading).get().first(), num)
}

#let ref-zh(it, _numbering: "1.1") = {
  let el = it.element
  let eq = math.equation
  let count;
  if el != none {
    let count_head = counter(heading).get().first()
    
    if el.func() == eq {
      count = counter(eq).at(el.location())
    } else if el.kind == figure {
      count = counter(figure.where(kind: figure)).at(el.location())
      // link(el.location(), numbering(_numbering, count_head, ..count))
    } else if el.kind == table {
      count = counter(figure.where(kind: table)).at(el.location())
      // link(el.location(), numbering(_numbering, count_head, ..count))
    } else if el.kind == raw {
      count = counter(figure.where(kind: raw)).at(el.location())
    }
    // link(el.location(), numbering(_numbering, count_head, ..count))
    link(el.location(), numbering("1", ..count))
    // else if el.func() == table {
    //   link(el.location(), numbering("表 1",  ..counter(tab).at(el.location())))
    // } 
    // el.func()
  } else {
    it
  }
}

#let btext(x, color: red, bg: rgb("fffd11a1")) = {
  highlight(fill: bg)[
    #text(x, fill: color)
  ]
}

#let set-figure(body) = {
  // set figure(numbering: it => strong[#numbering("1.1", it)])
  set figure.caption(separator: ". ")
  // set figure(numbering: it => strong[#numbering("1.1", it)])
  // set figure(numbering: num => numbering("1.1", counter(heading).get().first(), num))
  set figure(numbering: num => numbering("1", num))
  // set figure(numbering: num => numbering("1.1", counter(heading).get().first(), num))
  
  show figure.where(kind: image): set figure(supplement:  text("图", weight:"bold"))
  show figure.where(kind: table): set figure(supplement:  text("表", weight:"bold"))
  show figure.where(kind: table): set figure.caption(position: top)
  body
}


#let set-table(it) = {
  show table.cell.where(y: 0): strong
  // See the strokes section for details on this!
  let frame(stroke) = (x, y) => (
    left: 0pt, right: 0pt,
    // left: if x > 0 { 0pt } else { stroke },
    // right: stroke,
    top: if y < 2 { stroke } else { 0pt },
    bottom: stroke,
  )
  set table(
    // fill: (rgb("EAF2F5"), none),
    stroke: frame(rgb("21222C")),
    align: (x, y) => ( if x > 0 { center } else { left } )
  )
  it
}

#let template(doc, size: 13pt, footer: "", header: "", _numbering: "1.1") = {
  set text(size: size) // font: ("Microsoft YaHei")
  set heading(numbering: _numbering)
  set par(justify: true)
  set list(indent: 1em)

  show ref: ref-zh  // 引用

  // https://github.com/typst/typst/issues/1896
  show heading.where(level: 1): it => {
    counter(math.equation).update(0)
    counter(figure.where(kind: image)).update(0)
    counter(figure.where(kind: table)).update(0)
    counter(figure.where(kind: raw)).update(0)
    it
  }
  // set math.equation(numbering: num => numbering("(1.1)", counter(heading).get().first(), num))
  set math.equation(numbering: "(1)")
  
  show: set-figure
  show: set-table
  show: show-cn-fakebold

  // 链接
  show link: underline
  show link: set text(fill: rgb(0, 0, 255))

  // 代码
  show raw: set text(font: ("consolas", "Microsoft Yahei", "SimSun"), size: 10pt)

  // 页眉页脚
  
  // footer
  set page(
    footer: context [
      #text(footer, weight: "bold")
      #h(1fr)
      #counter(page).display("1/1", both: true)
    ], 
    header: align(right)[
      #text(header, weight: "bold")
    ],
    numbering: "1",
  )

  show regex("==(\w+)=="): x => {
    // let txt = x.fields().text // content object
    let txt = x.text
    txt = txt.replace("==", "")
    btext(txt)
  }
  doc
}

#let nonum(eq) = math.equation(block: true, numbering: none, eq)

#let box-blue(it) = {
  showybox(it, frame: (
    // title-color: red.darken(30%),
    border-color: blue.darken(10%),
    body-color: blue.lighten(95%) ), 
    // title: [*比热容*]
  )
}

#let box-red(it) = {
  showybox(it, frame: (
    // title-color: red.darken(30%),
    border-color: red.darken(30%),
    body-color: red.lighten(95%) ),
    // title: [*比热容*]
  )
}
