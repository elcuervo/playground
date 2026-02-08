# Rust WASM (Emscripten) プロジェクト

このプロジェクトは、Rustで書かれたコードをwasm32-unknown-emscriptenターゲットでコンパイルし、ブラウザで実行するデモです。

## 必要なもの

- Rust（rustup）
- Emscriptenツールチェーン

## セットアップ

### 1. Emscriptenターゲットの追加

```bash
rustup target add wasm32-unknown-emscripten
```

### 2. Emscripten SDKのインストール

```bash
# Emscripten SDKをクローン
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk

# 最新版をインストール
./emsdk install latest
./emsdk activate latest

# 環境変数を設定（セッションごとに実行）
source ./emsdk_env.sh
```

## ビルド方法

`.cargo/config.toml`にビルドオプションが設定されているため、以下のコマンドでビルドできます：

```bash
cargo build --target wasm32-unknown-emscripten --release
```

これにより、以下のファイルが生成されます：
- `playground.js` - Emscriptenが生成したJSグルーコード
- `playground.wasm` - コンパイルされたWebAssemblyバイナリ

## 実行方法

### ローカルサーバーを起動

```bash
# Pythonを使う場合
python3 -m http.server 8000

# またはnpmのserveを使う場合
npx serve .
```

### ブラウザで開く

http://localhost:8000 にアクセスして、ブラウザのコンソールを開くと、メッセージが表示されます。

## コードの説明

`src/main.rs` には2つの方法でコンソールにメッセージを出力しています：

1. **println!マクロ**: Emscriptenが自動的にconsole.logにリダイレクト
2. **emscripten_run_script**: JavaScriptコードを直接実行

## トラブルシューティング

### CORSエラーが発生する場合

ファイルを `file://` プロトコルで直接開くとCORSエラーが発生します。必ずローカルHTTPサーバーを使用してください。

### ビルドエラーが発生する場合

Emscripten環境変数が正しく設定されているか確認してください：

```bash
source /path/to/emsdk/emsdk_env.sh
```
