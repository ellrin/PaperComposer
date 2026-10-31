// Preview of LaTeX prose: math through KaTeX, common text commands as HTML.
// This is a reading aid for the editor; the submitted PDF is always built by
// real LaTeX with the publisher's class.

const MATH_ENVS = ["equation", "align", "gather", "multline", "eqnarray", "displaymath", "math"];
const ENV_RE = new RegExp(`\\\\begin\\{(${MATH_ENVS.join("|")})(\\*?)\\}([\\s\\S]*?)\\\\end\\{\\1\\2\\}`, "y");

window.PCTex = (() => {
  let macros = {};
  let labels = {}; // label -> equation number

  function setMacros(m) { macros = { ...m }; }

  /** Number display equations in reading order and map their labels. */
  function numberEquations(paragraphs) {
    labels = {};
    let n = 0;
    const re = new RegExp(`\\\\begin\\{(${MATH_ENVS.join("|")})(\\*?)\\}([\\s\\S]*?)\\\\end\\{\\1\\2\\}`, "g");
    paragraphs.forEach((p) => {
      for (const m of p.matchAll(re)) {
        if (m[2] === "*" || /\\nonumber|\\notag/.test(m[3])) continue;
        n += 1;
        const lab = m[3].match(/\\label\{([^}]*)\}/);
        if (lab) labels[lab[1]] = n;
      }
    });
    return labels;
  }

  function katexNode(src, display) {
    const span = document.createElement(display ? "div" : "span");
    span.className = display ? "math display" : "math";
    if (!window.katex) { span.textContent = src; return span; }
    try {
      katex.render(src, span, { displayMode: display, throwOnError: false, macros: { ...macros }, strict: "ignore", trust: false });
    } catch {
      span.textContent = src;
    }
    return span;
  }

  function envNode(env, star, body) {
    let src = body.replace(/\\label\{[^}]*\}/g, "");
    const lab = body.match(/\\label\{([^}]*)\}/);
    const num = lab && labels[lab[1]];
    const inner = env === "align" || env === "eqnarray" ? `\\begin{aligned}${src}\\end{aligned}`
      : env === "gather" ? `\\begin{gathered}${src}\\end{gathered}`
      : env === "multline" ? `\\begin{split}${src}\\end{split}`
      : src;
    const tagged = !star && !/\\nonumber|\\notag/.test(src) && num ? `${inner}\\tag{${num}}` : inner;
    return katexNode(tagged, true);
  }

  // Read a balanced {group} starting at s[i] === "{". Returns [content, nextIndex].
  function group(s, i) {
    let depth = 0;
    for (let j = i; j < s.length; j++) {
      if (s[j] === "\\") { j++; continue; }
      if (s[j] === "{") depth++;
      else if (s[j] === "}" && --depth === 0) return [s.slice(i + 1, j), j + 1];
    }
    return [s.slice(i + 1), s.length];
  }

  const SYMBOLS = { "\\%": "%", "\\&": "&", "\\_": "_", "\\#": "#", "\\$": "$", "\\{": "{", "\\}": "}", "\\ldots": "…", "\\dots": "…", "\\LaTeX": "LaTeX", "\\TeX": "TeX", "\\\\": "\n" };
  const SYMBOL_KEYS = Object.keys(SYMBOLS).sort((a, b) => b.length - a.length);

  /** Render a LaTeX prose string into a DocumentFragment. */
  function render(text) {
    const frag = document.createDocumentFragment();
    let buf = "";
    const flush = () => {
      if (!buf) return;
      frag.append(buf.replace(/---/g, "—").replace(/--/g, "–").replace(/~/g, " ").replace(/``/g, "“").replace(/''/g, "”"));
      buf = "";
    };
    let i = 0;
    while (i < text.length) {
      const c = text[i];
      const rest = text.slice(i);
      // display math
      if (rest.startsWith("$$")) {
        const end = text.indexOf("$$", i + 2);
        if (end > 0) { flush(); frag.append(katexNode(text.slice(i + 2, end), true)); i = end + 2; continue; }
      }
      if (rest.startsWith("\\[")) {
        const end = text.indexOf("\\]", i + 2);
        if (end > 0) { flush(); frag.append(katexNode(text.slice(i + 2, end), true)); i = end + 2; continue; }
      }
      if (rest.startsWith("\\begin{")) {
        ENV_RE.lastIndex = i;
        const m = ENV_RE.exec(text);
        if (m) { flush(); frag.append(envNode(m[1], m[2] === "*", m[3])); i = ENV_RE.lastIndex; continue; }
      }
      // inline math
      if (c === "$") {
        const end = text.indexOf("$", i + 1);
        if (end > 0) { flush(); frag.append(katexNode(text.slice(i + 1, end), false)); i = end + 1; continue; }
      }
      if (rest.startsWith("\\(")) {
        const end = text.indexOf("\\)", i + 2);
        if (end > 0) { flush(); frag.append(katexNode(text.slice(i + 2, end), false)); i = end + 2; continue; }
      }
      if (c === "\\") {
        // A symbol command; word commands such as \dots must not run into a letter.
        const sym = SYMBOL_KEYS.find((k) => rest.startsWith(k) && (!/[a-zA-Z]$/.test(k) || !/[a-zA-Z]/.test(text[i + k.length] ?? "")));
        if (sym) { buf += SYMBOLS[sym]; i += sym.length; continue; }
        const name = rest.match(/^\\([a-zA-Z]+)\*?/);
        if (name) {
          let j = i + name[0].length;
          while (text[j] === " " && text[j + 1] === "{") j++;
          const arg = text[j] === "{" ? group(text, j) : null;
          const cmd = name[1];
          const wrap = (tag, content) => { flush(); const n = document.createElement(tag); n.append(render(content)); frag.append(n); };
          if (arg && ["emph", "textit"].includes(cmd)) { wrap("em", arg[0]); i = arg[1]; continue; }
          if (arg && cmd === "textbf") { wrap("strong", arg[0]); i = arg[1]; continue; }
          if (arg && cmd === "underline") { wrap("u", arg[0]); i = arg[1]; continue; }
          if (arg && cmd === "textsc") { wrap("span", arg[0]); i = arg[1]; frag.lastChild.style.fontVariant = "small-caps"; continue; }
          if (arg && ["texttt", "url", "path"].includes(cmd)) { wrap("code", arg[0]); i = arg[1]; continue; }
          if (arg && ["textrm", "textsf", "text", "mbox"].includes(cmd)) { buf += ""; frag.append(render(arg[0])); i = arg[1]; continue; }
          if (arg && /^(cite|citep|citet|citealp|parencite|textcite)$/.test(cmd)) {
            flush();
            const n = document.createElement("span");
            n.className = "cite";
            n.textContent = `[${arg[0].split(",").map((s) => s.trim()).join(", ")}]`;
            frag.append(n);
            i = arg[1];
            continue;
          }
          if (arg && (cmd === "eqref" || cmd === "ref" || cmd === "autoref" || cmd === "cref")) {
            flush();
            const n = document.createElement("span");
            const num = labels[arg[0]];
            n.className = "ref" + (num ? "" : " missing");
            n.textContent = num ? (cmd === "eqref" ? `(${num})` : String(num)) : `[${arg[0]}]`;
            n.title = arg[0];
            frag.append(n);
            i = arg[1];
            continue;
          }
          if (arg && cmd === "footnote") {
            flush();
            const n = document.createElement("sup");
            n.className = "fn";
            n.textContent = "*";
            n.title = arg[0];
            frag.append(n);
            i = arg[1];
            continue;
          }
          if (arg && cmd === "label") { i = arg[1]; continue; }
        }
      }
      buf += c;
      i++;
    }
    flush();
    return frag;
  }

  /** True when a paragraph is a display block rather than prose. */
  function isDisplay(p) {
    const t = p.trim();
    return /^\\begin\{(equation|align|gather|multline|eqnarray|displaymath)\*?\}/.test(t) || t.startsWith("\\[") || t.startsWith("$$");
  }

  return { setMacros, numberEquations, render, isDisplay, get labels() { return labels; } };
})();
