# PaperComposer

本機運作的論文寫作編輯器：在排版好的畫面上直接編輯，用你自己的句型資料庫輔助寫作與檢查段落，輸出期刊要求的 LaTeX 或 Word。

> 開發初期。已完成：句型資料庫讀取與驗證、句型檢索、硬規則檢查、一次性段落生成（語氣鎖定）、桌面殼與介面雛形。

## 執行

需要 [Rust](https://rustup.rs)（1.85 以上）。

**桌面版**

```sh
cargo run -p papercomposer
```

- Windows：需要 Visual Studio Build Tools（C++）與 WebView2（Windows 10/11 已內建）。
- macOS：需要 Xcode Command Line Tools。
- 打包安裝檔：`cargo install tauri-cli --version "^2"`，再於 `apps/desktop/` 執行 `cargo tauri build`。Windows 產生 `.msi` / `.exe`，macOS 產生 `.app` / `.dmg`。

**瀏覽器開發模式**（改介面時最快）

```sh
cargo run -p pc-server
```

開啟 <http://127.0.0.1:7878>。

沒有指定工作區時，兩種模式都使用內建示範資料（`workspace-template/`），右上角顯示「示範資料」。

## 你的私人工作區

句型、模板、論文都是個人資料，**放在你自己的私有 repository**，不要放進這個 repo。結構如下（範本見 [`workspace-template/`](workspace-template/)）：

```
your-private-repo/
  profile/      句型、用詞、規則、範例段落、公式片段
  publishers/   各出版社的 .cls、範例 .tex 與模板設定
  papers/       每篇論文一個資料夾
  prompts/      自訂提示詞（可選）
```

第一次開啟 App 會出現**設定精靈**（之後可從「設定 → 重新設定」開啟），四個步驟都可略過：

1. **工作區**：三選一，每位使用者都指向**自己的**私有 repo，App 不綁定任何特定帳號
   - 開啟資料夾：已經在電腦上的工作區；缺少的資料夾與範本檔會自動補上（不覆蓋既有檔案）
   - Clone 我的 repo：貼上你的私有 repo 網址；若偵測到是**公開** repo 會提醒先改成 private
   - 建立新的：從範本建立新資料夾並 `git init`，可選擇填入遠端，第一次 commit 後直接 push
2. **Git**：檢查是否安裝 Git、設定 commit 作者名稱與 email
3. **本機模型**：偵測記憶體（含 Mac 統一記憶體與 NVIDIA 顯示卡），標示推薦與放不下的模型。按「準備下載」先列出要下載的檔案、大小與來源，**確認後才下載**；下載完成自動啟動並設為預設
4. **Claude**：貼上 API key（只存在系統金鑰圈）

也可以手動指定位置（擇一）：

- 環境變數 `PAPERCOMPOSER_WORKSPACE=/path/to/your-private-repo`
- 設定檔 `PaperComposer/config.toml` 寫入 `workspace_path = "/path/to/your-private-repo"`
  （Windows：`%APPDATA%`；macOS：`~/Library/Application Support/`；Linux：`~/.config/`）

設定檔在所有 repo 之外，路徑不會被 commit。欄位說明見 [`workspace-template/profile/README.md`](workspace-template/profile/README.md)。

## Git 同步

工作區是 Git repository 時（建議是你的私有 repo）：

- **開啟 App**：自動 pull。只做 fast-forward；兩邊都有新 commit 時會提示你按同步。
- **Ctrl+S**（或點狀態列的同步狀態）：commit 所有修改並 push。commit 訊息依修改的檔案自動產生，例如 `Update my-paper (method, result); profile (phrases)`。
- **另一台裝置先推了**：自動把你的修改接在遠端之後再 push；若兩邊改到同一句，**中止並還原**，你的修改仍保留在本機，不會自動合併。
- **commit 前檢查金鑰**：新增的內容看起來像 API key 或 token 時，拒絕 commit。

使用系統安裝的 `git`，登入交給你電腦上的 Git 憑證管理（Windows 為 Git Credential Manager），App 不保存 Git 密碼。作者名稱沿用你的 `git config user.name` 與 `user.email`。

## 公式

紙張上的數學式用內建的 KaTeX 顯示（離線可用），支援 `$…$`、`\[…\]`、`equation`／`align` 等環境、自動編號與 `\eqref`，並會讀取論文 `main.tex` 與 `macros*.tex` 裡的 `\newcommand`、`\DeclareMathOperator`。這是編輯時的預覽，投稿用的 PDF 仍由 LaTeX 搭配期刊 cls 產生。

## 連接模型

### 一鍵安裝的本機模型

設定精靈第 3 步會從 Hugging Face 下載 GGUF 模型、從 GitHub 下載對應平台的 [llama.cpp](https://github.com/ggml-org/llama.cpp)（Windows 用 Vulkan 版，支援 NVIDIA／AMD／Intel 顯示卡；Mac 用 Metal 版），存在本機資料夾（Windows：`%APPDATA%\PaperComposer`；macOS：`~/Library/Application Support/PaperComposer`；Linux：`~/.local/share/PaperComposer`）。App 啟動時自動開啟上次使用的模型，關閉時一併結束。

可選的模型列在 [`models/catalog.json`](models/catalog.json)，目前預設 Gemma 4 12B（Q8_0；記憶體較少時用 Q4_K_XL）。之後有新模型（例如 Gemma 5），在清單加一筆即可出現在精靈中：

```json
{
  "id": "gemma-5-xx-q8",
  "name": "Gemma 5 XX",
  "variant": "Q8_0",
  "summary": "新一代",
  "repo": "unsloth/gemma-5-xx-it-GGUF",
  "file_suffix": "Q8_0.gguf",
  "mmproj_suffix": "mmproj-BF16.gguf",
  "approx_gb": 13,
  "min_memory_gb": 24,
  "context": 32768,
  "license": "…",
  "vision": true
}
```

檔名由 Hugging Face 的檔案清單比對 `file_suffix` 找出，不需要寫完整網址。不想改 repo 時，可把自己的 `catalog.json` 放在上述本機資料夾的 `models/` 底下，會取代內建清單。

### 自己架設的模型

也可以使用任何 OpenAI 相容的端點，例如 llama.cpp、Ollama、LM Studio，或家裡的另一台電腦：

```toml
# config.toml
[engine]
url = "http://127.0.0.1:8080/v1"
model = "gemma-4-12b"
```

或用環境變數 `PAPERCOMPOSER_LLM_URL`、`PAPERCOMPOSER_LLM_MODEL`。未連接時，問答框仍會列出參考句型，並可複製完整提示詞到其他模型使用。

**Claude**：在「設定」貼上 API key，key 只存在系統金鑰圈（Windows 認證管理員、macOS 鑰匙圈），不寫入任何檔案；也可以用環境變數 `ANTHROPIC_API_KEY`。資料庫的「拆解」功能優先使用 Claude，未設定時改用本機模型。

## 專案結構

```
crates/core         資料庫讀取與驗證、句型檢索、段落檢查、生成提示詞與輸出驗證（不需模型）
crates/server       本機伺服器（只監聽 127.0.0.1），桌面版與瀏覽器模式共用
apps/desktop        Tauri 桌面殼：啟動伺服器並開啟視窗
ui/                 介面（HTML / CSS / JS）
workspace-template/ 私人工作區範本與示範資料
scripts/            Git hooks
```

## API

| 方法 | 路徑 | 說明 |
| --- | --- | --- |
| GET | `/api/workspace` | 工作區位置、論文與出版社清單 |
| GET | `/api/profile` | 資料庫名稱、筆數、格式問題 |
| POST | `/api/profile/reload` | 重新讀取資料庫 |
| GET | `/api/phrases?section=&function=&position=&text=&limit=` | 依欄位篩選並排序句型 |
| POST | `/api/check` `{ text, section }` | 逐段檢查 |
| GET | `/api/engine` | 模型連線狀態 |
| POST | `/api/compose` `{ points, section, function, min_words, max_words, language, before, after }` | 一次性段落生成 |
| GET | `/api/papers/{name}` | 論文各章節與段落 |
| PUT | `/api/papers/{name}/sections/{file}` `{ title, paragraphs }` | 儲存章節（一句一行） |
| GET / PUT | `/api/tables/{file}` | 讀寫資料庫 CSV（句型、片語、用詞、規則、範例、公式） |
| POST | `/api/decompose` `{ text, section }` | 把句子或段落拆成句型與片語建議 |
| GET | `/api/claude` | Claude 連線狀態 |
| POST | `/api/settings/claude` `{ key, model }` | 存入或移除 API key、選擇模型 |
| GET | `/api/git/status` | 分支、遠端、未同步的檔案、領先或落後的 commit 數 |
| POST | `/api/git/pull` | 從遠端更新（只做 fast-forward） |
| POST | `/api/git/sync` `{ message? }` | commit 並 push |
| GET | `/api/setup/status` | 設定精靈：工作區、Git、記憶體、模型、Claude 狀態 |
| POST | `/api/setup/workspace` `{ path, init }` | 使用既有資料夾（可補上範本） |
| POST | `/api/setup/clone` `{ url, parent }` | clone 自己的私有 repo |
| POST | `/api/setup/new` `{ path, remote? }` | 從範本建立新工作區 |
| GET | `/api/setup/visibility?url=` | 查詢 GitHub repo 是否公開 |
| POST | `/api/setup/git-identity` `{ name, email }` | 設定 Git 作者 |
| POST | `/api/models/plan` `{ id }` | 列出要下載的檔案、大小與來源 |
| POST | `/api/models/install` `{ id }` | 下載並啟用模型 |
| GET | `/api/models/progress` | 下載進度 |
| POST | `/api/models/start` / `/api/models/stop` | 啟動或停止本機模型 |

## 防止洩漏個人資料

```sh
sh scripts/install-hooks.sh
```

安裝 commit 前的金鑰掃描（需要 [gitleaks](https://github.com/gitleaks/gitleaks)）。

## 開發說明

本專案在開發過程中使用 Claude（Anthropic）協助撰寫程式碼。
