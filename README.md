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

## Google Drive API 設定

Paper Composer 需要 Google Drive API 來同步每個專案資料夾中的 PDF、JSON 與 `project.json`。第一次使用前，請先在 Google Cloud 建立 OAuth 憑證，並在 app 內填入設定。

需要準備的值:

```text
Google OAuth Client ID
Google OAuth Client Secret
Google Drive Folder ID
工作區總資料夾路徑
```

### 1. 建立或選擇 Google Cloud 專案

進入 [Google Cloud Console](https://console.cloud.google.com/)，建立新專案，或選擇既有專案。

### 2. 啟用 Google Drive API

進入:

```text
APIs & Services > Library
```

搜尋 `Google Drive API`，點進去後按 `Enable`。

### 3. 設定 OAuth Consent Screen

進入:

```text
APIs & Services > OAuth consent screen
```

開發測試時可維持 `Testing`。基本欄位填:

```text
App name: Paper Composer
User support email: 你的 Gmail
Developer contact information: 你的 Gmail
```

測試階段請在 `Test users` 加入自己的 Gmail。

### 4. 建立 OAuth Client

進入:

```text
APIs & Services > Credentials > Create Credentials > OAuth client ID
```

應用程式類型選:

```text
網頁應用程式
```

建立後取得:

```text
Client ID
Client Secret
```

### 5. 在 Paper Composer 填入設定

開啟 app 後進入 Google Drive 設定，填入:

```text
Client ID
Client Secret
工作區總資料夾
```

`工作區總資料夾` 是本機用來存放所有專案資料夾的位置，例如:

```text
/Users/你的帳號/Documents/PaperComposerProjects
```

設定完成後，app 會開啟瀏覽器進行 Google OAuth 授權。授權成功後，token 會儲存在本機 `user_data/drive_settings.json`。

### 6. 取得 Google Drive Folder ID

每個 Paper Composer 專案會對應一個 Google Drive 資料夾。打開要同步的 Drive 資料夾，網址通常像:

```text
https://drive.google.com/drive/folders/1AbCDefGhijkLmNoPqRsTuvWxYz
```

`/folders/` 後面的字串就是 Folder ID:

```text
1AbCDefGhijkLmNoPqRsTuvWxYz
```

在 app 內新增專案時，將這個 Folder ID 貼到 `Google Drive Folder ID`。

### 注意事項

- `Client Secret` 與 `user_data/drive_settings.json` 不要提交到 GitHub。
- 本專案的 `.gitignore` 已忽略 `user_data`，避免本機 token 被提交。
- 如果 OAuth 失敗，請確認 OAuth Client 類型是 `Desktop app`，且自己的 Gmail 已加入 Test users。
