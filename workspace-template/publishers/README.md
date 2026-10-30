# Publishers

One folder per publisher or journal, e.g. `elsevier-elsarticle/`:

```
elsevier-elsarticle/
  elsarticle.cls
  elsarticle-num.bst
  sample.tex          the publisher's example document
  template.toml       written by the app after you confirm the field mapping
```

Drop the files in; the app's template import (coming next) reads `sample.tex`
and proposes the mapping for title, authors, affiliations, abstract and keywords.
