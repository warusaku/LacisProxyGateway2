# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## 絶対ルール（Critical Rules）

### 作業ディレクトリは唯一無二
```
唯一の作業場所: /Users/hideakikurata/Library/CloudStorage/Dropbox/Mac (3)/Documents/obsidian/LacisProxyGateway2/code

❌ 禁止: /private/tmp/* での開発、サーバー上でのコード編集
✅ 必須: code/ で編集 → git commit → git push → ./scripts/deploy.sh
```

### 開発フロー
```
[ローカルMac]                [サーバー 192.168.3.242]
   編集 → commit → push
   ./scripts/deploy.sh  ───▶  rsync → build → restart
```

---

## コマンド

### Backend (Rust)
```bash
cd backend
cargo check                    # 型チェック（ローカル確認用）
cargo fmt && cargo clippy      # フォーマット・Lint
cargo test                     # テスト実行

# 環境変数で設定上書き
LACISPROXY__SERVER__HOST=0.0.0.0 LACISPROXY__SERVER__PORT=8080 cargo run
```

### Frontend (Next.js)
```bash
cd frontend
npm run dev                    # 開発サーバー (:3000)
npm run build                  # ビルド
npm run lint                   # ESLint
npm run type-check             # TypeScript型チェック
```

### デプロイ
```bash
./scripts/deploy.sh all        # 全体デプロイ
./scripts/deploy.sh backend    # バックエンドのみ
./scripts/deploy.sh frontend   # フロントエンドのみ
./scripts/deploy.sh status     # サービス状態確認
```

### サーバー操作
```bash
ssh akihabara_admin@192.168.3.242
sudo systemctl status lacis-proxy-gateway     # 状態
sudo systemctl restart lacis-proxy-gateway    # 再起動
journalctl -u lacis-proxy-gateway -f          # ログ
```

---

## アーキテクチャ

### コンポーネント構成
```
backend/                         # Rust (axum + tokio)
├── src/
│   ├── main.rs                 # エントリポイント、Router構築
│   ├── api/                    # HTTPハンドラ・ルート定義
│   │   ├── mod.rs              # routes() - 全エンドポイント定義
│   │   └── handlers.rs         # 各APIハンドラ実装
│   ├── config/                 # 設定読込 (TOML + 環境変数)
│   └── error/                  # エラー型定義
└── config/default.toml         # デフォルト設定

frontend/                        # Next.js 14 + TypeScript + Tailwind
└── src/app/                    # App Router
```

### 設定の優先順位
1. 環境変数 `LACISPROXY__*` (最優先)
2. `config/default.toml`
3. ハードコードデフォルト (host: 0.0.0.0, port: 8081)

### API エンドポイント
| Path | 用途 |
|------|------|
| `/health`, `/api/health` | ヘルスチェック |
| `/api/routes` | プロキシルート管理 |
| `/api/ddns` | DDNS設定管理 |
| `/api/security/blocked-ips` | IPブロック管理 |
| `/api/dashboard/*` | 統計・アクセスログ |

---

## ビルド注意事項

**MacローカルのバイナリはLinuxサーバーで動作しない**
- ローカル: `cargo check`, `cargo clippy` で検証のみ
- サーバービルド: deploy.sh が自動で `cargo build --release` を実行

---

## 同居サービス (ポート競合注意)

| サービス | ポート | サーバー |
|---------|--------|---------|
| eat結 | 3000, 8080 | 192.168.3.242 |
| sorapiapps | - | 192.168.3.241 |
| LacisProxyGateway | 8081 (default) | 192.168.3.242 |

---

## 参照

| カテゴリ | パス |
|---------|------|
| プロジェクト仕様 | `../LacisProxyGateway2.md` |
| ルール詳細 | `.claude/rules/`, `../docs/GOLDEN_RULES.md` |
| ローカル認証情報 | `CLAUDE.local.md` (gitignore済み) |
