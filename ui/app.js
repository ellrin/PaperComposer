// PaperComposer UI.
// Top bar: global actions (file, template, database, settings).
// Left: navigator of sections and paragraphs. Centre: the whole paper.
// Right: one-shot writing box and checks for the current paragraph.

const ROLES = ["Abstract", "Introduction", "Related work", "Method", "Result", "Discussion", "Conclusion", "Any"];
const TABLES = [
  ["phrases.csv", "句型"],
  ["expressions.csv", "片語"],
  ["vocabulary.csv", "用詞"],
  ["rules.csv", "規則"],
  ["exemplars.csv", "範例"],
  ["equations.csv", "公式"],
];

const $ = (s) => document.querySelector(s);
const el = (tag, cls, text) => { const n = document.createElement(tag); if (cls) n.className = cls; if (text != null) n.textContent = text; return n; };
const pad = (n) => String(n).padStart(2, "0");
const store = {
  get(k, d) { try { return JSON.parse(localStorage.getItem(k)) ?? d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch {} },
};

const state = {
  workspace: null,
  paper: null,        // { name, sections: [{file, title, paragraphs}] }
  reports: [],        // per section: array of paragraph reports
  cur: { s: 0, p: 0 },
  profile: null,
  ruleText: {},
  engine: null,
  claude: null,
  composed: null,
  db: { name: "phrases.csv", tables: {}, dirty: new Set(), issues: {} },
};

async function api(path, body, method) {
  const opts = body !== undefined
    ? { method: method ?? "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
    : undefined;
  const r = await fetch(path, opts);
  if (!r.ok) throw new Error((await r.text()) || `${r.status}`);
  const text = await r.text();
  return text ? JSON.parse(text) : null;
}

// ── Theme ──────────────────────────────────────────────
function applyTheme() {
  const t = store.get("pc.theme", "orange");
  const sys = matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  const m = store.get("pc.mode", sys);
  const root = document.documentElement;
  root.dataset.theme = t;
  root.dataset.mode = m;
  root.classList.toggle("dim-paper", store.get("pc.dim", false));
  document.querySelectorAll(".theme[data-theme]").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.theme === t)));
  $("#dimPaper").setAttribute("aria-pressed", String(store.get("pc.dim", false)));
  $("#mode").textContent = m === "dark" ? "☾" : "☀";
}

// ── Paper ──────────────────────────────────────────────
const sections = () => state.paper?.sections ?? [];
const curSection = () => sections()[state.cur.s];
const roleOf = (title) => ROLES.find((r) => r.toLowerCase() === title.toLowerCase())
  ?? ROLES.find((r) => title.toLowerCase().startsWith(r.toLowerCase().slice(0, 5))) ?? "Any";
const rank = (s) => ({ info: 1, warning: 2, error: 3 }[s] ?? 0);

async function openPaper(name) {
  try {
    const res = await api(`/api/papers/${encodeURIComponent(name)}`);
    state.paper = { name, sections: res.sections };
    PCTex.setMacros(res.macros ?? {});
    state.cur = { s: 0, p: 0 };
    store.set("pc.paper", name);
  } catch (e) {
    state.paper = null;
  }
  $("#paperName").textContent = state.paper?.name ?? "—";
  await checkAll();
  render();
}

async function checkSection(i) {
  const s = sections()[i];
  if (!s) return;
  const text = s.paragraphs.join("\n\n");
  try { state.reports[i] = text.trim() ? await api("/api/check", { text, section: roleOf(s.title) }) : []; }
  catch { state.reports[i] = []; }
}
async function checkAll() {
  state.reports = [];
  await Promise.all(sections().map((_, i) => checkSection(i)));
}

async function saveSection(i) {
  const s = sections()[i];
  const saved = await api(`/api/papers/${encodeURIComponent(state.paper.name)}/sections/${encodeURIComponent(s.file)}`,
    { title: s.title, paragraphs: s.paragraphs }, "PUT");
  sections()[i] = saved;
  await checkSection(i);
}

function render() {
  PCTex.numberEquations(sections().flatMap((s) => s.paragraphs));
  renderNav(); renderPaper(); renderSide(); renderStatus();
}

