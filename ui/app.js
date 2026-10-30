// PaperComposer UI shell (dev mode, served by pc-server).
// The document here is a demo held in memory; file-backed documents come next.

const SECTIONS = ["Introduction", "Method", "Result", "Discussion", "Conclusion"];

const DEMO = {
  Introduction: [
    "Retrieval-augmented generation has become central to question answering over the past decade.\nModels now consult external documents before they answer.\nThis reduces hallucination but adds a retrieval step that can fail silently.",
    "Despite recent advances in retrieval, long-context grounding remains largely unexplored.\nExisting approaches either truncate the context or utilize expensive re-ranking at every step, which limits their use on long scientific documents that routinely exceed the context window of current models by a large margin.",
    "In this paper, we propose a two-stage retriever, which separates recall from precision.\nOur contributions are threefold.",
  ],
  Method: [
    "Let $x_t$ denote the input at step $t$, where $t \\le T$.\nWe adopt a dense encoder because it handles paraphrase well.\nThe second stage scores each candidate against the query.",
  ],
  Result: [
    "Compared with BM25, our retriever improves recall@10 by 12.4\\%.\nThis gain is most pronounced on long inputs, where the baseline truncates context.\nThese results indicate that context handling drives the improvement.",
  ],
  Discussion: [],
  Conclusion: [],
};

