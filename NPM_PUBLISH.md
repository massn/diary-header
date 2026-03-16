# npm公開手順

このドキュメントでは、`diary-header`をnpmに公開する手順を説明します。

## 前提条件

1. [npmjs.com](https://www.npmjs.com/)のアカウントを持っていること
2. GitHubリポジトリへの書き込み権限があること

## ステップ1: 変更をコミット

```bash
# 作成したファイルをステージング
git add README.md package.json install.js .npmignore bin/ .github/

# コミット
git commit -m "feat: add npm package configuration for easy global installation

Add npm package support to allow users to install diary-header globally via \`npm install -g diary-header\`. This includes:

- package.json with binary wrapper configuration
- install.js script to download prebuilt binaries from GitHub Releases
- bin/diary-header.js Node.js wrapper to execute the Rust binary
- .npmignore to exclude unnecessary files from npm package
- GitHub Actions workflow to build cross-platform binaries and publish to npm
- Updated README.md with npm installation instructions

Users can now install with a simple \`npm install -g diary-header\` instead of requiring Rust/Cargo toolchain.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"

# プッシュ
git push origin main
```

## ステップ2: npmアクセストークンの取得

1. [npmjs.com](https://www.npmjs.com/)にログイン
2. アカウント設定 → Access Tokens → Generate New Token
3. トークンタイプは **"Automation"** を選択
4. トークンをコピー（一度しか表示されないので注意！）

## ステップ3: GitHubシークレットの設定

1. GitHubリポジトリ（https://github.com/massn/diary-header）を開く
2. Settings → Secrets and variables → Actions
3. "New repository secret"をクリック
4. 以下を入力：
   - Name: `NPM_TOKEN`
   - Secret: ステップ2でコピーしたトークン
5. "Add secret"をクリック

## ステップ4: リリースの作成

```bash
# バージョンタグを作成
git tag v0.1.0

# タグをプッシュ
git push origin v0.1.0
```

## ステップ5: 自動ビルドとリリースの確認

1. GitHubの Actions タブを開く（https://github.com/massn/diary-header/actions）
2. "Release" ワークフローが実行されていることを確認
3. 完了するまで待つ（10-15分程度）

ワークフローが完了すると：
- GitHub Releasesに各プラットフォーム用のバイナリがアップロードされる
- npmに自動的にパッケージが公開される

## ステップ6: 公開の確認

```bash
# npmでパッケージを検索
npm search diary-header

# または直接インストールして確認
npm install -g diary-header

# 実行テスト
diary-header --help
```

## トラブルシューティング

### ワークフローが失敗する場合

#### Linuxクロスコンパイルエラー
ARM64 Linux向けのビルドが失敗する場合、`.github/workflows/release.yml`を確認してください。

#### npm公開エラー
- `NPM_TOKEN`が正しく設定されているか確認
- npmアカウントに2FAが有効な場合、"Automation"トークンを使用していることを確認
- パッケージ名が既に使用されていないか確認（必要に応じて`package.json`の`name`を変更）

### パッケージ名が既に使用されている場合

```bash
# package.jsonのnameを変更
# 例: "diary-header" → "@massn/diary-header" または "diary-header-cli"
nano package.json

# 変更をコミット
git add package.json
git commit -m "chore: rename package to avoid npm naming conflict"
git push origin main

# 新しいタグで再度リリース
git tag v0.1.1
git push origin v0.1.1
```

## 今後のリリース手順

次回以降のリリースは簡単です：

```bash
# 1. package.jsonのバージョンを更新
nano package.json  # version を "0.1.1" → "0.1.2" など

# 2. Cargo.tomlのバージョンも更新
nano Cargo.toml    # version を "0.1.1" → "0.1.2" など

# 3. 変更をコミット
git add package.json Cargo.toml
git commit -m "chore: bump version to v0.1.2"
git push origin main

# 4. タグを作成してプッシュ
git tag v0.1.2
git push origin v0.1.2
```

以上で自動的にビルド＆公開されます。

## 参考リンク

- npm パッケージページ: https://www.npmjs.com/package/diary-header
- GitHub Releases: https://github.com/massn/diary-header/releases
- GitHub Actions: https://github.com/massn/diary-header/actions