function renderNav() {
  $("#navTitle").textContent = state.paper?.name ?? "";
  const nav = $("#nav");
  nav.innerHTML = "";
  if (!state.paper) { nav.append(el("li", "empty", "從「檔案」開啟論文")); return; }
  const open = store.get("pc.navOpen", {});
  sections().forEach((sec, si) => {
    const li = el("li", "sec");
    li.dataset.open = String(open[sec.file] !== false);
    const head = el("button");
    head.append(el("span", "caret", "▾"), el("span", "", sec.title), el("span", "count", String(sec.paragraphs.length)));
    head.onclick = () => {
      const now = li.dataset.open !== "true";
      li.dataset.open = String(now);
      open[sec.file] = now;
      store.set("pc.navOpen", open);
      document.getElementById(`sec-${si}`)?.scrollIntoView({ block: "start", behavior: "smooth" });
    };
    const ol = el("ol", "outline");
    sec.paragraphs.forEach((text, pi) => {
      const b = el("button");
      b.setAttribute("aria-current", String(si === state.cur.s && pi === state.cur.p));
      const rep = state.reports[si]?.[pi];
      // Label: rendered first line; display equations show their number.
      const t = el("span", "t");
      const eq = text.match(/\\label\{([^}]*)\}/);
      if (/^\\begin\{(equation|align|gather|multline)/.test(text.trim())) t.textContent = `式 ${eq && PCTex.labels?.[eq[1]] ? `(${PCTex.labels[eq[1]]})` : ""}`;
      else t.append(PCTex.render(text.split("\n")[0]));
      b.append(el("span", `status ${rep ? rep.status ?? "" : ""}`), el("span", "i", pad(pi + 1)), t);
      b.onclick = () => { select(si, pi); paraEl(si, pi)?.scrollIntoView({ block: "center", behavior: "smooth" }); };
      const item = el("li");
      item.append(b);
      ol.append(item);
    });
    li.append(head, ol);
    nav.append(li);
  });
}

const paraEl = (si, pi) => document.querySelector(`.para[data-s="${si}"][data-p="${pi}"]`);

