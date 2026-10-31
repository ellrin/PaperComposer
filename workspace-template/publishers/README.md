# Publishers

One folder per publisher or journal, e.g. `elsevier-elsarticle/`:

```
elsevier-elsarticle/
  elsarticle.cls
  elsarticle-num.bst
  sample.tex          the publisher's example document
  template.toml       written by the app after you confirm the field mapping
  *.csl               optional: citation style for Word export
  reference.docx      optional: Word styles for Word export
```

New papers get a copy of every file here except `sample.tex`. Word export
(Pandoc) uses the paper's `.csl` for citations and `reference.docx` for
fonts and headings. The `.csl` files come from the Citation Style Language
project (github.com/citation-style-language/styles, CC BY-SA 3.0).

Drop the files in; the app's template import (coming next) reads `sample.tex`
and proposes the mapping for title, authors, affiliations, abstract and keywords.
