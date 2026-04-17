# Hitotone - Agents Policy

このファイルは、AIエージェント（Claude Code, Gemini CLI, Cursor, etc.）が本リポジトリで作業をする際の原則とポインタを記述します。

## プロジェクト構造
- `src/`: フロントエンド (Vite, HTML, CSS, JS)
- `src-tauri/`: バックエンド (Rust, Tauri v2)
- `docs/adr/`: アーキテクチャの意思決定履歴

## 技術スタック・規約
- **Frontend**: Vanilla JS, Vanilla CSS. フレームワークの導入はADRで議論。
- **Lint/Format**: Biome を使用。コミット前に `npm run lint` が必須。
- **Backend**: Rust. `cargo fmt` と `cargo clippy` を遵守。

## 作業ワークフロー
1. **計画**: 変更前に計画を提示し、ユーザーの承認を得ること。
2. **ADR優先**: 重要な設計変更は `docs/adr/` に記録すること。
3. **自動テスト**: ミスが発生した場合は、再発防止のテストを追加すること。

## ポインタ
- 詳細な規約については [docs/adr/0002-adopt-harness-engineering-mvh.md](docs/adr/0002-adopt-harness-engineering-mvh.md) を参照。
