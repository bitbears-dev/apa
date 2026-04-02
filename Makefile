.PHONY: all build release fmt lint test clean install

# デフォルトターゲットは開発用ビルド
all: build

# 開発用ビルド
build:
	cargo build

# リリース版バイナリのビルド
release:
	cargo build --release

# コードのフォーマット
fmt:
	cargo fmt

# Lintの実行 (警告をエラーとして扱う場合は `-- -D warnings` などを付与可能)
lint:
	cargo clippy

# テストの実行
test:
	cargo test

# ビルド生成物のクリーンアップ
clean:
	cargo clean

# システム全体のPATHが通っている場所（例: ~/.cargo/bin）にインストール
install:
	cargo install --path .
