const paperFields = [
  "概述",
  "心得",
  "核心貢獻",
  "關鍵內容",
  "可放入 Introduction 的角度",
  "可放入 Related Work 的角度",
  "方法摘要",
  "資料集摘要",
  "結果摘要",
  "限制",
  "與本研究的關係",
  "可引用句子",
  "英文",
  "cite",
];

const algorithmFields = ["公式", "描述", "公式與描述", "輸入", "輸出", "使用原因", "限制", "與本研究的關係"];
const metadataFieldOptions = [
  { value: "short_name", label: "簡稱" },
  { value: "file_name", label: "檔名" },
  { value: "name", label: "素材名稱" },
  { value: "year", label: "年份" },
  { value: "kind", label: "素材類型" },
];

function fieldOptionsFor(sectionType) {
  const content = sectionType === "Method" ? algorithmFields : paperFields;
  return [
    ...metadataFieldOptions,
    ...content.map((f) => ({ value: f, label: f })),
  ];
}

function getFieldValue(material, field) {
  if (!material) return "";
  if (field === "short_name") return material.metadata?.short_name ?? material.name ?? "";
  if (field === "file_name") return material.metadata?.file_name ?? material.file_path ?? "";
  if (field === "name") return material.name ?? "";
  if (field === "year") return material.metadata?.year ?? "";
  if (field === "kind") return material.kind ?? "";
  return material.fields?.[field] ?? "";
}

function setFieldValue(material, field, value) {
  if (!material) return;
  material.metadata = material.metadata || {};
  material.fields = material.fields || {};
  if (field === "short_name") {
    material.metadata.short_name = value;
    material.name = value;
  } else if (field === "file_name") {
    material.metadata.file_name = value;
  } else if (field === "name") {
    material.name = value;
    if (!material.metadata.short_name) material.metadata.short_name = value;
  } else if (field === "year") {
    material.metadata.year = value;
  } else if (field === "kind") {
    material.kind = value;
  } else {
    material.fields[field] = value;
  }
}
const sectionTemplates = {
  "Abstract": { type: "GeneralNote", field: "概述" },
  "Introduction": { type: "Literature", field: "可放入 Introduction 的角度" },
  "Related works": { type: "Literature", field: "可放入 Related Work 的角度" },
  "Method": { type: "Method", field: "公式與描述" },
  "Results": { type: "Results", field: "結果摘要" },
  "Discussion": { type: "Discussion", field: "與本研究的關係" },
  "Others": { type: "Mixed", field: "概述" },
};
const bridgeUrl = "http://127.0.0.1:53683";

const colorDefaults = {
  sidebar: "#ede1bd",
  header: "#f4ead0",
  main: "#fbf7ed",
  grid: "#46321e",
  gridOpacity: 0.18,
  gridShape: "square",
  gridSize: 28,
  cardLeft: "#f3ead2",
  cardRight: "#fffdf5",
  cardDivider: "#a89766",
  accent: "#7c3aed",
  addBtn: "#7c3aed",
  sidebarText: "#1f2937",
  headerText: "#1f2937",
  mainText: "#1f2937",
  cardLeftText: "#1f2937",
  cardRightText: "#1f2937",
};

const colorPresets = [
  { key: "paper", label: "Paper Classic", colors: {
    sidebar: "#ede1bd", header: "#f4ead0", main: "#fbf7ed",
    cardLeft: "#f3ead2", cardRight: "#fffdf5", cardDivider: "#a89766",
    accent: "#7c3aed", addBtn: "#7c3aed",
    grid: "#46321e", gridOpacity: 0.18, gridShape: "square", gridSize: 28,
    sidebarText: "#1f2937", headerText: "#1f2937", mainText: "#1f2937",
    cardLeftText: "#1f2937", cardRightText: "#1f2937",
  } },
  { key: "sunset", label: "Sunset Beach", colors: {
    sidebar: "#f8b489", header: "#fcd6b6", main: "#fff1de",
    cardLeft: "#fbd2a8", cardRight: "#fff8ec", cardDivider: "#ea580c",
    accent: "#dc2626", addBtn: "#fb7185",
    grid: "#9a3412", gridOpacity: 0.12, gridShape: "dots", gridSize: 24,
    sidebarText: "#451a03", headerText: "#7c2d12", mainText: "#451a03",
    cardLeftText: "#7c2d12", cardRightText: "#451a03",
  } },
  { key: "forest", label: "Wood & Forest", colors: {
    sidebar: "#bdb38a", header: "#d8cea4", main: "#f0ead2",
    cardLeft: "#dfd6a9", cardRight: "#f7f3df", cardDivider: "#6b5d2e",
    accent: "#15803d", addBtn: "#65a30d",
    grid: "#3f3014", gridOpacity: 0.18, gridShape: "hexagon", gridSize: 30,
    sidebarText: "#1c2510", headerText: "#1c2510", mainText: "#1c2510",
    cardLeftText: "#1c2510", cardRightText: "#1c2510",
  } },
  { key: "lavender", label: "Soft Lavender", colors: {
    sidebar: "#d4c3ef", header: "#e7dcf5", main: "#f3eefc",
    cardLeft: "#ddcdf3", cardRight: "#fbf8ff", cardDivider: "#7c3aed",
    accent: "#6d28d9", addBtn: "#6366f1",
    grid: "#3b0764", gridOpacity: 0.12, gridShape: "square", gridSize: 30,
    sidebarText: "#3b0764", headerText: "#3b0764", mainText: "#1e1b4b",
    cardLeftText: "#1e1b4b", cardRightText: "#1e1b4b",
  } },
  { key: "mist", label: "Morning Mist", colors: {
    sidebar: "#a8b8c8", header: "#cbd6e2", main: "#e9eff5",
    cardLeft: "#c4d2e0", cardRight: "#f7fafc", cardDivider: "#475569",
    accent: "#0284c7", addBtn: "#06b6d4",
    grid: "#1e293b", gridOpacity: 0.14, gridShape: "triangle", gridSize: 30,
    sidebarText: "#0f172a", headerText: "#0f172a", mainText: "#0f172a",
    cardLeftText: "#0f172a", cardRightText: "#0f172a",
  } },
  { key: "cityNight", label: "City Night", colors: {
    sidebar: "#050a17", header: "#172238", main: "#020617",
    cardLeft: "#1e293b", cardRight: "#0a1424", cardDivider: "#38bdf8",
    accent: "#f472b6", addBtn: "#22d3ee",
    grid: "#38bdf8", gridOpacity: 0.10, gridShape: "square", gridSize: 30,
    sidebarText: "#e2e8f0", headerText: "#e2e8f0", mainText: "#f1f5f9",
    cardLeftText: "#f1f5f9", cardRightText: "#f1f5f9",
  } },
  { key: "cyberpunk", label: "Cyber Punk", colors: {
    sidebar: "#260840", header: "#0c1430", main: "#06030e",
    cardLeft: "#2a0e4a", cardRight: "#081226", cardDivider: "#f0abfc",
    accent: "#fde047", addBtn: "#ec4899",
    grid: "#e879f9", gridOpacity: 0.10, gridShape: "square", gridSize: 32,
    sidebarText: "#f0abfc", headerText: "#67e8f9", mainText: "#fde047",
    cardLeftText: "#f5d0fe", cardRightText: "#a5f3fc",
  } },
  { key: "midnightForest", label: "Midnight Forest", colors: {
    sidebar: "#08160e", header: "#15301f", main: "#040d08",
    cardLeft: "#1c3d2c", cardRight: "#0c1f15", cardDivider: "#5b8a73",
    accent: "#84cc16", addBtn: "#ca8a04",
    grid: "#4ade80", gridOpacity: 0.08, gridShape: "hexagon", gridSize: 32,
    sidebarText: "#dcfce7", headerText: "#dcfce7", mainText: "#ecfccb",
    cardLeftText: "#ecfccb", cardRightText: "#dcfce7",
  } },
  { key: "carbon", label: "Carbon Studio", colors: {
    sidebar: "#374151", header: "#1f2937", main: "#0a0e16",
    cardLeft: "#374151", cardRight: "#111827", cardDivider: "#9ca3af",
    accent: "#f97316", addBtn: "#f97316",
    grid: "#6b7280", gridOpacity: 0.10, gridShape: "square", gridSize: 28,
    sidebarText: "#f3f4f6", headerText: "#f3f4f6", mainText: "#f9fafb",
    cardLeftText: "#f9fafb", cardRightText: "#f9fafb",
  } },
  { key: "deepOcean", label: "Deep Ocean", colors: {
    sidebar: "#08274d", header: "#0c365f", main: "#020a18",
    cardLeft: "#0e3d68", cardRight: "#061f3a", cardDivider: "#2dd4bf",
    accent: "#5eead4", addBtn: "#06b6d4",
    grid: "#0891b2", gridOpacity: 0.12, gridShape: "triangle", gridSize: 32,
    sidebarText: "#cffafe", headerText: "#cffafe", mainText: "#e0f2fe",
    cardLeftText: "#e0f2fe", cardRightText: "#cffafe",
  } },
];

