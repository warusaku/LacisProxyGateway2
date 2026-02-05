# Security Rules

- Never print or commit secrets.
- Put sensitive info in `CLAUDE.local.md` only.
- Avoid insecure workarounds.

## 機密情報の取り扱い

### 絶対に含めてはいけないもの
- パスワード（平文）
- APIキー、シークレット
- プライベートキー
- データベース認証情報
- Discord Webhook URL

### 安全な保管場所
- `CLAUDE.local.md` (gitignore済み)
- 環境変数
- サーバー上の設定ファイル（/etc/配下）

## If you must reference a secret

- Redact it: `<REDACTED>`
- Describe location: "See CLAUDE.local.md section X"

## セキュリティ実装ガイドライン

1. **入力検証**: 全ての外部入力を検証
2. **IPフィルタ**: ブラックリスト/ホワイトリスト機能
3. **レートリミット**: DDoS対策
4. **ログ記録**: セキュリティイベントを記録
5. **Discord通知**: 異常検知時に即時通知