function renderPaper() {
  const paper = $("#paper");
  paper.innerHTML = "";
  if (!state.paper) return;
  sections().forEach((sec, si) => {
    const h = el("h2");
    h.append(PCTex.render(sec.title));
    h.id = `sec-${si}`;
    paper.append(h);
    sec.paragraphs.forEach((text, pi) => {
      const block = /\\begin\{(equation|align|gather|multline|eqnarray|displaymath)/.test(text);
      const p = el(block ? "div" : "p", block ? "para display" : "para");
      p.dataset.s = si;
      p.dataset.p = pi;
      if (block) {
        // Mixed prose + display math: render as a whole, without sentence marks.
        p.append(PCTex.render(text));
      } else {
        const rep = state.reports[si]?.[pi];
        const flagged = new Map();
        rep?.findings.forEach((f) => {
          if (f.sentence == null) return;
          if (rank(f.severity) > rank(flagged.get(f.sentence))) flagged.set(f.sentence, f.severity);
        });
        (rep?.sentence_text ?? [text.replace(/\s+/g, " ")]).forEach((s, i) => {
          const span = el("span", "s" + (flagged.has(i) ? ` flag ${flagged.get(i)}` : ""));
          span.append(PCTex.render(s), " ");
          p.append(span);
        });
      }
      p.onclick = () => select(si, pi);
      p.ondblclick = () => edit(p, si, pi);
      paper.append(p);
    });
    if (!sec.paragraphs.length) {
      const p = el("p", "para add-para", "雙擊新增段落");
      p.ondblclick = () => { sec.paragraphs.push(""); render(); edit(paraEl(si, 0), si, 0); };
      paper.append(p);
    }
  });
  markCurrent();
}

function select(si, pi) {
  state.cur = { s: si, p: pi };
  markCurrent();
  renderSide();
}

function markCurrent() {
  document.querySelectorAll(".para").forEach((p) =>
    p.classList.toggle("current", +p.dataset.s === state.cur.s && +p.dataset.p === state.cur.p));
  document.querySelectorAll(".nav .outline button").forEach((b) => b.setAttribute("aria-current", "false"));
  const navSec = document.querySelectorAll(".nav .sec")[state.cur.s];
  navSec?.querySelectorAll(".outline button")[state.cur.p]?.setAttribute("aria-current", "true");
}

function edit(p, si, pi) {
  if (!p) return;
  select(si, pi);
  const ta = document.createElement("textarea");
  ta.value = sections()[si].paragraphs[pi] ?? "";
  const fit = () => { ta.style.height = "auto"; ta.style.height = ta.scrollHeight + "px"; };
  ta.oninput = fit;
  ta.onkeydown = (e) => { if (e.key === "Escape" || (e.key === "Enter" && (e.metaKey || e.ctrlKey))) ta.blur(); };
  ta.onblur = async () => {
    const v = ta.value.trim();
    const list = sections()[si].paragraphs;
    if (v) list[pi] = v; else list.splice(pi, 1);
    try { await saveSection(si); } catch (e) { flash(`儲存失敗：${e.message}`); }
    render();
    refreshGit();
  };
  p.replaceChildren(ta);
  p.ondblclick = null;
  fit();
  ta.focus();
}

// ── Right panel ────────────────────────────────────────
function renderSide() { renderFindings(); syncCompose(); }

function renderFindings() {
  const ul = $("#findings");
  ul.innerHTML = "";
  const r = state.reports[state.cur.s]?.[state.cur.p];
  if (!r) { ul.append(el("li", "empty", "—")); return; }
  if (!r.findings.length && !r.pending.length) {
    const li = el("li"); li.append(el("span", "status"), el("span", "", "通過")); ul.append(li); return;
  }
  r.findings.forEach((f) => {
    const li = el("li");
    const body = el("span");
    body.append(el("span", "m", f.message), el("span", "rid", f.rule + (f.sentence != null ? ` · s${f.sentence + 1}` : "")));
    li.append(el("span", `status ${f.severity}`), body);
    ul.append(li);
  });
  r.pending.forEach((id) => {
    const li = el("li", "pending");
    const body = el("span");
    body.append(el("span", "m", state.ruleText[id] ?? id), el("span", "rid", id));
    li.append(el("span", "status info"), body);
    ul.append(li);
  });
}

function syncCompose() {
  const sel = $("#cSection");
  if (!sel.options.length) ROLES.forEach((r) => sel.append(new Option(r, r)));
  const sec = curSection();
  if (sec) sel.value = roleOf(sec.title);
}

const seg = (id) => $(`#${id} [aria-pressed="true"]`)?.dataset.v;

function slotted(template) {
  const frag = document.createDocumentFragment();
  template.split(/(\{[^}]*\})/).forEach((part) => {
    if (!part) return;
    if (part.startsWith("{")) frag.append(el("i", "", part)); else frag.append(part);
  });
  return frag;
}

async function compose() {
  const points = $("#points").value.trim();
  if (!points) { $("#points").focus(); return; }
  const list = curSection()?.paragraphs ?? [];
  const ctx = seg("cContext") === "on";
  const body = {
    points,
    section: $("#cSection").value,
    function: $("#cFunction").value || null,
    min_words: Number($("#cMin").value) || 120,
    max_words: Number($("#cMax").value) || 180,
    language: seg("cLang"),
    before: ctx ? list[state.cur.p] ?? null : null,
    after: ctx ? list[state.cur.p + 1] ?? null : null,
  };
  const btn = $("#generate");
  btn.disabled = true;
  btn.firstElementChild.textContent = "生成中";
  try { state.composed = await api("/api/compose", body); }
  catch (e) { state.composed = { error: e.message, references: [] }; }
  finally { btn.disabled = false; btn.firstElementChild.textContent = "生成"; }
  renderResult();
}

