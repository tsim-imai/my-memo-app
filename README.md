# 📋 Clipboard Manager

プログラマー向けの高機能macOSクリップボード管理アプリケーション

![Tauri](https://img.shields.io/badge/Tauri-2.5.0-blue)
![Rust](https://img.shields.io/badge/Rust-1.75+-orange)
![macOS](https://img.shields.io/badge/macOS-Big%20Sur+-lightgrey)
![License](https://img.shields.io/badge/license-MIT-green)

## 🎯 概要

Clipboard Managerは、macOS専用に設計されたプログラマー向けの高機能クリップボード管理ツールです。Tauriフレームワークを使用してRustとJavaScriptで開発され、軽量でありながら強力な機能を提供します。

### ✨ 主な特徴

- **📋 スマートクリップボード履歴**: 最大50件のクリップボード履歴を自動管理
- **⭐ 永続ブックマーク**: 重要なコード、トークン、設定を永続保存
- **🌐 IP履歴管理**: IPアドレスを自動検出・管理（最大10件）
- **⚡ グローバルホットキー**: `Cmd+Shift+V`でどこからでもアクセス
- **🎨 モダンUI**: ダークモード対応のレスポンシブデザイン
- **⌨️ 完全キーボード操作**: マウス不要の効率的な操作
- **🔍 高速検索**: リアルタイム検索とスマートフィルタ
- **📊 使用頻度追跡**: よく使うアイテムを学習・優先表示

## 🚀 セットアップ

### 前提条件

- macOS Big Sur (11.0) 以降
- Node.js 18+ 
- Rust 1.75+
- Xcode Command Line Tools

### インストール

1. **リポジトリをクローン**
```bash
git clone https://github.com/your-username/my-memo-app.git
cd my-memo-app
```

2. **依存関係をインストール**
```bash
# フロントエンド依存関係
npm install

# Rust環境の確認
rustc --version
```

3. **開発モードで起動**
```bash
npm run tauri dev
```

4. **本番ビルド**
```bash
npm run tauri build
```

### アクセシビリティ権限の設定

アプリの初回起動時に、macOSのアクセシビリティ権限が必要です：

1. システム環境設定 > セキュリティとプライバシー
2. プライバシータブ > アクセシビリティ
3. 🔒をクリックしてパスワード入力
4. "Clipboard Manager"にチェックを入れる

## 🎮 使用方法

### 基本操作

| 機能 | 操作方法 |
|------|----------|
| **アプリ表示** | `Cmd+Shift+V` |
| **タブ切り替え** | `Cmd+1-4` |
| **検索** | `Cmd+F` |
| **アイテム選択** | `↑` `↓` |
| **コピー** | `Enter` / `Space` / `Cmd+C` |
| **削除** | `Delete` |
| **ヘルプ** | `?` |

### 主要機能

#### 📋 クリップボード履歴
- 自動的にクリップボードの変更を監視
- 最大50件まで履歴を保持
- 重複アイテムの自動除去
- サイズ・タイムスタンプ・使用頻度を記録

#### ⭐ ブックマーク機能
- よく使うコード片やトークンを永続保存
- タグ機能で分類・整理
- 名前とコンテンツで検索可能
- 編集・複製・削除操作

#### 🌐 IP履歴管理
- テキスト内のIPアドレスを自動検出
- 検出パターン: `xxx.xxx.xxx.xxx`
- アクセス回数を自動カウント
- 最大10件まで保持

#### 🔍 高度な検索・ソート
- **検索**: リアルタイム文字列検索
- **ソート**: 最新順・頻度順・アルファベット順
- **フィルタ**: コンテンツタイプ別表示

## 🏗️ アーキテクチャ

### 技術スタック

```
Frontend (Vite + Vanilla JS)
├── HTML/CSS/JavaScript
├── リアルタイムUI更新
└── レスポンシブデザイン

Backend (Rust + Tauri)
├── クリップボード監視
├── データ永続化 (JSON)
├── システム統合
└── パフォーマンス最適化

macOS Integration
├── NSPasteboard API
├── Global Hotkeys
├── Menu Bar Integration
└── Accessibility APIs
```

### データ構造

```json
{
  "version": "1.0.0",
  "history": [
    {
      "id": "uuid",
      "content": "クリップボードの内容",
      "content_type": "text",
      "timestamp": "2025-01-01T00:00:00Z",
      "size": 1024,
      "access_count": 5,
      "last_accessed": "2025-01-01T12:00:00Z"
    }
  ],
  "bookmarks": [
    {
      "id": "uuid", 
      "name": "Git Token",
      "content": "ghp_xxxxxxxxxxxx",
      "content_type": "text",
      "timestamp": "2025-01-01T00:00:00Z",
      "tags": ["git", "token"],
      "access_count": 3,
      "last_accessed": "2025-01-01T10:00:00Z"
    }
  ],
  "recent_ips": [
    {
      "ip": "192.168.1.1",
      "timestamp": "2025-01-01T00:00:00Z", 
      "count": 7
    }
  ],
  "settings": {
    "hotkey": "cmd+shift+v",
    "history_limit": 50,
    "ip_limit": 10,
    "auto_start": true,
    "show_notifications": false
  }
}
```

## ⚡ パフォーマンス最適化

### バックエンド最適化
- **ハッシュベース変更検出**: 不要な処理を削減
- **アダプティブ監視**: エラー状況に応じて監視間隔を調整
- **メモリ効率化**: 大容量・古いアイテムの自動クリーンアップ
- **アトミック書き込み**: データ破損を防ぐ安全なファイル操作

### フロントエンド最適化
- **バーチャルスクロール**: 大量データの高速描画
- **デバウンス検索**: リアルタイム検索の最適化
- **レイジーローディング**: 必要時のみデータ読み込み

## 🛠️ 開発・デバッグ

### 開発コマンド

```bash
# 開発サーバー起動
npm run dev

# Tauriアプリ開発モード
npm run tauri dev

# ビルド
npm run build
npm run tauri build

# 依存関係更新
npm update
cargo update
```

### ログとデバッグ

アプリ内で以下の診断機能を利用できます：

- **📄 ログ表示**: リアルタイムログ確認
- **🔍 システム診断**: メモリ・ディスク使用量、パフォーマンス指標
- **🧹 メモリ最適化**: 手動クリーンアップ実行
- **📊 統計情報**: アイテム数、サイズ、使用率

## 📱 システム要件

| 項目 | 要件 |
|------|------|
| **OS** | macOS Big Sur (11.0) 以降 |
| **RAM** | 最小 4GB、推奨 8GB+ |
| **ディスク** | 100MB 空き容量 |
| **権限** | アクセシビリティ権限必須 |

## 🤝 コントリビューション

1. フォークを作成
2. フィーチャーブランチを作成 (`git checkout -b feature/amazing-feature`)
3. 変更をコミット (`git commit -m 'Add amazing feature'`)
4. ブランチにプッシュ (`git push origin feature/amazing-feature`)
5. プルリクエストを作成

### 開発ガイドライン

- Rustコードは`cargo fmt`でフォーマット
- JavaScriptコードはESLint準拠
- コミットメッセージは[Conventional Commits](https://conventionalcommits.org/)形式
- 新機能には適切なテストを追加

## 📄 ライセンス

MIT License - 詳細は[LICENSE](LICENSE)ファイルを参照

## 🙏 謝辞

- [Tauri](https://tauri.app/) - クロスプラットフォームアプリフレームワーク
- [clipboard-rs](https://github.com/DoumanAsh/clipboard-master) - クリップボードアクセス
- [Claude Code](https://claude.ai/code) - AI支援開発環境

## 📞 サポート

- 🐛 **バグレポート**: [Issues](https://github.com/your-username/my-memo-app/issues)
- 💡 **機能リクエスト**: [Discussions](https://github.com/your-username/my-memo-app/discussions) 
- 📧 **連絡先**: your-email@example.com

---

**Made with ❤️ for macOS developers**