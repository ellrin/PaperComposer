# Paper Composer

Paper Composer 是一個本地優先的論文素材組裝工具。每個專案都對應一個 Google Drive 資料夾，並放在工作區總資料夾下的一個專案子資料夾。

## Data Model

- `user_data/drive_settings.json`: 本機 Google OAuth 設定與 token。
- `user_data/workspace_settings.json`: 只記錄工作區總資料夾。
- 不使用專案索引檔。app 啟動時掃描工作區總資料夾底下含 `project.json` 的子資料夾。
- 每個專案子資料夾:
  - `project.json`: 章節、素材、顯示欄位與編輯內容。
  - `*.pdf`: 從 Drive 同步下來或本地新增的 PDF。
  - `*.json`: 從 Drive 同步下來或本地新增的素材 JSON。

## Sync Rules

- 開啟 app 時會掃描工作區總資料夾，並自動同步所有專案。
- 新增專案時會在工作區總資料夾底下建立專案子資料夾，並同步 Drive 內的 PDF 與 JSON。
- 已存在於本地的 PDF 不會重複下載。
- JSON 會以 Drive 最新內容更新到本地。
- 儲存素材時會寫入本地 `project.json`，再把本地資料夾內的 PDF / JSON 上傳或更新到 Drive。

## Build

```bash
./scripts/build_macos_app.sh
```

輸出位置:

```text
./Paper Composer.app
./dist/Paper Composer.app
```