function renderResult() {
  const box = $("#result");
  box.innerHTML = "";
  const r = state.composed;
  if (!r) return;
  const wrap = el("div", "result");
  if (r.draft) {
    const meta = el("div", "meta");
    const cov = el("span"); cov.append(el("b", "", `${r.check.covered}/${r.check.total}`), " 句用句型庫");
    meta.append(cov, el("span", "", `${r.check.words} 字`));
    wrap.append(meta);
    const d = el("div", "draft");
    r.draft.sentences.forEach((s) => {
      const span = el("span", "sent");
      span.append(PCTex.render(s.text));
      const ref = r.references.find((h) => h.phrase.id === s.phrase_id);
      if (ref) span.title = ref.phrase.template;
      d.append(span, el("sup", s.phrase_id ? "" : "free", s.phrase_id ?? "自由"), " ");
    });
    wrap.append(d);
    if (r.check.problems.length) {
      const ul = el("ul", "problems");
      r.check.problems.forEach((p) => ul.append(el("li", "", p)));
      wrap.append(ul);
    }
    const text = r.draft.sentences.map((s) => s.text).join("\n");
    const acts = el("div", "actions");
    [["插入", () => placeDraft(text, "insert")], ["取代", () => placeDraft(text, "replace")], ["重來", compose],
     ["複製", () => navigator.clipboard?.writeText(text.replace(/\n/g, " "))]]
      .forEach(([label, fn]) => { const b = el("button", "", label); b.onclick = fn; acts.append(b); });
    wrap.append(acts);
  } else {
    wrap.append(el("p", "note", r.error === "no engine" ? "未連接模型：以下是會交給模型的參考句型。" : `生成失敗：${r.error}`));
    if (r.prompt) {
      const acts = el("div", "actions");
      const b = el("button", "", "複製提示詞");
      b.onclick = () => navigator.clipboard?.writeText(`${r.prompt.system}\n\n${r.prompt.user}`);
      acts.append(b);
      wrap.append(acts);
    }
  }
  if (r.references?.length) {
    const ul = el("ul", "refs");
    r.references.forEach(({ phrase }) => {
      const li = el("li"); const t = el("span"); t.append(slotted(phrase.template));
      li.append(el("span", "", phrase.id), t); ul.append(li);
    });
    if (r.draft) { const det = el("details"); det.append(el("summary", "note", `參考句型 ${r.references.length}`), ul); wrap.append(det); }
    else wrap.append(ul);
  }
  box.append(wrap);
}

async function placeDraft(text, how) {
  const sec = curSection();
  if (!sec) return;
  if (how === "replace" && sec.paragraphs.length) sec.paragraphs[state.cur.p] = text;
  else {
    const at = sec.paragraphs.length ? state.cur.p + 1 : 0;
    sec.paragraphs.splice(at, 0, text);
    state.cur.p = at;
  }
  try { await saveSection(state.cur.s); } catch (e) { flash(`儲存失敗：${e.message}`); }
  render();
  refreshGit();
}

function renderStatus() {
  const words = state.reports.flat().reduce((n, r) => n + (r?.words ?? 0), 0);
  const paras = sections().reduce((n, s) => n + s.paragraphs.length, 0);
  const sec = curSection();
  const bar = $("#statusbar");
  bar.innerHTML = "";
  bar.append(el("span", "accent", sec ? `${pad(state.cur.s + 1)} / ${sec.title}` : "—"), el("span", "", `${paras} 段 · ${words} 字`));
  const eng = el("span"); const dot = el("span", "dot");
  dot.style.background = state.engine?.connected ? "var(--ok)" : "var(--faint)";
  eng.append(dot, state.engine?.name ?? "未連接模型"); bar.append(eng);
  const cl = el("span"); const cdot = el("span", "dot");
  cdot.style.background = state.claude?.configured ? "var(--ok)" : "var(--faint)";
  cl.append(cdot, "Claude"); bar.append(cl);
  bar.append(gitIndicator());
}

// ── Git sync ───────────────────────────────────────────
function gitIndicator() {
  const g = state.git;
  const b = el("button", "sync");
  const dot = el("span", "dot");
  let label, color = "var(--faint)";
  if (state.syncing) { label = "同步中"; color = "var(--accent)"; }
  else if (!g?.repo) { label = state.workspace?.demo ? "示範（不同步）" : "無 Git"; }
  else if (g.changed.length) { label = `${g.changed.length} 項未同步`; color = "var(--warn)"; }
  else if (g.ahead) { label = `${g.ahead} 個 commit 未推送`; color = "var(--warn)"; }
  else if (!g.remote) { label = "已 commit（無遠端）"; color = "var(--ok)"; }
  else { label = state.lastSync ? `已同步 ${state.lastSync}` : "已同步"; color = "var(--ok)"; }
  dot.style.background = color;
  b.append(dot, label);
  b.title = g?.repo ? `${g.branch ?? ""} → ${g.remote ?? "沒有遠端"}\nCtrl+S 同步` : "工作區不是 Git repository";
  b.disabled = !g?.repo || state.syncing;
  b.onclick = syncNow;
  return b;
}

