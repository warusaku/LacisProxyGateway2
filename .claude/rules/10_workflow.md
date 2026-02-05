# Workflow Rules

- Prefer small, reviewable changes (MECE, minimal diff).
- Before implementation: confirm current behavior.
- After implementation: run tests and report evidence.

## 開発フロー

1. **変更前**: 現状確認、影響範囲の特定
2. **実装**: 小さな単位でコミット
3. **テスト**: ローカルで型チェック・lint
4. **デプロイ**: deploy.sh でサーバーへ

## コミットルール

```bash
# 1. 変更をステージ
git add -A

# 2. コミット（変更内容を明確に）
git commit -m "feat: DDNS連携機能追加"

# 3. プッシュ
git push origin main

# 4. デプロイ
./scripts/deploy.sh all
```

## Reporting format

- 目的:
- 現状確認（根拠）:
- 変更点（MECE）:
- テスト（実行コマンド/結果）:
- 影響範囲:
- 未確定点:
