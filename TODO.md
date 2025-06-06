# macOS クリップボード管理アプリ開発 TODO

## プロジェクト概要
プログラマー向けの高機能クリップボード管理アプリをTauriで開発

## 機能要件
- クリップボード履歴管理（50件）
- ブックマーク機能（永続保存）
- IP履歴管理（xxx.xxx.xxx.xxx形式、10件）
- ホットキー操作（Cmd+Shift+V）
- JSON形式でのデータ保存
- メニューバー常駐、Dock非表示

## 開発フェーズ

### Phase 1: プロジェクトセットアップ
- [x] Tauriプロジェクト初期化
- [x] 必要な依存関係のインストール
- [x] 基本的な設定ファイル作成
- [x] macOS権限設定の準備

### Phase 2: バックエンド基盤
- [x] クリップボード監視システムの実装
- [x] データ構造の定義（JSON）
- [x] ファイル保存・読み込み機能
- [x] IP アドレス検出ロジック

### Phase 3: コア機能実装
- [ ] クリップボード履歴管理（50件制限）
- [ ] ブックマーク機能（CRUD操作）
- [ ] IP履歴管理（10件制限）
- [ ] 重複排除機能

### Phase 4: macOS システム統合
- [ ] グローバルホットキー（Cmd+Shift+V）
- [ ] メニューバーアイコン・メニュー
- [ ] Dock非表示設定
- [ ] アクセシビリティ権限チェック

### Phase 5: フロントエンド UI
- [ ] メイン履歴ウィンドウ
- [ ] ブックマーク管理画面
- [ ] IP履歴表示
- [ ] 設定画面
- [ ] 検索・フィルタ機能

### Phase 6: UX改善
- [ ] キーボードナビゲーション
- [ ] プレビュー表示（テキスト・画像）
- [ ] 使用頻度ベースのソート
- [ ] ダークモード対応

### Phase 7: 最適化・仕上げ
- [ ] パフォーマンス最適化
- [ ] メモリ使用量の最適化
- [ ] エラーハンドリング
- [ ] ログ機能

## 技術スタック
- **フレームワーク**: Tauri
- **フロントエンド**: HTML/CSS/JavaScript (または React/Vue)
- **バックエンド**: Rust
- **データ保存**: JSON
- **macOS API**: NSPasteboard, GlobalHotKey

## データ構造案
```json
{
  "version": "1.0.0",
  "history": [
    {
      "id": "uuid",
      "content": "text/image/path",
      "type": "text|image|file",
      "timestamp": "2025-01-01T00:00:00Z",
      "size": 1024
    }
  ],
  "bookmarks": [
    {
      "id": "uuid",
      "name": "Git Token",
      "content": "ghp_xxxx",
      "type": "text",
      "timestamp": "2025-01-01T00:00:00Z",
      "tags": ["git", "token"]
    }
  ],
  "recent_ips": [
    {
      "ip": "192.168.1.1",
      "timestamp": "2025-01-01T00:00:00Z",
      "count": 3
    }
  ],
  "settings": {
    "hotkey": "cmd+shift+v",
    "history_limit": 50,
    "ip_limit": 10
  }
}
```

## 実装優先度
1. **High**: Phase 1-3 (基本機能)
2. **Medium**: Phase 4-5 (システム統合・UI)
3. **Low**: Phase 6-7 (UX改善・最適化)