async function refreshGit() {
  try { state.git = await api("/api/git/status"); } catch { state.git = null; }
  renderStatus();
}

async function syncNow() {
  if (!state.git?.repo || state.syncing) return;
  // Finish any paragraph being edited first, so it is part of the commit.
  const active = document.activeElement;
  if (active?.tagName === "TEXTAREA" && active.closest(".para")) { active.blur(); await new Promise((r) => setTimeout(r, 200)); }
  if (state.db.dirty.size) await saveDb();
  state.syncing = true;
  renderStatus();
  try {
    const r = await api("/api/git/sync", {});
    state.git = r.status;
    state.lastSync = new Date().toLocaleTimeString("zh-TW", { hour: "2-digit", minute: "2-digit", hour12: false });
    if (r.pulled && state.paper) await openPaper(state.paper.name);
    flash(r.committed ? `已同步：${r.committed}` : "沒有新的修改");
  } catch (e) {
    flash(e.message, 8000);
    await refreshGit();
  } finally {
    state.syncing = false;
    renderStatus();
  }
}

let flashTimer;
function flash(msg, ms = 3000) {
  let t = $("#flash");
  if (!t) { t = el("div", "tip flash"); t.id = "flash"; document.body.append(t); }
  t.textContent = msg;
  clearTimeout(flashTimer);
  flashTimer = setTimeout(() => t.remove(), ms);
}

// ── Sheets ─────────────────────────────────────────────
const SHEETS = { actTemplate: "sheetTemplate", actDb: "sheetDb", actSettings: "sheetSettings" };

function openSheet(id) {
  $("#sheetSetup").hidden = true;
  Object.entries(SHEETS).forEach(([btn, sheet]) => {
    const on = sheet === id;
    $(`#${sheet}`).hidden = !on;
    $(`#${btn}`).setAttribute("aria-pressed", String(on));
  });
  if (id === "sheetDb") openDb();
  if (id === "sheetSettings") renderSettings();
  if (id === "sheetTemplate") renderTemplates();
}
function closeSheets() {
  if (state.db.dirty.size && !$("#sheetDb").hidden && !confirm("資料庫有未儲存的修改，仍要關閉？")) return;
  openSheet(null);
}

// file menu
function toggleFileMenu() {
  const m = $("#fileMenu");
  if (!m.hidden) { m.hidden = true; return; }
  m.innerHTML = "";
  const papers = state.workspace?.papers ?? [];
  if (!papers.length) m.append(el("div", "empty", "工作區的 papers/ 沒有論文"));
  papers.forEach((name) => {
    const b = el("button", "", name);
    b.setAttribute("aria-current", String(name === state.paper?.name));
    b.onclick = () => { m.hidden = true; openPaper(name); };
    m.append(b);
  });
  const r = $("#actFile").getBoundingClientRect();
  m.style.top = `${r.bottom + 4}px`;
  m.style.left = `${r.left}px`;
  m.hidden = false;
}

// templates
function renderTemplates() {
  const ul = $("#publishers");
  ul.innerHTML = "";
  const pubs = state.workspace?.publishers ?? [];
  if (!pubs.length) ul.append(el("li", "empty", "publishers/ 目前是空的"));
  pubs.forEach((p) => ul.append(el("li", "", p)));
}

// settings
async function renderSettings() {
  try { state.claude = await api("/api/claude"); } catch {}
  const c = state.claude;
  const st = $("#claudeState");
  st.className = c?.configured ? "ok" : "";
  st.textContent = c?.configured ? (c.source === "env" ? "已連接（環境變數）" : "已連接（系統金鑰圈）") : "未設定";
  const sel = $("#claudeModel");
  if (c && !sel.options.length) c.models.forEach((m) => sel.append(new Option(m, m)));
  if (c) sel.value = c.model;
  const e = state.engine;
  $("#engineState").textContent = e?.connected ? `已連接 ${e.name}` : "未連接";
  $("#engineState").className = e?.connected ? "ok" : "";
  $("#wsPath").textContent = (state.workspace?.root ?? "—") + (state.workspace?.demo ? "（示範）" : "");
}

