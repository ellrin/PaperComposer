# PaperComposer

本機運作的論文寫作編輯器：在排版好的畫面上直接編輯，用你自己的句型資料庫輔助寫作與檢查段落，輸出期刊要求的 LaTeX 或 Word。

> 目前是開發初期的骨架：句型資料庫讀取與驗證、句型檢索、硬規則檢查，以及介面雛形。

## 快速開始

需要 Rust（1.85 以上）。

```sh
cargo run -p pc-server
```

開啟 <http://127.0.0.1:7878>。沒有指定個人設定檔時，會使用內建的示範資料（`profile-template/`），右上角顯示「示範資料」。

## 使用你自己的寫作資料庫

你的句型、用詞與規則屬於個人資料，**請放在自己的私有 repository**，不要放進這個 repo。

1. 建立一個私有 repo，把 `profile-template/` 裡的檔案複製進去，換成你自己的內容。
2. 用以下任一方式告訴 App 位置：
   - 環境變數：`PAPERCOMPOSER_PROFILE=/path/to/profile cargo run -p pc-server`
   - 設定檔：在系統設定資料夾的 `PaperComposer/config.toml` 寫入
     `profile_path = "/path/to/profile"`
     （macOS：`~/Library/Application Support/`；Windows：`%APPDATA%`；Linux：`~/.config/`）

設定檔放在所有 repo 之外，路徑不會被 commit。欄位說明見 [`profile-template/README.md`](profile-template/README.md)。

## 專案結構

```
crates/core     句型資料庫讀取與驗證、句型檢索、段落檢查（不需模型）
crates/server   本機開發伺服器（只監聽 127.0.0.1），之後改接 Tauri
ui/             介面（HTML / CSS / JS）
profile-template/  寫作資料庫範本與示範資料
scripts/        Git hooks
```

## API（開發用）

| 方法 | 路徑 | 說明 |
| --- | --- | --- |
| GET | `/api/profile` | 設定檔名稱、筆數、格式問題 |
| POST | `/api/profile/reload` | 重新讀取設定檔 |
| GET | `/api/phrases?section=&function=&position=&text=&limit=` | 依欄位篩選並排序句型 |
| POST | `/api/check` `{ text, section }` | 逐段檢查，回傳觸發的規則 |

## 防止洩漏個人資料

```sh
sh scripts/install-hooks.sh
```

安裝 commit 前的金鑰掃描（需要 [gitleaks](https://github.com/gitleaks/gitleaks)）。API key 一律存在系統金鑰圈，不寫入任何檔案。

## 開發說明

本專案在開發過程中使用 Claude（Anthropic）協助撰寫程式碼。