const $ = (s) => document.querySelector(s);
const store = {
  get(k, d) { try { return JSON.parse(localStorage.getItem(k)) ?? d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch {} },
};

const state = {
  doc: store.get("pc.doc", DEMO),
  section: store.get("pc.section", "Introduction"),
  current: 0,
  reports: [],
  profile: null,
};

// ── API ────────────────────────────────────────────────
async function api(path, body) {
  const r = await fetch(path, body
    ? { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
    : undefined);
  if (!r.ok) throw new Error(`${path}: ${r.status}`);
  return r.status === 204 ? null : r.json();
}

// ── Theme ──────────────────────────────────────────────
function applyTheme() {
  const t = store.get("pc.theme", "orange");
  const sys = matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  const m = store.get("pc.mode", sys);
  document.documentElement.dataset.theme = t;
  document.documentElement.dataset.mode = m;
  document.documentElement.classList.toggle("dim-paper", store.get("pc.dim", false));
  document.querySelectorAll(".theme[data-theme]").forEach((b) =>
    b.setAttribute("aria-pressed", String(b.dataset.theme === t)));
  $("#dimPaper").setAttribute("aria-pressed", String(store.get("pc.dim", false)));
  $("#mode").textContent = m === "dark" ? "☾" : "☀";
}

// ── Rendering ──────────────────────────────────────────
const paras = () => state.doc[state.section] ?? [];
const worst = (r) => r?.status ?? (r ? "ok" : "");

function renderSections() {
  const nav = $("#sections");
  nav.innerHTML = "";
  SECTIONS.forEach((name, i) => {
    const b = document.createElement("button");
    b.innerHTML = `<span class="n">${String(i + 1).padStart(2, "0")}</span>${name}`;
    b.setAttribute("aria-current", String(name === state.section));
    const ps = state.doc[name] ?? [];
    if (ps.length && name !== state.section && state.sectionClean?.[name]) b.classList.add("done");
    b.onclick = () => { state.section = name; state.current = 0; store.set("pc.section", name); refresh(); };
    nav.append(b);
  });
  const i = SECTIONS.indexOf(state.section);
  $("#sectionStep").textContent = `STEP ${String(i + 1).padStart(2, "0")}`;
  $("#sectionName").textContent = state.section;
}

function renderPaper() {
  const paper = $("#paper");
  paper.innerHTML = `<h1>${state.section}</h1>`;
  paras().forEach((text, i) => {
    const p = document.createElement("p");
    p.className = "para" + (i === state.current ? " current" : "");
    const rep = state.reports[i];
    const flagged = new Map();
    rep?.findings.forEach((f) => {
      if (f.sentence == null) return;
      const prev = flagged.get(f.sentence);
      if (!prev || rank(f.severity) > rank(prev)) flagged.set(f.sentence, f.severity);
    });
    const sents = rep?.sentence_text ?? [text.replace(/\s+/g, " ")];
    sents.forEach((s, si) => {
      const span = document.createElement("span");
      span.className = "s" + (flagged.has(si) ? ` flag ${flagged.get(si)}` : "");
      span.textContent = s + " ";
      p.append(span);
    });
    p.onclick = () => { state.current = i; renderSide(); markCurrent(); };
    p.ondblclick = () => edit(p, i);
    paper.append(p);
  });
  if (!paras().length) {
    const e = document.createElement("p");
    e.className = "para";
    e.style.color = "#8d8d94";
    e.textContent = "雙擊開始寫第一段";
    e.ondblclick = () => { state.doc[state.section] = [""]; edit(e, 0); };
    paper.append(e);
  }
}

const rank = (s) => ({ info: 1, warning: 2, error: 3 }[s] ?? 0);

function markCurrent() {
  document.querySelectorAll(".para").forEach((p, i) => p.classList.toggle("current", i === state.current));
  document.querySelectorAll(".outline button").forEach((b, i) => b.setAttribute("aria-current", String(i === state.current)));
}

function edit(p, i) {
  state.current = i;
  const ta = document.createElement("textarea");
  ta.value = paras()[i] ?? "";
  const fit = () => { ta.style.height = "auto"; ta.style.height = ta.scrollHeight + "px"; };
  ta.oninput = fit;
  ta.onkeydown = (e) => {
    if (e.key === "Escape" || (e.key === "Enter" && (e.metaKey || e.ctrlKey))) ta.blur();
  };
  ta.onblur = async () => {
    const v = ta.value.trim();
    const list = paras().slice();
    if (v) list[i] = v; else list.splice(i, 1);
    state.doc[state.section] = list;
    store.set("pc.doc", state.doc);
    await refresh();
  };
  p.replaceChildren(ta);
  p.ondblclick = null;
  fit();
  ta.focus();
}

function renderOutline() {
  const ol = $("#outline");
  ol.innerHTML = "";
  paras().forEach((text, i) => {
    const li = document.createElement("li");
    const b = document.createElement("button");
    b.setAttribute("aria-current", String(i === state.current));
    b.innerHTML = `<span class="status ${worst(state.reports[i])}"></span><span class="i">${String(i + 1).padStart(2, "0")}</span><span class="t"></span>`;
    b.querySelector(".t").textContent = text.split("\n")[0];
    b.onclick = () => {
      state.current = i;
      markCurrent();
      renderSide();
      document.querySelectorAll(".para")[i]?.scrollIntoView({ block: "center", behavior: "smooth" });
    };
    li.append(b);
    ol.append(li);
  });
}

function renderFindings() {
  const ul = $("#findings");
  ul.innerHTML = "";
  const r = state.reports[state.current];
  if (!r) { ul.innerHTML = `<li class="empty">—</li>`; return; }
  if (!r.findings.length && !r.pending.length) { ul.innerHTML = `<li><span class="status"></span><span>通過</span></li>`; return; }
  r.findings.forEach((f) => {
    const li = document.createElement("li");
    li.innerHTML = `<span class="status ${f.severity}"></span><span><span class="m"></span><span class="rid">${f.rule}${f.sentence != null ? ` · s${f.sentence + 1}` : ""}</span></span>`;
    li.querySelector(".m").textContent = f.message;
    ul.append(li);
  });
  const rules = state.profile?.rules_by_id ?? {};
  r.pending.forEach((id) => {
    const li = document.createElement("li");
    li.className = "pending";
    li.innerHTML = `<span class="status info"></span><span><span class="m"></span><span class="rid">${id}</span></span>`;
    li.querySelector(".m").textContent = rules[id] ?? id;
    ul.append(li);
  });
}

let searchTimer;
async function renderHits() {
  const params = new URLSearchParams({ section: state.section, limit: "8" });
  if ($("#q").value.trim()) params.set("text", $("#q").value.trim());
  if ($("#fn").value) params.set("function", $("#fn").value);
  if ($("#pos").value) params.set("position", $("#pos").value);
  const ul = $("#hits");
  let hits = [];
  try { hits = await api(`/api/phrases?${params}`); } catch { ul.innerHTML = `<li class="empty">—</li>`; return; }
  ul.innerHTML = "";
  if (!hits.length) { ul.innerHTML = `<li class="empty">沒有符合的句型</li>`; return; }
  hits.forEach(({ phrase }) => {
    const li = document.createElement("li");
    const b = document.createElement("button");
    b.className = "hit";
    b.innerHTML = `<div class="meta"><span>${phrase.id}</span><span>${phrase.function}</span><span>${phrase.position}</span></div><div class="tpl"></div>`;
    // Slots in accent, everything else as plain text.
    phrase.template.split(/(\{[^}]*\})/).forEach((part) => {
      const n = part.startsWith("{") ? document.createElement("i") : document.createTextNode(part);
      if (n.nodeType === 1) n.textContent = part;
      b.querySelector(".tpl").append(n);
    });
    b.onclick = async () => {
      try { await navigator.clipboard.writeText(phrase.template); } catch {}
      b.querySelector(".meta").lastElementChild.textContent = "已複製";
    };
    li.append(b);
    ul.append(li);
  });
}

function renderStatus() {
  const words = state.reports.reduce((n, r) => n + r.words, 0);
  const p = state.profile;
  $("#statusbar").innerHTML =
    `<span class="accent">${String(SECTIONS.indexOf(state.section) + 1).padStart(2, "0")} / ${state.section}</span>` +
    `<span>${paras().length} 段 · ${words} 字</span>` +
    `<span><span class="dot"></span>未連接模型</span>` +
    (p ? `<span>${p.phrases} 句型 · ${p.rules} 規則</span>` : "");
}

function renderSide() { renderFindings(); renderHits(); }

async function refresh() {
  const text = paras().join("\n\n");
  try { state.reports = text.trim() ? await api("/api/check", { text, section: state.section }) : []; }
  catch { state.reports = []; }
  if (state.current >= paras().length) state.current = Math.max(0, paras().length - 1);
  renderSections();
  renderPaper();
  renderOutline();
  renderSide();
  renderStatus();
}

async function loadProfile() {
  const chips = $("#profileChips");
  try {
    const p = await api("/api/profile");
    state.profile = p;
    chips.innerHTML = "";
    const name = document.createElement("span");
    name.className = "chip";
    name.textContent = p.error ? "設定檔讀取失敗" : p.name;
    name.title = p.error ?? p.path;
    chips.append(name);
    if (p.demo) {
      const d = document.createElement("span");
      d.className = "chip warn";
      d.textContent = "示範資料";
      d.title = "尚未指定你的私人設定檔，目前使用內建範本";
      chips.append(d);
    }
    if (p.issues.length) {
      const i = document.createElement("span");
      i.className = "chip warn";
      i.textContent = `${p.issues.length} 筆格式問題`;
      i.title = p.issues.map((x) => `${x.file}:${x.line} ${x.column ?? ""} ${x.message}`).join("\n");
      chips.append(i);
    }
    const fn = $("#fn");
    p.functions.forEach((f) => fn.append(new Option(f, f)));
  } catch {
    chips.innerHTML = `<span class="chip warn">伺服器未啟動</span>`;
  }
  // Soft-rule messages, for the pending list.
  try {
    const r = await fetch("/api/profile/rules");
    if (r.ok) state.profile.rules_by_id = await r.json();
  } catch {}
}

// ── Wiring ─────────────────────────────────────────────
function wire() {
  document.querySelectorAll(".block .toggle").forEach((t) => {
    t.onclick = () => {
      const b = t.closest(".block");
      const open = b.dataset.open !== "true";
      b.dataset.open = String(open);
      t.textContent = open ? "−" : "+";
    };
  });

  let tip;
  const closeTip = () => { tip?.remove(); tip = null; document.querySelectorAll(".help").forEach((h) => h.setAttribute("aria-expanded", "false")); };
  document.querySelectorAll(".help").forEach((h) => {
    h.setAttribute("aria-expanded", "false");
    h.onclick = (e) => {
      e.stopPropagation();
      const was = h.getAttribute("aria-expanded") === "true";
      closeTip();
      if (was) return;
      tip = document.createElement("div");
      tip.className = "tip";
      tip.textContent = h.dataset.tip;
      document.body.append(tip);
      const r = h.getBoundingClientRect();
      tip.style.top = `${r.bottom + 6}px`;
      tip.style.left = `${Math.max(8, Math.min(r.left - 100, innerWidth - 260))}px`;
      h.setAttribute("aria-expanded", "true");
    };
  });
  document.addEventListener("click", closeTip);
  document.addEventListener("keydown", (e) => { if (e.key === "Escape") closeTip(); });

  document.querySelectorAll(".theme[data-theme]").forEach((b) => {
    b.onclick = () => { store.set("pc.theme", b.dataset.theme); applyTheme(); };
  });
  $("#mode").onclick = () => {
    store.set("pc.mode", document.documentElement.dataset.mode === "dark" ? "light" : "dark");
    applyTheme();
  };
  $("#dimPaper").onclick = () => { store.set("pc.dim", !store.get("pc.dim", false)); applyTheme(); };

  $("#q").oninput = () => { clearTimeout(searchTimer); searchTimer = setTimeout(renderHits, 150); };
  $("#fn").onchange = renderHits;
  $("#pos").onchange = renderHits;
  $("#generate").title = "需要連接模型引擎";
}

applyTheme();
wire();
loadProfile().then(refresh);