async function saveClaude(body) {
  try { state.claude = await api("/api/settings/claude", body); flash("已更新 Claude 設定"); }
  catch (e) { flash(e.message); }
  $("#claudeKey").value = "";
  renderSettings();
  renderStatus();
}

// ── Database editor ────────────────────────────────────
async function openDb() {
  const tabs = $("#dbTabs");
  if (!tabs.children.length) {
    TABLES.forEach(([name, label]) => {
      const b = el("button");
      b.setAttribute("role", "tab");
      b.dataset.name = name;
      b.append(label, el("span", "c", ""));
      b.onclick = () => showTable(name);
      tabs.append(b);
    });
    const ds = $("#dbSection");
    ROLES.forEach((r) => ds.append(new Option(r, r)));
    const dc = $("#dcSection");
    ROLES.forEach((r) => dc.append(new Option(r, r)));
  }
  await Promise.all(TABLES.map(async ([name]) => {
    if (!state.db.dirty.has(name)) {
      try { state.db.tables[name] = await api(`/api/tables/${name}`); } catch { state.db.tables[name] = { headers: [], rows: [] }; }
    }
  }));
  $("#dcSection").value = roleOf(curSection()?.title ?? "Any");
  showTable(state.db.name);
}

const COL_CLASS = { id: "id", priority: "short", type: "short", kind: "short", position: "short", severity: "short", threshold: "short" };

function showTable(name) {
  state.db.name = name;
  document.querySelectorAll("#dbTabs button").forEach((b) => {
    b.setAttribute("aria-selected", String(b.dataset.name === name));
    b.querySelector(".c").textContent = state.db.tables[b.dataset.name]?.rows.length ?? "";
  });
  const t = state.db.tables[name];
  const grid = $("#dbGrid");
  grid.innerHTML = "";
  const thead = el("thead"); const hr = el("tr");
  t.headers.forEach((h) => hr.append(el("th", "", h)));
  hr.append(el("th", "", ""));
  thead.append(hr);
  const tbody = el("tbody");
  t.rows.forEach((row, ri) => tbody.append(rowEl(t, row, ri)));
  grid.append(thead, tbody);
  $("#dbSection").hidden = !t.headers.includes("section");
  $("#dbDirty").hidden = !state.db.dirty.has(name);
  const iss = state.db.issues[name] ?? [];
  $("#dbIssues").textContent = iss.map((i) => `第 ${i.line} 行 ${i.column ?? ""}：${i.message}`).join("　");
  applyFilter();
}

function rowEl(t, row, ri) {
  const tr = el("tr");
  if (row._new) tr.className = "new";
  t.headers.forEach((h, ci) => {
    const td = el("td", COL_CLASS[h] ?? "");
    const ta = el("textarea", "cell");
    ta.rows = 1;
    ta.value = row[ci] ?? "";
    ta.oninput = () => { row[ci] = ta.value; markDirty(); };
    td.append(ta);
    tr.append(td);
  });
  const td = el("td");
  const del = el("button", "del", "✕");
  del.title = "刪除這一列";
  del.onclick = () => { t.rows.splice(ri, 1); markDirty(); showTable(state.db.name); };
  td.append(del);
  tr.append(td);
  return tr;
}

function markDirty(name = state.db.name) {
  state.db.dirty.add(name);
  if (name === state.db.name) $("#dbDirty").hidden = false;
}

function applyFilter() {
  const q = $("#dbFilter").value.trim().toLowerCase();
  const sec = $("#dbSection").value;
  const t = state.db.tables[state.db.name];
  const si = t.headers.indexOf("section");
  document.querySelectorAll("#dbGrid tbody tr").forEach((tr, i) => {
    const row = t.rows[i];
    const okQ = !q || row.some((c) => String(c).toLowerCase().includes(q));
    const okS = !sec || si < 0 || row[si] === sec || row[si] === "Any";
    tr.hidden = !(okQ && okS);
  });
}

function nextId(t, prefix) {
  const ii = t.headers.indexOf("id");
  const max = Math.max(0, ...t.rows.map((r) => parseInt(String(r[ii] ?? "").replace(prefix, ""), 10)).filter((n) => !isNaN(n)));
  return `${prefix}${String(max + 1).padStart(3, "0")}`;
}
const PREFIX = { "phrases.csv": "P", "expressions.csv": "X", "rules.csv": "R", "exemplars.csv": "E", "equations.csv": "Q" };

