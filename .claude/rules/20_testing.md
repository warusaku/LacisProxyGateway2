# Testing Rules

- No "done" without checks/tests.
- Provide the exact commands you ran.
- If a test cannot be run, explain why.

## Minimum evidence

### Backend (Rust)
```bash
# 型チェック
cargo check

# フォーマット確認
cargo fmt --check

# Lint
cargo clippy -- -D warnings

# ユニットテスト
cargo test

# 統合テスト
cargo test --test '*'
```

### Frontend (React)
```bash
# 型チェック
npm run type-check

# Lint
npm run lint

# テスト
npm run test
```

### デプロイ後確認

```bash
# サービス状態
ssh akihabara_admin@192.168.3.242 "systemctl status lacis-proxy-gateway"

# ログ確認
ssh akihabara_admin@192.168.3.242 "journalctl -u lacis-proxy-gateway -n 50"

# エンドポイント確認
curl -I http://192.168.3.242/LacisProxyGateway2
```
