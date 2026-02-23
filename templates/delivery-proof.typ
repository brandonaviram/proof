#let data = json("data.json")

#set document(title: data.client + " — Delivery Proof")
#set text(font: ("Apercu Pro", "Helvetica Neue", "Helvetica"), size: 9pt, fill: luma(30))

// ── Cover Page ───────────────────────────────────────────

#page(margin: (x: 1.5in, y: 2in))[
  #align(center + horizon)[
    #text(10pt, weight: "medium", tracking: 0.2em, fill: luma(120))[
      DELIVERY PROOF
    ]
    #v(2.5em)
    #text(32pt, weight: "light")[#data.client]
    #v(1em)
    #if data.title != none [
      #text(14pt, weight: "light", fill: luma(80))[#data.title]
      #v(1em)
    ]
    #text(11pt, fill: luma(120))[#data.date]
    #v(4em)
    #line(length: 2in, stroke: 0.5pt + luma(210))
    #v(1.5em)
    #text(9pt, fill: luma(140))[
      #str(data.summary.total_files) files
      #h(0.8em) · #h(0.8em)
      #data.summary.total_size
    ]
  ]
]

// ── Page settings for contact sheet + manifest ───────────

#set page(
  paper: "us-letter",
  margin: (x: 0.6in, top: 0.8in, bottom: 0.7in),
  header: [
    #set text(7.5pt, fill: luma(160))
    #data.client #h(1fr) #data.date
    #v(0.2em)
    #line(length: 100%, stroke: 0.25pt + luma(230))
  ],
  footer: context [
    #set text(7.5pt, fill: luma(160))
    #line(length: 100%, stroke: 0.25pt + luma(230))
    #v(0.2em)
    #h(1fr) #counter(page).display()
  ],
)

// ── Contact Sheet ────────────────────────────────────────

#text(13pt, weight: "medium")[Contact Sheet]
#v(0.8em)

#let cols = data.columns

#let cell-height = 110pt

#let make-cell(asset) = block(breakable: false)[
  #if data.auto_orient {
    box(
      width: 100%,
      height: cell-height,
      clip: true,
      radius: 2pt,
      stroke: 0.5pt + luma(220),
      fill: luma(250),
    )[
      #align(center + horizon)[
        #if asset.thumbnail != none {
          image(asset.thumbnail, height: cell-height, fit: "contain")
        } else {
          text(7pt, fill: luma(160))[No preview]
        }
      ]
    ]
  } else {
    box(
      width: 100%,
      clip: true,
      radius: 2pt,
      stroke: 0.5pt + luma(220),
    )[
      #if asset.thumbnail != none {
        image(asset.thumbnail, width: 100%)
      } else {
        rect(width: 100%, height: 50pt, fill: luma(245))[
          #align(center + horizon, text(7pt, fill: luma(160))[No preview])
        ]
      }
    ]
  }
  #v(3pt)
  #text(6pt, fill: luma(100))[#asset.filename]
]

#grid(
  columns: (1fr,) * cols,
  column-gutter: 8pt,
  row-gutter: 12pt,
  ..data.assets.map(make-cell)
)

// ── Manifest ─────────────────────────────────────────────

#pagebreak()

#text(13pt, weight: "medium")[Manifest]
#v(0.8em)

#table(
  columns: (1fr, auto, auto, auto, auto),
  stroke: none,
  inset: (x: 8pt, y: 5pt),
  fill: (_, row) => if row == 0 { luma(240) } else if calc.odd(row) { luma(248) } else { white },
  table.header(
    text(weight: "semibold", size: 8pt)[Filename],
    text(weight: "semibold", size: 8pt)[Type],
    text(weight: "semibold", size: 8pt)[Resolution],
    text(weight: "semibold", size: 8pt)[Format],
    text(weight: "semibold", size: 8pt)[Size],
  ),
  ..data.assets.map(asset => (
    text(size: 8pt)[#asset.filename],
    text(size: 8pt, fill: luma(80))[#asset.kind],
    text(size: 8pt)[#asset.resolution],
    text(size: 8pt, fill: luma(80))[#asset.format],
    text(size: 8pt)[#asset.human_size],
  )).flatten()
)

// ── Summary ──────────────────────────────────────────────

#v(2em)
#line(length: 100%, stroke: 0.25pt + luma(220))
#v(0.8em)
#text(9pt, fill: luma(100))[
  *#str(data.summary.total_files) files* delivered
  #h(0.5em) · #h(0.5em)
  #data.summary.total_size total
  #if data.summary.image_count > 0 [
    #h(0.5em) · #h(0.5em)
    #str(data.summary.image_count) images
  ]
  #if data.summary.video_count > 0 [
    #h(0.5em) · #h(0.5em)
    #str(data.summary.video_count) videos
  ]
]