function addRow(name, values = {}) {
  const t = state.db.tables[name];
  const row = t.headers.map(() => "");
  if (t.headers.includes("id") && PREFIX[name]) row[t.headers.indexOf("id")] = nextId(t, PREFIX[name]);
  Object.entries(values).forEach(([k, v]) => { const i = t.headers.indexOf(k); if (i >= 0) row[i] = v; });
  if (name === "phrases.csv" && !values.priority) row[t.headers.indexOf("priority")] = "3";
  row._new = true;
  t.rows.push(row);
  markDirty(name);
  return row;
}

async function saveDb() {
  const names = [...state.db.dirty];
  for (const name of names) {
    const t = state.db.tables[name];
    try {
      state.db.issues[name] = await api(`/api/tables/${name}`, { headers: t.headers, rows: t.rows.map((r) => [...r]) }, "PUT");
      state.db.dirty.delete(name);
      t.rows.forEach((r) => delete r._new);
    } catch (e) { flash(`${name} 儲存失敗：${e.message}`); }
  }
  try { state.profile = await api("/api/profile"); } catch {}
  showTable(state.db.name);
  const n = Object.values(state.db.issues).flat().length;
  flash(n ? `已儲存，有 ${n} 筆格式問題` : "已儲存");
  refreshGit();
}

// decompose
async function decompose() {
  const text = $("#dcText").value.trim();
  if (!text) { $("#dcText").focus(); return; }
  const out = $("#dcOut");
  const btn = $("#dcRun");
  btn.disabled = true; btn.textContent = "拆解中";
  out.innerHTML = "";
  try {
    const p = await api("/api/decompose", { text, section: $("#dcSection").value });
    renderProposals(p);
  } catch (e) {
    out.append(el("p", "note", e.message));
  } finally { btn.disabled = false; btn.textContent = "拆解"; }
}

function renderProposals(p) {
  const out = $("#dcOut");
  out.innerHTML = "";
  const groups = [
    ["phrases.csv", "句型", p.phrases, (x) => [`${x.section} · ${x.function} · ${x.position}`, x.template, x.example]],
    ["expressions.csv", "片語", p.expressions, (x) => [`${x.kind}${x.section ? " · " + x.section : ""}`, x.text, x.usage]],
  ];
  const picks = [];
  groups.forEach(([name, label, items, show]) => {
    if (!items.length) return;
    const head = el("div", "prop-head"); head.append(label, el("span", "c", String(items.length)));
    const ul = el("ul", "prop");
    items.forEach((x) => {
      const li = el("li");
      const cb = el("input"); cb.type = "checkbox"; cb.checked = true;
      const [meta, main, sub] = show(x);
      const body = el("div");
      const t = el("div", "t"); t.append(name === "phrases.csv" ? slotted(main) : main);
      body.append(el("div", "meta", meta), t);
      if (sub) body.append(el("div", "u", sub));
      li.append(cb, body);
      ul.append(li);
      picks.push([cb, name, x]);
    });
    out.append(head, ul);
  });
  const add = el("button", "primary small", "加入勾選項目");
  add.style.marginTop = "8px";
  add.onclick = () => {
    let n = 0;
    picks.filter(([cb]) => cb.checked).forEach(([, name, x]) => {
      if (name === "phrases.csv") addRow(name, { section: x.section, function: x.function, position: x.position, template: x.template, example: x.example, tags: (x.tags ?? []).join(";") });
      else addRow(name, { kind: x.kind, text: x.text, usage: x.usage, section: x.section, example: x.example });
      n++;
    });
    out.innerHTML = "";
    out.append(el("p", "note", `已加入 ${n} 筆，按「儲存」寫入資料庫。`));
    showTable(state.db.name);
  };
  out.append(add);
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
      tip = el("div", "tip", h.dataset.tip);
      document.body.append(tip);
      const r = h.getBoundingClientRect();
      tip.style.top = `${r.bottom + 6}px`;
      tip.style.left = `${Math.max(8, Math.min(r.left - 100, innerWidth - 260))}px`;
      h.setAttribute("aria-expanded", "true");
    };
  });
  document.addEventListener("click", (e) => {
    closeTip();
    if (!e.target.closest("#fileMenu, #actFile")) $("#fileMenu").hidden = true;
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") { closeTip(); $("#fileMenu").hidden = true; }
    // Ctrl+S: save database edits, then commit and push the workspace.
    if ((e.ctrlKey || e.metaKey) && e.key === "s") { e.preventDefault(); syncNow(); }
  });

  $("#actFile").onclick = toggleFileMenu;
  Object.entries(SHEETS).forEach(([btn, sheet]) => {
    $(`#${btn}`).onclick = () => ($(`#${sheet}`).hidden ? openSheet(sheet) : closeSheets());
  });
  document.querySelectorAll("[data-close]").forEach((b) => (b.onclick = closeSheets));

  document.querySelectorAll(".theme[data-theme]").forEach((b) => { b.onclick = () => { store.set("pc.theme", b.dataset.theme); applyTheme(); }; });
  $("#mode").onclick = () => { store.set("pc.mode", document.documentElement.dataset.mode === "dark" ? "light" : "dark"); applyTheme(); };
  $("#dimPaper").onclick = () => { store.set("pc.dim", !store.get("pc.dim", false)); applyTheme(); };

  $("#claudeSave").onclick = () => { const k = $("#claudeKey").value.trim(); if (k) saveClaude({ key: k }); };
  $("#claudeRemove").onclick = () => saveClaude({ key: "" });
  $("#claudeModel").onchange = () => saveClaude({ model: $("#claudeModel").value });

  document.querySelectorAll(".seg").forEach((s) => {
    s.querySelectorAll("button").forEach((b) => {
      b.onclick = () => s.querySelectorAll("button").forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
    });
  });
  $("#generate").onclick = compose;
  $("#points").onkeydown = (e) => { if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) compose(); };

  $("#dbFilter").oninput = applyFilter;
  $("#dbSection").onchange = applyFilter;
  $("#dbAdd").onclick = () => {
    const sec = $("#dbSection").value;
    addRow(state.db.name, sec ? { section: sec } : {});
    showTable(state.db.name);
    const rows = document.querySelectorAll("#dbGrid tbody tr");
    rows[rows.length - 1]?.querySelector(".cell:not([value])")?.focus();
  };
  $("#dbSave").onclick = saveDb;
  $("#dcRun").onclick = decompose;
}

