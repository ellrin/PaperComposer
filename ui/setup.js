// Setup wizard: workspace, Git, local model, Claude.
// Every user points the app at their own private workspace; nothing here is
// tied to a particular account or repository.

const Setup = (() => {
  let st = null;
  let step = 0;
  let mode = "open";
  let selected = null;
  let poll = null;
  let changed = false; // workspace changed: reload the app on close

  const gb = (bytes) => `${(bytes / 1024 ** 3).toFixed(bytes > 1024 ** 3 * 10 ? 0 : 1)} GB`;
  const mb = (bytes) => bytes < 1024 ** 3 ? `${Math.round(bytes / 1024 ** 2)} MB` : gb(bytes);

  async function refresh() {
    try { st = await api("/api/setup/status"); } catch (e) { flash(e.message); return; }
    render();
  }

  function open(s = 0) {
    document.querySelectorAll(".sheet").forEach((x) => (x.hidden = true));
    $("#sheetSetup").hidden = false;
    step = s;
    refresh();
  }

  function close() {
    clearInterval(poll);
    $("#sheetSetup").hidden = true;
    store.set("pc.setupSeen", true);
    if (changed) location.reload();
  }

  const done = () => [
    !!st && !st.workspace.demo && st.workspace.has_profile,
    !!st && st.git_tool.installed && !!st.git_tool.name && !!st.git_tool.email,
    !!st && (!!st.running || st.models.some((m) => m.installed)),
    !!st && st.claude.configured,
  ];

  function render() {
    const d = done();
    document.querySelectorAll("#wzTabs button").forEach((b, i) => {
      b.setAttribute("aria-selected", String(i === step));
      b.querySelector(".st").textContent = d[i] ? "✓" : "";
      b.onclick = () => { step = i; render(); };
    });
    document.querySelectorAll(".wizard .step").forEach((s) => (s.hidden = +s.dataset.step !== step));
    $("#wzPrev").disabled = step === 0;
    $("#wzNext").textContent = step === 3 ? "完成" : "下一步";
    [renderWorkspace, renderGit, renderModels, renderClaude][step]();
  }

  // ── 01 workspace ──
  function check(ok, text) { const li = el("li"); li.append(el("span", `status ${ok ? "" : "warning"}`), el("span", "", text)); return li; }

  function renderWorkspace() {
    const w = st.workspace;
    const now = $("#wzWsNow");
    now.innerHTML = "";
    if (w.demo) now.append(el("p", "note", "目前使用示範資料。選擇你自己的私人工作區："));
    else {
      now.append(el("code", "", w.root));
      const ul = el("ul", "checks");
      ul.append(check(w.has_profile, "profile/"));
      ul.append(check(!!w.git?.repo, w.git?.repo ? `Git（${w.git.branch}）` : "不是 Git repository"));
      if (w.git?.repo) ul.append(check(!!w.git.remote, w.git.remote ? `遠端 ${w.git.remote}` : "沒有遠端，無法跨裝置同步"));
      ul.append(check(true, `${w.papers} 篇論文`));
      now.append(ul);
    }
    document.querySelectorAll("#wzWsMode button").forEach((b) => {
      b.setAttribute("aria-pressed", String(b.dataset.v === mode));
      b.onclick = () => { mode = b.dataset.v; renderWorkspace(); };
    });
    document.querySelectorAll('.step[data-step="0"] .form').forEach((f) => (f.hidden = f.dataset.mode !== mode));
  }

  async function visibility(input, out) {
    out.textContent = "";
    out.className = "vis";
    const url = input.value.trim();
    if (!url) return;
    try {
      const r = await api(`/api/setup/visibility?url=${encodeURIComponent(url)}`);
      if (r.host !== "github" || r.public == null) return;
      out.textContent = r.public ? "公開 repo：請先改成 private" : "private";
      out.className = r.public ? "vis bad" : "vis ok";
    } catch {}
  }

  async function workspaceAction(path, body, btn) {
    btn.disabled = true;
    try {
      const r = await api(path, body);
      changed = true;
      flash(r.note ?? `已使用 ${r.root}${r.added ? `，補上 ${r.added} 個檔案` : ""}`, r.note ? 8000 : 3000);
      await refresh();
    } catch (e) { flash(e.message, 8000); }
    finally { btn.disabled = false; }
  }

  // ── 02 git ──
  function renderGit() {
    const g = st.git_tool;
    const now = $("#wzGitNow");
    now.innerHTML = "";
    const ul = el("ul", "checks");
    ul.append(check(g.installed, g.installed ? g.version : "找不到 Git：請先安裝 Git（Windows 請裝 Git for Windows），再重新開啟 App"));
    if (g.installed) ul.append(check(!!(g.name && g.email), g.name && g.email ? `作者 ${g.name} <${g.email}>` : "尚未設定作者"));
    now.append(ul);
    $('.step[data-step="1"] .form').hidden = !g.installed;
    if (!$("#wzGitName").value) $("#wzGitName").value = g.name ?? "";
    if (!$("#wzGitEmail").value) $("#wzGitEmail").value = g.email ?? "";
  }

  // ── 03 local model ──
  function renderModels() {
    const s = st.system;
    const sys = $("#wzSys");
    sys.innerHTML = "";
    const parts = [`記憶體 ${Math.round(s.memory_gb)} GB${s.unified_memory ? "（統一記憶體）" : ""}`];
    if (s.gpu) parts.push(`${s.gpu}${s.gpu_memory_gb ? ` ${Math.round(s.gpu_memory_gb)} GB` : ""}`);
    sys.append(el("p", "note", parts.join(" · ")));

    if (!selected) selected = st.running ?? st.models.find((m) => m.installed)?.entry.id ?? st.recommended;
    const ul = $("#wzModels");
    ul.innerHTML = "";
    st.models.forEach(({ entry: m, fits, installed }) => {
      const li = el("li", "model" + (m.id === selected ? " on" : "") + (fits ? "" : " dim"));
      const head = el("div", "mh");
      head.append(el("b", "", `${m.name} ${m.variant}`), el("span", "sum", m.summary));
      const tags = el("div", "tags");
      if (m.id === st.recommended) tags.append(el("span", "tag acc", "推薦"));
      if (st.running === m.id) tags.append(el("span", "tag ok", "使用中"));
      else if (installed) tags.append(el("span", "tag", "已下載"));
      if (!fits) tags.append(el("span", "tag bad", `需 ${m.min_memory_gb} GB`));
      head.append(tags);
      const meta = el("div", "meta", `約 ${+m.approx_gb.toFixed(1)} GB · 至少 ${m.min_memory_gb} GB 記憶體 · ${m.license}${m.vision ? " · 可看圖" : ""}`);
      li.append(head, meta);
      li.onclick = () => { selected = m.id; $("#wzPlan").innerHTML = ""; renderModels(); };
      ul.append(li);
    });
    if (!st.models.some((m) => m.fits)) ul.append(el("li", "note", "這台電腦的記憶體不足以執行本機模型，建議只使用 Claude。"));
    renderModelActions();
  }

  function renderModelActions() {
    const box = $("#wzPlan");
    if (box.dataset.busy === "1") return;
    box.innerHTML = "";
    const m = st.models.find((x) => x.entry.id === selected);
    if (!m) return;
    const acts = el("div", "actions-row");
    if (st.running === selected) {
      const b = el("button", "ghost", "停止");
      b.onclick = async () => { await api("/api/models/stop", {}); refresh(); };
      acts.append(b);
    } else if (m.installed) {
      const b = el("button", "primary small", "啟用");
      b.onclick = async () => {
        b.disabled = true; b.textContent = "啟動中…";
        try { await api("/api/models/start", { id: selected }); flash("模型已啟用"); } catch (e) { flash(e.message, 8000); }
        refresh();
      };
      acts.append(b);
    } else {
      const b = el("button", "primary small", "準備下載");
      b.disabled = !m.fits;
      b.onclick = () => preparePlan(b);
      acts.append(b);
    }
    box.append(acts);
  }

  async function preparePlan(btn) {
    btn.disabled = true; btn.textContent = "查詢檔案中…";
    let plan;
    try { plan = await api("/api/models/plan", { id: selected }); }
    catch (e) { flash(e.message, 8000); btn.disabled = false; btn.textContent = "準備下載"; return; }
    const box = $("#wzPlan");
    box.innerHTML = "";
    const t = el("table", "plan");
    plan.files.forEach((f) => {
      const tr = el("tr");
      tr.append(el("td", "k", { model: "模型", mmproj: "看圖", runtime: "llama.cpp" }[f.kind] ?? f.kind), el("td", "n", f.name), el("td", "s", f.installed ? "已有" : mb(f.size)), el("td", "src", f.source));
      t.append(tr);
    });
    box.append(t);
    const acts = el("div", "actions-row");
    const go = el("button", "primary small", `確認下載 ${mb(plan.total_to_download)}`);
    go.onclick = () => install();
    const cancel = el("button", "ghost", "取消");
    cancel.onclick = () => renderModelActions();
    acts.append(cancel, go);
    box.append(acts);
  }

  async function install() {
    try { await api("/api/models/install", { id: selected }); }
    catch (e) { flash(e.message, 8000); return; }
    const box = $("#wzPlan");
    box.dataset.busy = "1";
    clearInterval(poll);
    poll = setInterval(async () => {
      let p;
      try { p = await api("/api/models/progress"); } catch { return; }
      box.innerHTML = "";
      p.files.sort((a, b) => a.name.localeCompare(b.name)).forEach((f) => {
        const row = el("div", "prog");
        const pct = f.total ? Math.min(100, (f.done / f.total) * 100) : 0;
        const bar = el("div", "bar"); const fill = el("i"); fill.style.width = `${pct}%`; bar.append(fill);
        row.append(el("span", "n", f.name), bar, el("span", "s", f.state === "error" ? f.error : `${mb(f.done)} / ${mb(f.total)}`));
        box.append(row);
      });
      const phase = p.phase;
      box.append(el("p", "note", phase === "starting" ? "下載完成，正在啟動模型…" : phase === "ready" ? "已啟用" : phase.startsWith("error") ? phase.slice(7) : "下載中…"));
      if (phase === "ready" || phase.startsWith("error")) {
        clearInterval(poll);
        box.dataset.busy = "0";
        await refresh();
        if (phase === "ready") {
          flash("本機模型已啟用");
          try { state.engine = await api("/api/engine"); renderStatus(); } catch (e) { console.warn(e); }
        }
      }
    }, 1000);
  }

  // ── 04 claude ──
  function renderClaude() {
    const c = st.claude;
    const now = $("#wzClaudeNow");
    now.innerHTML = "";
    const ul = el("ul", "checks");
    ul.append(check(c.configured, c.configured ? (c.source === "env" ? "已連接（環境變數）" : "已連接（系統金鑰圈）") : "尚未設定，可略過"));
    now.append(ul);
    const sel = $("#wzClaudeModel");
    if (!sel.options.length) c.models.forEach((m) => sel.append(new Option(m, m)));
    sel.value = c.model;
  }

  function wire() {
    $("#wzPrev").onclick = () => { if (step > 0) { step--; render(); } };
    $("#wzNext").onclick = () => { if (step < 3) { step++; render(); } else close(); };
    $("#wzSkip").onclick = () => { if (step < 3) { step++; render(); } else close(); };
    $("#sheetSetup [data-close]").onclick = close;
    $("#openSetup").onclick = () => open(0);

    $("#wzOpen").onclick = (e) => workspaceAction("/api/setup/workspace", { path: $("#wzOpenPath").value, init: $("#wzOpenInit").checked }, e.target);
    $("#wzClone").onclick = (e) => workspaceAction("/api/setup/clone", { url: $("#wzCloneUrl").value, parent: $("#wzCloneParent").value }, e.target);
    $("#wzNew").onclick = (e) => workspaceAction("/api/setup/new", { path: $("#wzNewPath").value, remote: $("#wzNewRemote").value || null }, e.target);
    $("#wzCloneUrl").onchange = () => visibility($("#wzCloneUrl"), $("#wzCloneVis"));
    $("#wzNewRemote").onchange = () => visibility($("#wzNewRemote"), $("#wzNewVis"));

    $("#wzGitSave").onclick = async () => {
      try { await api("/api/setup/git-identity", { name: $("#wzGitName").value, email: $("#wzGitEmail").value }); flash("已設定 Git 作者"); }
      catch (e) { flash(e.message, 6000); }
      refresh();
    };
    $("#wzClaudeSave").onclick = async () => {
      const body = { model: $("#wzClaudeModel").value };
      const k = $("#wzClaudeKey").value.trim();
      if (k) body.key = k;
      try { state.claude = await api("/api/settings/claude", body); flash("已更新 Claude 設定"); } catch (e) { flash(e.message, 6000); }
      $("#wzClaudeKey").value = "";
      renderStatus();
      refresh();
    };
  }

  /** First run: no workspace configured yet. */
  async function maybeOpen() {
    try {
      const s = await api("/api/setup/status");
      if (s.first_run && !store.get("pc.setupSeen", false)) { st = s; open(0); }
    } catch {}
  }

  return { open, wire, maybeOpen };
})();