function hexToRgba(hex, alpha) {
  const m = String(hex || "").replace("#", "").match(/^([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i);
  if (!m) return `rgba(0,0,0,${alpha})`;
  const [r, g, b] = [m[1], m[2], m[3]].map((h) => parseInt(h, 16));
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function buildGridImage(shape, size, colorRgba) {
  if (shape === "dots") {
    return `radial-gradient(circle at 1px 1px, ${colorRgba} 1px, transparent 0)`;
  }
  if (shape === "triangle") {
    const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='${size}' height='${size}'><polygon points='${size / 2},2 2,${size - 2} ${size - 2},${size - 2}' fill='none' stroke='${colorRgba}' stroke-width='1'/></svg>`;
    return `url("data:image/svg+xml;utf8,${encodeURIComponent(svg)}")`;
  }
  if (shape === "hexagon") {
    const w = size;
    const h = size * 0.866;
    const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='${w * 1.5}' height='${h * 2}'><path d='M ${w * 0.5} 1 L ${w * 1.25} ${h * 0.5} L ${w * 1.25} ${h * 1.5} L ${w * 0.5} ${h * 2 - 1} L ${w * 0.25} ${h * 1.5} L ${w * 0.25} ${h * 0.5} Z' fill='none' stroke='${colorRgba}' stroke-width='1'/></svg>`;
    return `url("data:image/svg+xml;utf8,${encodeURIComponent(svg)}")`;
  }
  return `linear-gradient(90deg, ${colorRgba} 1px, transparent 1px), linear-gradient(0deg, ${colorRgba} 1px, transparent 1px)`;
}

function applyColors() {
  const c = state.colors;
  const r = document.documentElement.style;
  const gridRgba = hexToRgba(c.grid, c.gridOpacity);
  r.setProperty("--c-sidebar-bg", c.sidebar);
  r.setProperty("--c-header-bg", c.header);
  r.setProperty("--c-main-bg", c.main);
  r.setProperty("--c-grid-color", gridRgba);
  r.setProperty("--c-grid-size", `${c.gridSize}px`);
  r.setProperty("--c-grid-image", buildGridImage(c.gridShape, c.gridSize, gridRgba));
  r.setProperty("--c-card-left-bg", c.cardLeft);
  r.setProperty("--c-card-right-bg", c.cardRight);
  r.setProperty("--c-card-divider", c.cardDivider);
  r.setProperty("--c-accent", c.accent);
  r.setProperty("--c-add-btn", c.addBtn);
  r.setProperty("--c-sidebar-text", c.sidebarText);
  r.setProperty("--c-header-text", c.headerText);
  r.setProperty("--c-main-text", c.mainText);
  r.setProperty("--c-card-left-text", c.cardLeftText);
  r.setProperty("--c-card-right-text", c.cardRightText);
}

function applyPreset(key) {
  const preset = colorPresets.find((p) => p.key === key);
  if (!preset) return;
  state.colors = { ...colorDefaults, ...preset.colors };
  applyColors();
  syncColorDialogInputs();
}

function renderPresetChips() {
  const wrap = document.getElementById("presetChips");
  if (!wrap) return;
  wrap.innerHTML = colorPresets.map((p) => {
    const sw = p.colors;
    return `<button type="button" class="preset-chip" data-preset-key="${p.key}" title="${escapeHtml(p.label)}">
      <span class="chip-swatch">
        <i style="background:${sw.sidebar}"></i><i style="background:${sw.main}"></i><i style="background:${sw.accent}"></i>
      </span>
      <span class="chip-label">${escapeHtml(p.label)}</span>
    </button>`;
  }).join("");
  wrap.querySelectorAll("[data-preset-key]").forEach((btn) => {
    btn.onclick = () => applyPreset(btn.dataset.presetKey);
  });
}

async function loadColors() {
  let loaded = null;
  try {
    const data = await bridgeFetch("/colors");
    if (data && typeof data === "object") loaded = data;
  } catch (_) {
    try {
      const raw = localStorage.getItem("paperComposer.colors");
      if (raw) loaded = JSON.parse(raw);
    } catch (_) {}
  }
  state.colors = { ...colorDefaults, ...(loaded || {}) };
  applyColors();
}

async function saveColors() {
  try {
    await fetch(`${bridgeUrl}/colors`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(state.colors),
    });
  } catch (_) {}
  try {
    localStorage.setItem("paperComposer.colors", JSON.stringify(state.colors));
  } catch (_) {}
}

const state = {
  projectsList: [],       // [{project_id, project_name, folder_id}]
  projectsData: {},       // {project_id: {project, papers, algorithms, ...}}
  activeProjectId: null,
  sectionIndex: 0,
  driveConnected: false,
  driveMessage: "尚未連線",
  storedSecret: false,
  driveIndex: { pdfs: [], jsons: [] },
  dirty: false,
  dirtyBlocks: new Set(),
  syncing: false,
  workspaceRoot: "",
  colors: { ...colorDefaults },
  selectedBlockId: null,
  syncStatus: "idle",
  syncLabel: "就緒",
  autosyncTimer: null,
};

function selectBlock(blockId) {
  if (!blockId) return;
  state.selectedBlockId = blockId;
  document.querySelectorAll(".material-card.selected").forEach((c) => c.classList.remove("selected"));
  const target = document.querySelector(`.material-card [data-block-id="${blockId}"]`)?.closest(".material-card");
  if (target) target.classList.add("selected");
}

function markBlockDirty(blockId) {
  if (!blockId) return;
  state.dirtyBlocks.add(blockId);
  const btn = document.querySelector(`[data-save-block="${blockId}"]`);
  if (btn) btn.classList.add("dirty");
}

function clearBlockDirty(blockId) {
  if (!blockId) {
    state.dirtyBlocks.clear();
    document.querySelectorAll("[data-save-block]").forEach((n) => n.classList.remove("dirty"));
    return;
  }
  state.dirtyBlocks.delete(blockId);
  const btn = document.querySelector(`[data-save-block="${blockId}"]`);
  if (btn) btn.classList.remove("dirty");
}

function currentProjectData() {
  return state.projectsData[state.activeProjectId] || null;
}

const $ = (id) => document.getElementById(id);

function project() {
  const data = currentProjectData();
  return data ? data.project : null;
}

function section() {
  const proj = project();
  return proj && proj.sections ? proj.sections[state.sectionIndex] : null;
}

function materials(kind) {
  const data = currentProjectData();
  if (!data) return [];
  const key = kind.toLowerCase() + "s";
  return data[key] ?? [];
}

function render() {
  renderProjects();
  renderSections();
  renderControls();
  renderPdfSources();
  renderBlocks();
  renderDriveStatus();
}

function renderDriveStatus() {
  const dot = $("driveStatus")?.querySelector(".sync-dot");
  if (dot) dot.setAttribute("data-state", state.syncStatus || "idle");
  const label = $("syncLabel");
  if (label) label.textContent = state.syncLabel || state.driveMessage || "就緒";
  if ($("driveStatus")) $("driveStatus").title = state.syncLabel || state.driveMessage || "";
  const info = $("driveInfo");
  if (info) info.textContent = state.driveMessage;
}

function setSyncStatus(status, label) {
  state.syncStatus = status;
  if (label !== undefined) state.syncLabel = label;
  renderDriveStatus();
}

function renderProjects() {
  const activeIndex = state.projectsList.findIndex(p => p.project_id === state.activeProjectId);
  $("projectList").innerHTML = projectListHtml(state.projectsList, activeIndex);
  document.querySelectorAll("[data-project]").forEach((node) => {
    node.onclick = () => {
      const index = Number(node.dataset.project);
      if (index >= 0 && index < state.projectsList.length) {
        state.activeProjectId = state.projectsList[index].project_id;
        state.sectionIndex = 0;
        render();
        syncDriveFolderIndex({ silent: false, noAlert: true });
      }
    };
  });
  document.querySelectorAll("[data-delete-project]").forEach((node) => {
    node.onclick = (event) => {
      event.stopPropagation();
      event.preventDefault();
      const index = Number(node.dataset.deleteProject);
      if (index >= 0 && index < state.projectsList.length) removeProjectAt(index);
    };
  });
}

function projectListHtml(entries, active) {
  return entries
    .map((entry, index) => `
      <div class="project-slot">
        <button class="project-icon ${index === active ? "active" : ""}" data-project="${index}" title="${escapeHtml(entry.project_name)}">
          <span class="pi-badge">${escapeHtml(projectInitial(entry.project_name, index))}</span>
          <span class="pi-name">${escapeHtml(entry.project_name || `Project ${index + 1}`)}</span>
        </button>
        <button class="project-delete" data-delete-project="${index}" title="刪除專案" aria-label="刪除專案">×</button>
      </div>
    `)
    .join("");
}

function renderSections() {
  const proj = project();
  const sections = proj && proj.sections ? proj.sections : [];
  normalizeOrder();
  $("sectionList").innerHTML = sections
    .map((item, index) => `
      <div class="tab-wrap">
        <button class="tab ${index === state.sectionIndex ? "active" : ""}" data-section="${index}" draggable="true" title="拖曳調整順序，點一下切換，雙擊改名">
          <span>${escapeHtml(item.title)}</span>
        </button>
      </div>
    `)
    .join("");
  document.querySelectorAll("[data-section]").forEach((node) => {
    node.onclick = () => {
      state.sectionIndex = Number(node.dataset.section);
      render();
    };
    node.ondblclick = (event) => {
      event.preventDefault();
      renameSection(Number(node.dataset.section));
    };
  });
  bindSectionDrag(sections);
}

function projectInitial(name, index) {
  const trimmed = String(name || "").trim();
  if (!trimmed) return `P${index + 1}`;
  const ascii = trimmed.match(/[A-Za-z0-9]/);
  if (ascii) return ascii[0].toUpperCase();
  return trimmed.slice(0, 1);
}

function renderControls() {
  const sec = section();
  const opts = fieldOptionsFor(sec?.section_type || "Literature");
  const optionsHtml = opts.map((o) => `<option value="${escapeHtml(o.value)}">${escapeHtml(o.label)}</option>`).join("");
  if (!sec) {
    $("sectionTitle").textContent = project() ? project().project_name : "";
    $("sectionTitle").contentEditable = "false";
    $("sectionSubtitle").textContent = "";
    $("leftDisplayField").innerHTML = optionsHtml;
    $("leftDisplayField").value = "file_name";
    $("displayField").innerHTML = optionsHtml;
    $("displayField").value = paperFields[0];
    $("removeSection").hidden = true;
    $("exportMarkdown").hidden = true;
    setMaterialToolbarEnabled(false);
    return;
  }
  $("removeSection").hidden = false;
  $("exportMarkdown").hidden = false;
  $("sectionTitle").textContent = sec.title;
  $("sectionTitle").contentEditable = "true";
  $("sectionTitle").title = "可直接改章節名稱";
  $("sectionSubtitle").textContent = sec.blocks.length
    ? "點擊卡片內文字即可直接編輯。"
    : "同步 Drive 或按 + 新增素材。";
  const validValues = new Set(opts.map((o) => o.value));
  $("leftDisplayField").innerHTML = optionsHtml;
  $("leftDisplayField").value = validValues.has(sec.left_display_field) ? sec.left_display_field : "file_name";
  $("displayField").innerHTML = optionsHtml;
  const fallbackRight = sec.section_type === "Method" ? algorithmFields[0] : paperFields[0];
  $("displayField").value = validValues.has(sec.display_field) ? sec.display_field : fallbackRight;
  setMaterialToolbarEnabled(true);
}

function setMaterialToolbarEnabled(enabled) {
  ["materialKind", "pdfSource", "addMaterial"].forEach((id) => {
    const node = $(id);
    if (node) node.disabled = !enabled;
  });
  document.querySelector(".material-toolbar")?.classList.toggle("locked", !enabled);
}

function renderPdfSources() {
  const select = $("pdfSource");
  if (!select) return;
  const kind = $("materialKind")?.value || "Paper";
  if (kind !== "Paper") {
    select.innerHTML = `<option value="">${kind} 不需要選 PDF</option>`;
    select.disabled = true;
    return;
  }
  select.disabled = !section();
  const pdfs = state.driveIndex.pdfs || [];
  if (!pdfs.length) {
    select.innerHTML = `<option value="">尚未同步 Drive PDF</option>`;
    return;
  }
  select.innerHTML = [
    `<option value="">選擇 PDF 建立 Paper</option>`,
    ...pdfs.map((pdf) => `<option value="${escapeHtml(pdf.id)}">${escapeHtml(truncateName(pdf.name, 58))}</option>`),
  ].join("");
}

function renderBlocks() {
  const sec = section();
  if (!sec) {
    $("blockList").innerHTML = project()
      ? `<section class="empty-hint"><h3>還沒有段落</h3><p>請點選上方「+ 新增段落」開始。</p></section>`
      : "";
    return;
  }
  const blocks = sec.blocks;
  if (!blocks.length) {
    $("blockList").innerHTML = `
      <section class="empty-state">
        <h3>尚無素材</h3>
        <p>按 + 新增。</p>
        <div class="empty-actions">
          <button class="primary-button" id="emptyAdd">+</button>
        </div>
      </section>
    `;
    $("emptyAdd").onclick = addMaterial;
    return;
  }

  $("blockList").innerHTML = blocks.map((block, index) => blockCardHtml(block, index)).join("");
  bindBlockDrag(blocks);
  document.querySelectorAll(".material-card").forEach((card) => {
    const blockId = card.querySelector("[data-block-id]")?.dataset.blockId;
    if (blockId && blockId === state.selectedBlockId) card.classList.add("selected");
    card.addEventListener("focusin", () => selectBlock(blockId));
    card.addEventListener("mousedown", () => selectBlock(blockId));
  });
  document.querySelectorAll(".card-field").forEach((node) => {
    node.oninput = () => {
      const block = blocks.find((item) => item.block_id === node.dataset.blockId);
      if (!block) return;
      const material = findMaterial(block);
      if (!material) return;
      const sec = section();
      const field = node.dataset.blockSide === "left"
        ? (sec.left_display_field || "file_name")
        : (sec.display_field || "概述");
      setFieldValue(material, field, node.value);
      markDirty();
      markBlockDirty(node.dataset.blockId);
    };
  });
  document.querySelectorAll("[data-save-block]").forEach((node) => {
    if (state.dirtyBlocks.has(node.dataset.saveBlock)) node.classList.add("dirty");
    node.onclick = async () => {
      const blockId = node.dataset.saveBlock;
      const ok = await saveNow(false);
      if (ok) clearBlockDirty(blockId);
    };
  });
  document.querySelectorAll("[data-remove-block]").forEach((node) => {
    node.onclick = () => removeBlock(node.dataset.removeBlock);
  });
  document.querySelectorAll("[data-open-pdf]").forEach((node) => {
    node.onclick = (event) => {
      event.preventDefault();
      event.stopPropagation();
      openLocalPdf(node.dataset.openPdf);
    };
  });
}

async function openLocalPdf(blockId) {
  const sec = section();
  const block = sec?.blocks?.find((b) => b.block_id === blockId);
  if (!block) return;
  const material = findMaterial(block);
  if (!material) return;
  const entry = activeEntry();
  const fileName = material.file_path || material.metadata?.file_name || "";
  if (!entry?.local_path || !fileName) {
    alert("找不到 PDF 本地路徑，請先同步 Drive。");
    return;
  }
  const separator = entry.local_path.includes("\\") ? "\\" : "/";
  const path = `${entry.local_path}${separator}${fileName}`;
  try {
    await bridgeFetch("/open-file", {
      method: "POST",
      body: JSON.stringify({ path }),
    });
  } catch (error) {
    alert(`開啟失敗：${error.message}`);
  }
}

function blockCardHtml(block, index) {
  const material = findMaterial(block);
  const sec = section();
  const leftField = sec.left_display_field || "file_name";
  const rightField = sec.display_field || "概述";
  const leftValue = getFieldValue(material, leftField);
  const rightValue = getFieldValue(material, rightField);
  const tooltip = material?.metadata?.file_name ?? material?.name ?? "Missing material";
  const pdfName = block.block_type === "Paper"
    ? (material?.file_path || material?.metadata?.file_name || "")
    : "";
  const openButton = pdfName
    ? `<button type="button" class="card-open" data-open-pdf="${block.block_id}" title="開啟 ${escapeHtml(pdfName)}" aria-label="開啟 PDF">📂</button>`
    : "";
  return `
    <article class="material-card" draggable="true" data-block-index="${index}">
      <span class="drag-handle" title="拖曳調整順序">⋮⋮</span>
      <textarea class="card-field card-field-left" data-block-side="left" data-block-id="${block.block_id}" title="${escapeHtml(tooltip)}" spellcheck="false">${escapeHtml(leftValue)}</textarea>
      <button type="button" class="card-save" data-save-block="${block.block_id}" title="儲存並上傳" aria-label="儲存並上傳">儲存</button>
      <button type="button" class="card-remove" data-remove-block="${block.block_id}" title="移除素材" aria-label="移除素材">×</button>
      <textarea class="card-field card-field-right" data-block-side="right" data-block-id="${block.block_id}" spellcheck="false">${escapeHtml(rightValue)}</textarea>
      ${openButton}
    </article>
  `;
}

function addMaterial() {
  const sec = section();
  if (!sec) {
    alert("請先新增專案和章節");
    return;
  }
  const data = currentProjectData();
  if (!data) {
    alert("請先選擇專案");
    return;
  }
  const kind = $("materialKind").value;
  if (kind === "Paper" && $("pdfSource")?.value) {
    addPaperFromDrive();
    return;
  }
  const listName = kind.toLowerCase() + "s";
  const list = data[listName] ?? data.notes ?? [];
  const n = list.length + 1;
  const field = $("displayField").value || "概述";
  const now = new Date().toISOString();
  const material = {
    id: crypto.randomUUID(),
    kind,
    name: `素材 ${n}`,
    fields: { [field]: "點擊這裡直接編輯這個素材要放進段落的內容。" },
    metadata: { display_order: String(n), relevance_score: "0", year: "" },
    file_path: "",
    created_at: now,
    updated_at: now,
  };
  list.push(material);
  if (!sec.blocks) sec.blocks = [];
  sec.blocks.push({
    block_id: crypto.randomUUID(),
    idx: sec.blocks.length + 1,
    block_type: kind,
    source_id: material.id,
    display_field: field,
    display_order: sec.blocks.length + 1,
    custom_sentence: "",
    note: "",
    created_at: now,
    updated_at: now,
  });
  sec.title = sec.title === "開始組裝" ? "未命名段落" : sec.title;
  render();
  saveNow(true);
}

function addPaperFromDrive() {
  const data = currentProjectData();
  if (!data) return;
  const pdf = (state.driveIndex.pdfs || []).find((item) => item.id === $("pdfSource").value);
  if (!pdf) return;
  const json = findJsonForPdf(pdf.name);
  const parsed = normalizePaperJson(json?.content, pdf);
  const existing = data.papers.find((item) => item.metadata?.drive_file_id === pdf.id);
  const material = existing ?? {
    id: crypto.randomUUID(),
    kind: "Paper",
    name: parsed.name,
    fields: parsed.fields,
    metadata: parsed.metadata,
    file_path: pdf.name,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
  material.name = parsed.name;
  material.fields = { ...material.fields, ...parsed.fields };
  material.metadata = { ...material.metadata, ...parsed.metadata };
  material.file_path = pdf.name;
  if (!existing) data.papers.push(material);
  if (!section().blocks.some((block) => block.source_id === material.id)) {
    if (!section().blocks) section().blocks = [];
    const now = new Date().toISOString();
    section().blocks.push({
      block_id: crypto.randomUUID(),
      idx: section().blocks.length + 1,
      block_type: "Paper",
      source_id: material.id,
      display_field: $("displayField").value || "概述",
      display_order: section().blocks.length + 1,
      custom_sentence: "",
      note: "",
      created_at: now,
      updated_at: now,
    });
  }
  section().title = section().title === "開始組裝" ? "未命名段落" : section().title;
  render();
  saveNow(true);
}

function findJsonForPdf(pdfName) {
  const base = baseName(pdfName).toLowerCase();
  return (state.driveIndex.jsons || []).find((json) => baseName(json.name).toLowerCase() === base);
}

function normalizePaperJson(content, pdf) {
  const data = content && typeof content === "object" && !Array.isArray(content) ? content : {};
  const fields = data.fields && typeof data.fields === "object" ? { ...data.fields } : {};
  for (const key of paperFields) {
    if (typeof data[key] === "string" && !fields[key]) fields[key] = data[key];
  }
  if (!fields["概述"]) fields["概述"] = "";
  const metadata = data.metadata && typeof data.metadata === "object" ? { ...data.metadata } : {};
  const shortName = data.short_name || data.shortName || data.簡稱 || metadata.short_name || metadata.簡稱 || "";
  metadata.short_name = shortName;
  metadata.file_name = pdf.name;
  metadata.drive_file_id = pdf.id;
  metadata.drive_json_loaded = content ? "true" : "false";
  return {
    name: shortName || truncateName(pdf.name, 28),
    fields,
    metadata,
  };
}

function addProject() {
  $("newProjectDialog").showModal();
}

function activeEntry() {
  return state.projectsList.find(p => p.project_id === state.activeProjectId) || null;
}

function isLocalProject(entry) {
  return !!entry && typeof entry.folder_id === "string" && entry.folder_id.startsWith("local:");
}

function localPathFromEntry(entry) {
  if (!entry) return "";
  return entry.local_path || (entry.folder_id ? entry.folder_id.replace(/^local:/, "") : "");
}

async function confirmAddProject() {
  const name = $("npName").value.trim();
  const folderId = $("npFolderId").value.trim();

  if (!name || !folderId) {
    alert("專案名稱、Folder ID 不能為空");
    return;
  }
  if (!state.workspaceRoot) {
    alert("請先到 Google Drive 設定填寫工作區總資料夾");
    return;
  }

  try {
    showSync("建立專案", "正在建立本地資料夾並同步 Drive PDF / JSON。");
    const result = await bridgeFetch("/add-project", {
      method: "POST",
      body: JSON.stringify({ project_name: name, folder_id: folderId }),
    });

    if (!result.ok) {
      alert(result.message || "新增專案失敗");
      return;
    }

    const created = result.project_entry || {
      project_id: result.project_id,
      project_name: name,
      folder_id: folderId,
      local_path: "",
    };
    state.projectsList.push(created);
    state.projectsData[created.project_id] = result.project_data;
    state.driveIndex = {
      pdfs: result.pdfs || [],
      jsons: result.jsons || [],
    };
    state.activeProjectId = created.project_id;
    state.sectionIndex = 0;
    state.driveMessage = result.message || "已建立專案";
    render();
    $("newProjectDialog").close();
    $("npName").value = "";
    $("npFolderId").value = "";
    await syncDriveFolderIndex({ silent: true, noAlert: true });
  } catch (error) {
    alert("新增專案失敗：" + error.message);
  } finally {
    hideSync();
  }
}

function toggleSectionMenu() {
  const menu = $("sectionMenu");
  menu.hidden = !menu.hidden;
}

function addSectionFromTemplate(title) {
  const proj = project();
  if (!proj) {
    alert("請先新增專案");
    return;
  }
  if (!proj.sections) proj.sections = [];
  const template = sectionTemplates[title] || sectionTemplates.Others;
  proj.sections.push({
    section_id: crypto.randomUUID(),
    idx: proj.sections.length + 1,
    title,
    section_type: template.type,
    left_display_field: "file_name",
    source_folder: "",
    display_field: template.field,
    sort_rules: [],
    blocks: [],
  });
  state.sectionIndex = proj.sections.length - 1;
  normalizeOrder();
  markDirty();
  render();
  saveNow(true);
}

function renameSection(index) {
  const proj = project();
  const sec = proj?.sections?.[index];
  if (!sec) return;
  const next = prompt("章節名稱", sec.title);
  if (next === null) return;
  const trimmed = next.trim();
  if (!trimmed) return;
  sec.title = trimmed;
  render();
  saveNow(true);
}

async function removeProjectAt(index) {
  const entry = state.projectsList[index];
  if (!entry) return;
  if (!confirm(`移除「${entry.project_name}」？會移除本地專案資料夾，雲端 PDF 不動，雲端 JSON 內容會刪除。`)) return;
  try {
    showSync("移除專案", "正在移除本地資料夾並清除 Drive JSON，PDF 會保留。");
    const result = await bridgeFetch("/delete-project", {
      method: "POST",
      body: JSON.stringify(entryPayload(entry)),
    });
    if (!result.ok) {
      alert(result.message || "刪除失敗");
      return;
    }
  } catch (error) {
    alert("刪除失敗：" + error.message);
    return;
  } finally {
    hideSync();
  }
  state.projectsList.splice(index, 1);
  delete state.projectsData[entry.project_id];
  if (state.activeProjectId === entry.project_id) {
    state.activeProjectId = state.projectsList.length > 0 ? state.projectsList[0].project_id : null;
  }
  state.sectionIndex = 0;
  render();
}

async function removeSectionAt(index) {
  const sec = project()?.sections?.[index];
  if (!sec) return;
  if (!confirm(`移除段落「${sec.title}」？素材檔案不會刪除。`)) return;
  const [removedSection] = project().sections.splice(index, 1);
  for (const block of removedSection.blocks || []) {
    removeUnusedMaterial(block);
  }
  state.sectionIndex = Math.max(0, Math.min(state.sectionIndex, project().sections.length - 1));
  render();
  if (await saveNow()) await syncDriveFolderIndex({ silent: true, noAlert: true });
}

function removeBlock(blockId) {
  const index = section().blocks.findIndex((block) => block.block_id === blockId);
  if (index < 0) return;
  const material = findMaterial(section().blocks[index]);
  if (!confirm(`移除素材「${materialDisplayName(material, "name")}」？素材檔案不會刪除。`)) return;
  const [block] = section().blocks.splice(index, 1);
  removeUnusedMaterial(block);
  render();
  (async () => {
    if (await saveNow()) await syncDriveFolderIndex({ silent: true, noAlert: true });
  })();
}

function removeUnusedMaterial(block) {
  const data = currentProjectData();
  if (!data) return null;
  const stillUsed = data.project.sections.some((sectionItem) =>
    sectionItem.blocks.some((item) => item.source_id === block.source_id)
  );
  if (stillUsed) return null;
  const listName = block.block_type.toLowerCase() + "s";
  const list = data[listName] ?? data.notes;
  const idx = list.findIndex((item) => item.id === block.source_id);
  if (idx < 0) return null;
  list.splice(idx, 1);
  return true;
}

function bindBlockDrag(blocks) {
  let fromIndex = null;
  document.querySelectorAll("[data-block-index]").forEach((node) => {
    node.addEventListener("dragstart", (event) => {
      fromIndex = Number(node.dataset.blockIndex);
      node.classList.add("dragging");
      event.dataTransfer.effectAllowed = "move";
    });
    node.addEventListener("dragend", () => {
      node.classList.remove("dragging");
      fromIndex = null;
    });
    node.addEventListener("dragover", (event) => {
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
    });
    node.addEventListener("drop", (event) => {
      event.preventDefault();
      const toIndex = Number(node.dataset.blockIndex);
      if (fromIndex === null || fromIndex === toIndex) return;
      const [moved] = blocks.splice(fromIndex, 1);
      blocks.splice(toIndex, 0, moved);
      normalizeOrder();
      render();
      saveNow(true);
    });
  });
}

function bindSectionDrag(sections) {
  let fromIndex = null;
  document.querySelectorAll("[data-section]").forEach((node) => {
    node.addEventListener("dragstart", (event) => {
      fromIndex = Number(node.dataset.section);
      node.classList.add("dragging");
      event.dataTransfer.effectAllowed = "move";
    });
    node.addEventListener("dragend", () => {
      node.classList.remove("dragging");
      fromIndex = null;
    });
    node.addEventListener("dragover", (event) => {
      event.preventDefault();
      event.dataTransfer.dropEffect = "move";
    });
    node.addEventListener("drop", (event) => {
      event.preventDefault();
      const toIndex = Number(node.dataset.section);
      if (fromIndex === null || fromIndex === toIndex) return;
      const activeSection = section();
      const [moved] = sections.splice(fromIndex, 1);
      sections.splice(toIndex, 0, moved);
      normalizeOrder();
      state.sectionIndex = Math.max(0, sections.findIndex((item) => item.section_id === activeSection?.section_id));
      render();
      saveNow(true);
    });
  });
}

function normalizeOrder() {
  const data = currentProjectData();
  const sections = data?.project?.sections || [];
  sections.forEach((sec, sectionIndex) => {
    sec.idx = sectionIndex + 1;
    (sec.blocks || []).forEach((block, blockIndex) => {
      block.idx = blockIndex + 1;
      block.display_order = blockIndex + 1;
      const material = findMaterial(block);
      if (material) {
        material.metadata = material.metadata || {};
        material.metadata.idx = String(blockIndex + 1);
        material.metadata.display_order = String(blockIndex + 1);
      }
    });
  });
}

function findMaterial(block) {
  const data = currentProjectData();
  if (!data) return null;
  const listName = block.block_type.toLowerCase() + "s";
  const list = data[listName] ?? data.notes ?? [];
  return list.find((item) => item.id === block.source_id);
}

async function saveNow(_silent = true) {
  const data = currentProjectData();
  if (!data || !state.activeProjectId) return false;
  if (state.syncing) return false;
  normalizeOrder();
  touchProjectData(data);
  const entry = activeEntry();
  const endpoint = isLocalProject(entry) ? "/save-project" : "/upload-project";
  state.syncing = true;
  setSyncStatus("syncing", "同步中…");
  try {
    const result = await bridgeFetch(endpoint, {
      method: "POST",
      body: JSON.stringify({
        ...entryPayload(entry),
        project_data: data,
      }),
    });
    state.driveMessage = result.message || "已儲存";
    if (!isLocalProject(entry)) state.driveConnected = true;
    state.dirty = false;
    clearBlockDirty();
    setSyncStatus("synced", formatSyncedAt(new Date()));
    return true;
  } catch (error) {
    state.driveMessage = `儲存失敗：${error.message}`;
    setSyncStatus("error", "同步失敗");
    return false;
  } finally {
    state.syncing = false;
  }
}

function formatSyncedAt(date) {
  const hh = String(date.getHours()).padStart(2, "0");
  const mm = String(date.getMinutes()).padStart(2, "0");
  return `已同步 · ${hh}:${mm}`;
}

function startAutoSync() {
  if (state.autosyncTimer) clearInterval(state.autosyncTimer);
  state.autosyncTimer = setInterval(async () => {
    if (state.syncing) return;
    if (!state.activeProjectId) return;
    if (state.dirty) {
      try {
        const saved = await saveNow();
        if (!saved) return;
      } catch (_) {
        return;
      }
    }
    const entry = activeEntry();
    if (!entry) return;
    try {
      state.syncing = true;
      setSyncStatus("syncing", isLocalProject(entry) ? "檢查本地資料夾…" : "檢查 Drive…");
      await syncDriveFolderIndex({ silent: true, noAlert: true });
      setSyncStatus("synced", formatSyncedAt(new Date()));
    } catch (_) {
      setSyncStatus("error", "同步失敗");
    } finally {
      state.syncing = false;
    }
  }, 5_000);
}

function entryPayload(entry = activeEntry()) {
  return {
    project_id: entry?.project_id || state.activeProjectId || "",
    project_name: entry?.project_name || project()?.project_name || "",
    folder_id: entry?.folder_id || project()?.folder_id || "",
    local_path: entry?.local_path || "",
  };
}

function markDirty() {
  state.dirty = true;
  if (state.syncStatus !== "syncing" && state.syncStatus !== "error") {
    setSyncStatus("idle", "尚未同步");
  }
}

function showSync(title, message) {
  state.syncing = true;
  const dialog = $("syncDialog");
  $("syncTitle").textContent = title || "同步中";
  $("syncMessage").textContent = message || "正在更新本地資料夾。";
  if (dialog && !dialog.open) dialog.showModal();
}

function hideSync() {
  state.syncing = false;
  const dialog = $("syncDialog");
  if (dialog?.open) dialog.close();
}

function touchProjectData(data) {
  const now = new Date().toISOString();
  if (data.project) data.project.updated_at = now;
  for (const material of [...(data.papers || []), ...(data.algorithms || []), ...(data.images || []), ...(data.tables || []), ...(data.notes || [])]) {
    if (!material.created_at) material.created_at = now;
    material.updated_at = now;
    material.fields = material.fields || {};
    material.metadata = material.metadata || {};
    if (!material.metadata.short_name && material.name) material.metadata.short_name = material.name;
  }
  for (const sec of data.project?.sections || []) {
    if (!sec.left_display_field) sec.left_display_field = "file_name";
    if (!sec.source_folder) sec.source_folder = "";
    if (!sec.sort_rules) sec.sort_rules = [];
    for (const block of sec.blocks || []) {
      if (!block.created_at) block.created_at = now;
      block.updated_at = now;
      if (!block.display_field) block.display_field = sec.display_field || "概述";
      if (!block.custom_sentence) block.custom_sentence = "";
      if (!block.note) block.note = "";
    }
  }
}

function exportSectionMarkdown(data, sec) {
  const lines = [`## ${sec.title}`, ""];
  for (const block of sec.blocks || []) {
    const material = findMaterialInData(data, block);
    const title = materialDisplayName(material, sec.left_display_field || "file_name");
    const content = material?.fields?.[sec.display_field] || "";
    lines.push(`### ${title}`, "", content, "");
  }
  return lines.join("\n");
}

function findMaterialInData(data, block) {
  const listName = block.block_type.toLowerCase() + "s";
  return (data[listName] || []).find((item) => item.id === block.source_id);
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function baseName(name) {
  return String(name || "").replace(/\.[^/.]+$/, "");
}

function truncateName(name, max = 24) {
  const value = String(name || "");
  return value.length > max ? `${value.slice(0, Math.max(1, max - 3))}...` : value;
}

function materialDisplayName(material, field = "short_name") {
  if (!material) return "Missing material";
  if (field === "file_name") return truncateName(material.metadata?.file_name || material.file_path || material.name, 30);
  if (field === "name") return material.name || "未命名素材";
  if (field === "year") return material.metadata?.year || material.name || "未填年份";
  if (field === "kind") return material.kind || "素材";
  return material.metadata?.short_name || material.metadata?.["簡稱"] || material.name || truncateName(material.metadata?.file_name || material.file_path, 24);
}

$("addProject").onclick = addProject;
$("cancelNewProject").onclick = () => $("newProjectDialog").close();
$("confirmNewProject").onclick = confirmAddProject;
$("addSection").onclick = toggleSectionMenu;
$("sectionMenu").addEventListener("click", (event) => {
  const item = event.target.closest("[data-section-template]");
  if (!item) return;
  event.preventDefault();
  $("sectionMenu").hidden = true;
  addSectionFromTemplate(item.dataset.sectionTemplate);
});
$("addMaterial").onclick = addMaterial;
$("connectDrive").onclick = () => $("driveDialog").showModal();
$("saveDriveSettings").onclick = async (event) => {
  event.preventDefault();
  await connectDrive();
};
$("materialKind").onchange = renderPdfSources;
$("leftDisplayField").onchange = () => {
  const sec = section();
  if (sec) {
    sec.left_display_field = $("leftDisplayField").value;
    normalizeOrder();
    render();
    saveNow(true);
  }
};
$("displayField").onchange = () => {
  const sec = section();
  if (sec) {
    sec.display_field = $("displayField").value;
    normalizeOrder();
    render();
    saveNow(true);
  }
};
$("sectionTitle").onblur = () => {
  const sec = section();
  if (!sec) return;
  const next = $("sectionTitle").textContent.trim();
  if (!next) {
    $("sectionTitle").textContent = sec.title;
    return;
  }
  if (next !== sec.title) {
    sec.title = next;
    renderSections();
    saveNow(true);
  }
};
$("sectionTitle").onkeydown = (event) => {
  if (event.key === "Enter") {
    event.preventDefault();
    $("sectionTitle").blur();
  }
};
$("exportMarkdown").onclick = () => {
  const data = currentProjectData();
  const sec = section();
  if (!data || !sec) {
    alert("請先選擇段落");
    return;
  }
  const md = exportSectionMarkdown(data, sec);
  $("exportTitle").textContent = `匯出 Markdown — ${sec.title}`;
  $("exportMdContent").value = md;
  $("exportDialog").showModal();
};

$("copyMd").onclick = async () => {
  const textarea = $("exportMdContent");
  const text = textarea.value;
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
    } else {
      textarea.removeAttribute("readonly");
      textarea.focus();
      textarea.select();
      document.execCommand("copy");
      textarea.setAttribute("readonly", "");
    }
    const btn = $("copyMd");
    const original = btn.textContent;
    btn.textContent = "已複製";
    setTimeout(() => { btn.textContent = original; }, 1200);
  } catch (e) {
    alert("複製失敗，請手動全選複製。");
  }
};

$("removeSection").onclick = () => removeSectionAt(state.sectionIndex);

function syncColorDialogInputs() {
  document.querySelectorAll("[data-color-key]").forEach((node) => {
    const key = node.dataset.colorKey;
    const value = state.colors[key];
    if (value === undefined) return;
    node.value = value;
  });
}

$("openColorDialog").onclick = () => {
  renderPresetChips();
  syncColorDialogInputs();
  $("colorDialog").showModal();
};

document.querySelectorAll("[data-color-key]").forEach((node) => {
  node.addEventListener("input", () => {
    const key = node.dataset.colorKey;
    let value = node.value;
    if (node.type === "number") value = Number(value) || colorDefaults[key];
    else if (node.type === "range") value = Number(value);
    state.colors[key] = value;
    applyColors();
  });
});

$("saveColors").onclick = async (event) => {
  event.preventDefault();
  await saveColors();
  $("colorDialog").close();
};

$("resetColors").onclick = () => {
  state.colors = { ...colorDefaults };
  applyColors();
  syncColorDialogInputs();
};

document.addEventListener("pointerdown", (event) => {
  if (!event.target.closest("#sectionMenu") && !event.target.closest("#addSection")) {
    $("sectionMenu").hidden = true;
  }
}, true);

document.addEventListener("click", (event) => {
  const blockButton = event.target.closest("[data-remove-block]");
  if (blockButton) {
    event.preventDefault();
    event.stopPropagation();
    removeBlock(blockButton.dataset.removeBlock);
  }
}, true);

window.addEventListener("beforeunload", (event) => {
  if (!state.dirty) return;
  event.preventDefault();
  event.returnValue = "尚有未儲存內容，確定要關閉？";
});

window.paperComposerHasUnsavedChanges = () => state.dirty;

bootstrap();

async function bootstrap() {
  await loadColors();
  setSyncStatus("idle", "就緒");
  render();
  await loadProjectDriveSettings();
  if (!state.driveConnected || !state.workspaceRoot) {
    $("driveDialog").showModal();
    return;
  }
  showSync("啟動同步", "正在把 Drive 內的 PDF / JSON 更新到本地專案資料夾。");
  await initializeBridge();
  hideSync();
  if (state.driveConnected) setSyncStatus("synced", formatSyncedAt(new Date()));
  startAutoSync();
}

async function connectDrive() {
  setSyncStatus("syncing", "連線中…");
  const payload = {
    client_id: $("clientId").value.trim(),
    client_secret: $("clientSecret").value.trim(),
    workspace_root: $("workspaceRoot").value.trim(),
  };
  try {
    const result = await bridgeFetch("/connect", {
      method: "POST",
      body: JSON.stringify(payload),
    });
    state.driveConnected = Boolean(result.authenticated);
    state.storedSecret = result.has_client_secret;
    state.workspaceRoot = $("workspaceRoot").value.trim();
    state.driveMessage = result.message;
    $("driveDialog").close();
    render();
    await initializeBridge();
    setSyncStatus("synced", formatSyncedAt(new Date()));
  } catch (error) {
    setSyncStatus("error", "連線失敗");
    alert(error.message);
  }
}

async function syncDriveFolderIndex(options = {}) {
  const silent = Boolean(options.silent);
  const noAlert = Boolean(options.noAlert);
  const entry = activeEntry();
  if (!entry) {
    if (!silent) alert("請先選擇專案");
    return;
  }
  try {
    if (!silent) showSync("同步專案", "正在更新本地資料夾中的 PDF / JSON。");
    const result = await bridgeFetch("/sync-project", {
      method: "POST",
      body: JSON.stringify(entryPayload(entry)),
    });
    state.driveIndex = {
      pdfs: result.pdfs || [],
      jsons: result.jsons || [],
    };
    if (result.project_entry) {
      const idx = state.projectsList.findIndex((item) => item.project_id === result.project_entry.project_id);
      if (idx >= 0) state.projectsList[idx] = result.project_entry;
    }
    if (result.project_data && !state.dirty) {
      state.projectsData[entry.project_id] = result.project_data;
    }
    state.driveMessage = result.message;
    if (!isLocalProject(entry)) {
      state.driveConnected = true;
    }
    render();
    if (!silent && !noAlert) alert(result.message);
  } catch (error) {
    state.driveMessage = `掃資料夾失敗：${error.message}`;
    render();
    if (!silent && !noAlert) alert(error.message);
  } finally {
    if (!silent) hideSync();
  }
}

async function uploadToDrive() {
  const data = currentProjectData();
  if (!data) {
    alert("請先選擇專案");
    return;
  }
  const entry = activeEntry();
  const isLocal = isLocalProject(entry);
  const endpoint = isLocal ? "/save-project" : "/upload-project";

  try {
    $("driveStatus").textContent = isLocal ? "存檔中" : "上傳中";
    const result = await bridgeFetch(endpoint, {
      method: "POST",
      body: JSON.stringify({
        ...entryPayload(entry),
        project_data: data,
      }),
    });
    if (!isLocal) state.driveConnected = true;
    state.driveMessage = result.message;
    render();
    alert(result.message);
  } catch (error) {
    $("driveStatus").textContent = isLocal ? "存檔失敗" : "上傳失敗";
    alert(error.message);
  }
}

async function initializeBridge() {
  try {
    const result = await bridgeFetch("/startup-sync");
    if (result.ok) {
      state.projectsList = result.projects || [];
      state.projectsData = {};
      if (result.projects_data) {
        for (const project of result.projects || []) {
          state.projectsData[project.project_id] = result.projects_data[project.project_id];
        }
      }
      if (state.projectsList.length > 0 && !state.activeProjectId) {
        state.activeProjectId = state.projectsList[0].project_id;
      }
      state.driveConnected = true;
      state.driveMessage = result.message || "已同步";
    } else {
      state.driveMessage = result.message || "同步失敗";
    }
  } catch (error) {
    state.driveMessage = "橋接未啟動";
  }
  render();
  if (state.activeProjectId) {
    await syncDriveFolderIndex({ silent: true });
  }
}

async function loadProjectDriveSettings() {
  try {
    const settings = await bridgeFetch("/settings");
    state.driveConnected = Boolean(settings.authenticated);
    state.storedSecret = Boolean(settings.has_client_secret);
    state.workspaceRoot = settings.workspace_root || "";
    state.driveMessage = settings.configured ? settings.message : "尚未設定 Drive";
    $("clientId").value = settings.client_id || "";
    $("workspaceRoot").value = settings.workspace_root || "";
    if (settings.has_client_secret) {
      $("clientSecret").placeholder = "已儲存，留空即可沿用";
    }
    render();
  } catch (_) {
    state.driveMessage = "Drive 後端未啟動";
    render();
  }
}


async function bridgeFetch(path, options = {}) {
  const response = await fetch(`${bridgeUrl}${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  const text = await response.text();
  const result = text ? JSON.parse(text) : {};
  if (!response.ok || result.ok === false) {
    throw new Error(result.message || `Bridge error ${response.status}`);
  }
  return result;
}