async function boot() {
  applyTheme();
  wire();
  try { state.workspace = await api("/api/workspace"); } catch {}
  // Bring in work from other devices before showing anything.
  try {
    const r = await api("/api/git/pull", {});
    if (r?.pulled) flash("已從遠端更新");
  } catch (e) { flash(e.message, 8000); }
  await refreshGit();
  try {
    state.profile = await api("/api/profile");
    state.ruleText = await api("/api/profile/rules");
    const chips = $("#profileChips");
    chips.innerHTML = "";
    const name = el("span", "chip", state.profile.error ? "資料庫讀取失敗" : state.profile.name);
    name.title = state.profile.error ?? state.profile.path;
    chips.append(name);
    if (state.profile.demo) { const d = el("span", "chip warn", "示範資料"); d.title = "尚未指定你的私人工作區"; chips.append(d); }
    if (state.profile.issues.length) {
      const i = el("span", "chip warn", `${state.profile.issues.length} 筆格式問題`);
      i.title = state.profile.issues.map((x) => `${x.file}:${x.line} ${x.column ?? ""} ${x.message}`).join("\n");
      chips.append(i);
    }
    state.profile.functions.forEach((f) => $("#cFunction").append(new Option(f.replace(/_/g, " "), f)));
  } catch {
    $("#profileChips").innerHTML = '<span class="chip warn">伺服器未啟動</span>';
  }
  const papers = state.workspace?.papers ?? [];
  const last = store.get("pc.paper", null);
  if (papers.length) await openPaper(papers.includes(last) ? last : papers[0]);
  else render();
  try { state.engine = await api("/api/engine"); } catch {}
  try { state.claude = await api("/api/claude"); } catch {}
  renderStatus();
  // setup.js loads after this file; by now it is defined.
  Setup.wire();
  await Setup.maybeOpen();
  // A local model may still be starting in the background.
  setTimeout(async () => { try { state.engine = await api("/api/engine"); renderStatus(); } catch {} }, 15000);
}

boot();
