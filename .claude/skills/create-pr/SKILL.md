---
name: create-pr
description: Create a GitHub pull request for TrainLCD StationAPI that conforms to .github/pull_request_template.md, assigns @TinyKitten, and auto-checks the 変更の種類 boxes based on the commit/file diff. Use whenever the user asks to open a PR in this repo.
---

# create-pr

このリポジトリの PR 作成手順を一本化したスキル。`.github/pull_request_template.md` を厳守し、Assignee・変更の種類・テスト欄を自動で組み立てる。

## 入力（呼び出し元が指定）

すべて任意。未指定なら下の既定値・推論で埋める。推論結果に不安があるとき（例: 多数のコミットで方向性がバラバラ）はユーザーに確認してから進める。

| 項目 | 既定値 / 推論元 |
| ---- | ---- |
| `base` | リポジトリの既定ブランチ（`gh repo view --json defaultBranchRef -q .defaultBranchRef.name`、StationAPI では通常 `dev`） |
| `head` | カレントブランチ（`git rev-parse --abbrev-ref HEAD`）。出力が `HEAD`（detached HEAD）のとき、または `base` と同じブランチのときは自動選択せず、手順 1 で切り出す |
| `title` | 下の「タイトル推論ルール」参照 |
| `summary` | 空なら「概要」「変更内容」本文はテンプレのコメントのみ残す |
| `related_issue` | **ユーザー入力を最優先**。指定が `#N`（数値のみ）なら `Closes #N`、`Closes #N` / `Fixes #N` / `Refs #N` 形式ならその接頭語を保って出力。`related_issue` が空のときに限り、コミット件名から `Closes #N` / `Fixes #N` / `Refs #N` を抽出（接頭語を維持。`#N` 単体表記なら `Closes` を補う）。両方とも見つからなければ節のコメントのみ |
| `skip_checks` | `false`（PR本文「テスト」節のチェック欄 3 項目を ON）。`true` なら全 OFF。**本文表示のみを制御するフラグで、Step 1 の `cargo fmt` / `clippy` / `test` の実際の実行は保証しない**（実行可否は「コードに変更があるか」で決める）。**Step 1 で `cargo` チェックを実行していない（=コード／データ変更なし）ケースでは、`skip_checks` の値に関わらず 3 項目すべて OFF にする** |
| `labels` | 文字列配列、または未指定。**通常は未指定で OK**（`.github/workflows/pr_labeler.yml` がブランチ名と変更ファイルから自動付与する）。手動指定が必要な場合は `gh pr create --label <name>` で渡す（作成後に `gh pr edit --add-label` すると `pull_request: opened` トリガのワークフローに間に合わないため、必ず `gh pr create` 時に渡す） |

### タイトル推論ルール

`origin/<base>..origin/<head>` のコミット件名を対象に、以下を順に試す:

1. **コミット 1 件のみ**: その件名をそのまま使う。
2. **コミット複数・共通テーマあり**: 最新コミットの件名、もしくは件名群を要約した日本語の単文を使う。
3. **ブランチ名が `feature/` / `fix/` / `data/` / `chore/` / `release/` で始まる**: プレフィックスを取り除き、残りの `kebab-case` を日本語や自然文に整える。確信が持てないときは整形せずブランチ名のまま使ってよい。
4. **どれでも決まらない**: 最新コミット件名を採用し、「このタイトルで作成してよいか」をユーザーに確認する。

このリポジトリの直近 PR タイトル（例: `近鉄生駒鋼索線 鳥居前駅と近鉄生駒駅をグループ接続`、`京急本線 line_symbol2_colorのコードが違う`）に倣い、**簡潔な日本語の単文**に整形する。Conventional Commits プレフィックス（`fix:` `feat:` など）は基本的に剥がす。整形時は意味を変えないこと。

## 前提条件

- カレントディレクトリが `git rev-parse --show-toplevel` で解決できるリポジトリ内。
- `gh` CLI が認証済み。
- `head` ブランチが origin に push 済み。未 push の場合はユーザーに push の可否を確認する（勝手に push しない）。散文の前提で終わらせず、手順 2 でローカルと origin の commit ID を突き合わせて機械的に検出する。
- **ref 名をシェルソースへ直接埋め込まない。** 本書の `<base>` / `<head>` は説明用のプレースホルダ。実際のコマンドでは値を `BASE_REF` / `HEAD_REF` に取り込み、以降は必ず `"$BASE_REF"` / `"$HEAD_REF"` で参照する。git の ref 名は `'` / `$( )` / バッククォート / `;` を含められるため、リテラルを直接置換すると構文が壊れるか、意図しないコマンドが実行される。値はコマンド出力から取り込む（ユーザー指定がある場合のみ、その値を代入する）:

  ```bash
  BASE_REF="$(gh repo view --json defaultBranchRef -q .defaultBranchRef.name)"

  HEAD_REF="$(git rev-parse --abbrev-ref HEAD)"
  if [ "$HEAD_REF" = "HEAD" ] || [ "$HEAD_REF" = "$BASE_REF" ]; then
    printf 'PR の head にできるブランチに居ない: %s\n' "$HEAD_REF" >&2
    exit 1   # 手順 1 で切り出す
  fi

  # ref 名の文字種を検証する。gh --head・ファイル名 slug・シェル展開の
  # すべてで安全に使える集合か。BASE_REF / HEAD_REF に同じ規則を適用する
  validate_ref() {
    case "$1" in
      '' | *[!A-Za-z0-9._/-]*)
        printf 'ref 名に想定外の文字が含まれる: %s\n' "$1" >&2; return 1 ;;
    esac
  }
  validate_ref "$BASE_REF" || exit 1
  validate_ref "$HEAD_REF" || exit 1

  # origin 上のブランチを commit ID へ解決する。
  # 実際の解決は fetch 後（手順 2）に一度だけ行うので、ここでは定義のみ
  resolve_remote_rev() {
    validate_ref "$1" || return 1
    rev="$(git rev-parse --verify --quiet "refs/remotes/origin/$1")"
    if [ -z "$rev" ]; then
      printf 'origin 上に存在しない: %s\n' "$1" >&2; return 1
    fi
    printf '%s\n' "$rev"
  }
  ```

- **ref は `refs/remotes/origin/<名前>` / `refs/heads/<名前>` の完全形で解決する。** 短縮名を `git rev-parse` に渡すと、同名のタグやローカルブランチが優先されて意図と違うコミットを指すことがある。`--verify --quiet` を付けると、存在しない ref は終了コード 1 と空出力になるので、**解決結果が空でないことの検証が必須**。省くと未 push のブランチが空の `BASE_REV` / `HEAD_REV` として無言で通過する。
- **`BASE_REF` / `HEAD_REF` が `^[A-Za-z0-9._/-]+$` に一致しない場合は自動で進めない。** git の ref 名には `gh --head` やファイル名 slug に使えない文字、シェルに解釈される文字が入り得る。`gh repo view` 由来の `BASE_REF` にも同じ検査を適用し（`resolve_remote_rev` の `case` がこれを担う）、一致しない値が返ったらユーザーに正しいブランチ名を確認する。

## 手順

1. **head / base の整合性チェックと自動ブランチ切り出し**

   `base == head` になるケース（例: `dev` に居てデフォルト base も `dev`）や detached HEAD は、そのまま進めると PR が作れない。`git status` で作業ツリーの内容を確認し、以下のいずれかで救済する:

   - 作業中の変更（staged / unstaged / 直近の未 push コミット）がある場合、**新しいブランチを切ってそこに退避**してから続行する。
   - 何の変更も無い場合は「PR 対象の差分が無い」と報告して中断する。

   **ブランチ名の推論**（`CONTRIBUTING.md` の命名規則に従う）:

   | プレフィックス | 採用条件 |
   | ---- | ---- |
   | `fix/` | 変更内容や直近コミット件名にバグ修正・`fix`・`修正`・`不具合` を示唆する語がある |
   | `data/` | 変更が `data/**/*.csv` などデータ系のみ |
   | `chore/` | 依存更新（`Cargo.toml` / `Cargo.lock`）・ビルド設定など雑務のみ |
   | `release/` | リリース作業（バージョンバンプなど。ユーザーが明示した場合のみ） |
   | `feature/` | 上記いずれにも当たらない場合の既定（新機能・通常の改修） |

   命名規則は `pr-labeler.yml` のラベル自動付与にも連動するため、**プレフィックスは厳守**（`feat/` ではなく `feature/`、`docs/` は使わない）。slug は変更ファイル・コミット件名から短い英小文字 kebab-case を作る（例: `fix-line-symbol-color`、`data/keikyu-line-color`）。確信が持てない場合は slug 候補を 1〜2 個出してユーザーに確認。

   切り出し手順:
   ```bash
   git switch -c <inferred-branch>
   git status                       # 何をコミットするか必ず先に確認
   # 未コミットなら:
   git add -u                       # 追跡済みの staged/unstaged をまとめてステージ
   # 未追跡ファイルも退避対象なら明示的にパス指定で追加（`git add -A` / `.` は使わない）:
   #   git add path/to/untracked-file ...
   git commit -m "<日本語単文>"
   git push -u origin <inferred-branch>
   ```
   - **`git add -A` / `git add .` は使わない。** `.gitignore` に載っていない一時ファイルまで巻き込む。`git status` の出力を読んでから、追跡済みは `git add -u`、未追跡は明示パスで追加する。関係ないファイルが入ったら `git restore --staged <path>` で外す。
   - 変更が既にコミット済みでブランチだけが無い（`dev` の上に直接コミットした等）場合は、`git switch -c <inferred-branch>` だけでそのコミットを新ブランチへ引き継げる。`dev` 側を元に戻す必要があれば、**作業ツリーに触れない `git branch -f dev origin/dev`** を使う（実行の可否はユーザーに確認する）。`dev` が別の worktree で checkout 済みなら git 自身がこのコマンドを拒否するので、取り違えも起きない。`git switch dev && git reset --hard origin/dev` は避ける: 追跡済みファイルの staged／unstaged 変更を問答無用で捨てるうえ、`dev` を別の worktree が持っていると `git switch` 自体が失敗する。どうしても checkout して戻すなら、`git worktree list` で `dev` の所在を確認し、その worktree で `git status --short` が空であることを確かめてから実行する（変更があれば先に WIP コミット（推奨）か名前付き stash（`git stash push -u -m "<tag>"`。stash スタックは全 worktree 共有なので `git stash pop` ではなく `git stash apply <sha>` で戻す）へ退避する）。
   - コミット前に下記の品質チェックを通す（`CONTRIBUTING.md` ルール、手順 3 で定義する「コード本体パス」に変更が無ければ省略可）:
     - `cargo fmt --all -- --check`
     - `make clippy`
     - `make test`
   - データのみの変更（`data/*.csv` 等）を含む場合は `cargo run -p data_validator` も流す。
   - push は新規ブランチなので安全だが、実行前にユーザーへ要約（ブランチ名・含めるファイル・コミットメッセージ案）を提示して承認を取る。

   以降の手順では推論後の head を使う。

2. **状態確認とモード決定（新規作成 / 更新）**
   - `git fetch` で `BASE_REF` / `HEAD_REF` の remote-tracking ref を更新する。**`BASE_REV` / `HEAD_REV` の解決は必ず fetch の後に行う。** 先に解決すると古い commit ID で差分を測ることになる。前提条件で済ませておくのは ref 名の文字種検証までにとどめる。
   - **fetch 対象は refspec で明示する。** ブランチ名だけを渡す形（`git fetch origin <branch>`）は remote-tracking ref の更新が `remote.origin.fetch` の設定に依存する。別マシンで refspec が絞られていると `refs/remotes/origin/<branch>` が古いまま後続の解決と一致検査を通ってしまう。
   - **`BASE_REV` / `HEAD_REV` は origin 側の位置なので、ローカルの `HEAD_REF` がそれと一致することを機械的に確かめる。** 一致しなければ未 push のコミットがあり、そのまま進むとその分を含まない範囲で PR が組み上がる。検出したら push の可否をユーザーに確認して中断する（勝手に push しない）。
   - **ローカルに `HEAD_REF` が無い場合も中断する。** 空は「未 push が無い」証拠ではなく、単に検証できていない状態（ローカルで削除済み、あるいは origin にしか無いブランチを `head` に指定した、など）。`git switch --track "origin/<名前>"` でローカルへ取り込んでからやり直す。
   - コミットとファイル差分の**両方**を確認する。`git log` はコミットの有無しか見ないため、空コミットだけが載ったブランチが通過してしまう。

     ```bash
     # remote.origin.fetch の設定に左右されないよう refspec で明示する
     git fetch origin \
       "+refs/heads/$BASE_REF:refs/remotes/origin/$BASE_REF" \
       "+refs/heads/$HEAD_REF:refs/remotes/origin/$HEAD_REF"
     BASE_REV="$(resolve_remote_rev "$BASE_REF")" || exit 1   # 前提条件で定義したヘルパ。解決はここが最初
     HEAD_REV="$(resolve_remote_rev "$HEAD_REF")" || exit 1

     # ローカルの HEAD_REF を解決し、origin の先端と一致することを確認する。
     # HEAD_REF は validate_ref で検証済み
     HEAD_LOCAL_REV="$(git rev-parse --verify --quiet "refs/heads/$HEAD_REF")"
     # 該当が無いと空を返すので、中身で判定する
     if [ -z "$HEAD_LOCAL_REV" ]; then
       printf 'ローカルに %s が無い。origin だけを見て組むと一致を検証できない。\n' "$HEAD_REF" >&2
       printf 'git switch --track "origin/%s" で取り込んでからやり直す。\n' "$HEAD_REF" >&2
       exit 1
     fi
     if [ "$HEAD_LOCAL_REV" != "$HEAD_REV" ]; then
       printf 'ローカル %s が origin と一致しない:\n  local  = %s\n  origin = %s\n' \
         "$HEAD_REF" "$HEAD_LOCAL_REV" "$HEAD_REV" >&2
       exit 1   # 未 push の変更がある。push の可否をユーザーに確認してから進む
     fi

     git log --oneline "$BASE_REV..$HEAD_REV"
     git diff --name-only "$BASE_REV" "$HEAD_REV"
     ```

     コミット一覧が空、または `git diff --name-only` の出力が空の場合は「PR 対象の差分が無い」と報告し、**既存 PR の検索へ進まずに中断する**。
   - `gh pr list --base "$BASE_REF" --head "$HEAD_REF" --state open --json number,url,body` で既存 open PR を確認。
     - **存在しない場合**: 新規作成モード。以降、手順 5 で `gh pr create`。
     - **存在する場合**: 更新モード。既存本文を最新差分で再生成する。以降、手順 5 で `gh pr edit`。タイトルは既存を**原則尊重**（ユーザー推論より優先）。ただし手順 5 の整合性チェックで主題が大きくズレていると判断した場合のみ更新案を提示する。

3. **変更の種類を判定**

   `origin/<base>..origin/<head>` のコミット件名と変更ファイルを取得:
   ```bash
   git log --pretty=%s "$BASE_REV..$HEAD_REV"
   git diff --name-only "$BASE_REV" "$HEAD_REV"
   ```

   **大原則: 判定はアプリ挙動／データに対する変更かどうかで決める**。下の「コード本体パス」が一切変わっていない場合、「バグ修正」「新機能」「リファクタリング」は OFF（コミット件名に `fix` / `feat` 等の語があっても）。スキル・設定・ドキュメントのメタ変更を「新機能」と誤分類しないための安全弁。「データの修正・追加」は `data/**` の変更を独立に判定する（後述「変更ファイルパスベース」「コミット件名ベース」を参照）。

   この大原則のもとで、各項目を独立に評価（複数該当可、大文字小文字無視）。英字のトリガ語句は単語として一致したときだけ数える（`ci` / `cd` / `add` / `data` が `station_cd` や `line_cd` の一部に当たって誤判定しないように）。日本語のトリガ語句は部分一致でよい。

   **コード本体パス**（バグ修正 / 新機能 / リファクタリングのゲート）

   - `src/**`（Worker 本体）
   - `stationapi/src/**`
   - `preprocessor/src/**`
   - `data_validator/src/**`
   - `tools/**`
   - `build.rs`
   - `schema/**`
   - `Cargo.toml` / `Cargo.lock`
   - `wrangler.jsonc`

   **コード本体変更ありの場合 — コミット件名ベース**

   | 項目 | トリガ語句 |
   | ---- | ---- |
   | バグ修正 | `fix`, `Hotfix`, `バグ`, `修正`, `不具合` |
   | 新機能 | `feat`, `add`, `新機能`, `追加`, `導入`, `対応` |
   | リファクタリング | `refactor`, `リファクタ`, `整理`, `clean`, `tidy` |

   **変更ファイルパスベース**（コード本体変更の有無に関わらず評価）

   | 項目 | パターン |
   | ---- | ---- |
   | データの修正・追加 | `data/**/*.csv` |
   | ドキュメント | 変更が `*.md` / `docs/**` / `README*` / `.claude/**` / `AGENTS.md` / `CONTRIBUTING.md` のみ、またはそれらを主体とする |
   | CI/CD | `.github/workflows/**`, `.github/**/*.yml`, `Makefile` のいずれかを含む |

   **コミット件名ベース（データ・ドキュメント・CI/CD）**

   | 項目 | トリガ語句 |
   | ---- | ---- |
   | データの修正・追加 | `データ`, `data`, `駅`, `路線`, `numbering`, `CSV`, `csv` |
   | ドキュメント | `docs`, `ドキュメント`, `README`, `changelog`, `AGENTS`, `CONTRIBUTING` |
   | CI/CD | `ci`, `cd`, `workflow`, `release`, `Bump version`, `labeler` |

   判定ロジック:
   - 上の「大原則」のゲートをまず適用。コード本体／データの変更が無ければバグ修正・新機能・リファクタリング・データの修正・追加は強制 OFF。
   - 「データの修正・追加」は `data/**` の変更があれば ON。`data/README.md` のみの変更なら「ドキュメント」のみ ON にする。
   - 「ドキュメント」は変更にコード本体や CSV を含まない場合に ON。混在する場合は基本 OFF（主目的が分かるならそちらを優先）。ただし `.claude/**` や `AGENTS.md` のみの変更は「ドキュメント」を ON にする（運用ドキュメント扱い）。
   - 「CI/CD」は `.github/workflows/**` 等の変更があれば独立に ON。
   - 残りの項目は、コミット件名またはファイルパスのトリガに 1 つでも当てはまれば `- [x]`、それ以外は `- [ ]`。
   - 全項目が OFF のときのみ `その他` を `- [x]` にする。他項目が ON のときは `その他` は必ず `- [ ]`。

4. **本文組み立て**

   `.github/pull_request_template.md` の節構成をそのまま使い、下の置換だけを行う。節の追加・削除は禁止。

   節は見出し（`## 概要` / `## 変更の種類` / `## 変更内容` / `## テスト` / `## 関連Issue` / `## スクリーンショット（任意）`）で区切られる。各節の内容を下のルールで決める。

   **新規作成モード**
   - 「概要」節: `summary` があれば挿入。無ければテンプレのコメントだけ残す。
   - 「変更の種類」節: 手順 3 の結果で各 `- [ ]` / `- [x]` を決定。**項目順序は必ずテンプレ通り**（バグ修正 / 新機能 / データの修正・追加 / リファクタリング / ドキュメント / CI/CD / その他）。
   - 「変更内容」節: コミット件名と変更ファイルから短い箇条書きを生成。`summary` があればそれを優先。データのみの PR では追加・修正した路線・駅などを箇条書きで列挙すると親切。
   - 「テスト」節:
     - **判定基準: 手順 3 の「コード本体パス」（`stationapi/src/**` ほか）に変更が無い場合は Step 1 の `cargo` チェックを省略したとみなし、3 項目すべて OFF**（`skip_checks` より優先）。本文末尾に「省略: コード変更なし」等の短い注記を残す。
     - 上記に該当しない場合は `skip_checks` が真なら 3 項目すべて OFF、偽なら 3 項目すべて ON。テキストはテンプレのまま（`make fmt` / `make clippy` / `make test`）。
   - 「関連Issue」節: `related_issue` が指定されていればユーザー入力を最優先で出力（`#N` のみなら `Closes #N`、`Closes/Fixes/Refs #N` 形式なら接頭語を維持）。空のときに限りコミット件名から `Closes/Fixes/Refs #N` を抽出。どちらも無ければコメントのみ。
   - 「スクリーンショット」節: 常にコメントのみ（API レスポンスの diff など必要なら呼び出し側が後から編集する前提）。

   **更新モード**（既存 PR の本文を再生成）

   既存本文を節ごとに分割し、以下のルールで部分的に書き換える。人間が書き込んだ散文は壊さない。

   | 節 | 更新方針 |
   | ---- | ---- |
   | 概要 | 既存内容を尊重。空欄（テンプレのコメントのみ）なら新規作成モードと同じ生成を試みる。 |
   | 変更の種類 | **常に手順 3 の結果で上書き**（機械的判定）。 |
   | 変更内容 | 冒頭の箇条書きブロック（`-` で始まる連続行）を最新差分で再生成。その下に人間が書いた散文があれば残す。 |
   | テスト | **常に `skip_checks` に従う**（手順 4 の本文組み立てと同じルール）。 |
   | 関連Issue | 既存内容を尊重。コミット件名に `Closes/Fixes/Refs #N` があり、かつ既存本文中に同じ Issue 番号 `#N` を指す表現が存在しない場合のみ追記（重複は作らない。比較時は `Closes` / `closes` / `Fixes` / `fixes` / `Refs` / `refs` を同一視し、空白・記号差は無視して `#N` 単位で照合）。 |
   | スクリーンショット | 既存内容を尊重。自動では触らない。 |

   差し替え後の本文と既存本文の差分をユーザーに提示し、承認を得てから手順 5 へ進む。自動上書き節で人間の手入れらしき痕跡（テンプレのコメント以外の文章）がある場合は、どう扱うかをユーザーに確認する。

5. **PR 作成 / 更新**

   本文は **必ず一時ファイル経由で渡す**（`gh pr create --body-file` / `gh pr edit --body-file`）。理由: `--body "$(cat <<'EOF' ... EOF)"` のようにヒアドキュメントをシェル経由で渡すと、エディタ側の癖や Claude Code 側の生成で本文中のバッククォートが `\`` のように誤って escape されてしまい、PR 画面でコードスパン／フェンスがレンダリングされない事故が起きる。`--body-file` ならシェルの引用符を一切介さないので構造的に起きない。

   実装手順:

   1. Write ツールで本文を一時ファイルに書き出す（例: `/tmp/pr-body-<slug>.md`）。ファイル名に使う ref（ブランチ名・PR 番号など）は **ファイル名として安全な集合（`A-Za-z0-9._-`）にスラッグ化** する。具体的には:
      - `/`・改行・制御文字・空白・非 ASCII などを `_` に置換
      - 連続した `_` は 1 つに畳み、先頭・末尾の `_` は除去
      - 必要なら長さを 100〜200 文字程度に切り詰める

      生のブランチ名を直結するとサブディレクトリ解釈や制御文字混入で Write／削除が失敗する。バッククォートは **素のまま** 書く。escape しない。
   2. 下の `gh` コマンドをサブシェル内で `trap` と一緒に実行する。`gh` の成功・失敗に関わらず `EXIT` / `INT` / `TERM` のどれでも一時ファイルを確実に削除されるようにする（`&&` で `rm` を繋ぐだけだと失敗時に `/tmp` にゴミが残る）。
   3. `gh` 呼び出しと `rm`（を含む `trap`）は Bash tool の 1 呼び出し内で完結させる。別呼び出しで後片付けすると、前段の呼び出しがエラー／中断で終わった場合にクリーンアップが実行されない。

   **新規作成モード**

   ```bash
   # ref 名（ブランチ名）をファイル名として安全な集合（A-Za-z0-9._-）にスラッグ化
   REF_SLUG="$(printf '%s' "$HEAD_REF" \
     | tr -d '\r\n' \
     | tr -c 'A-Za-z0-9._-' '_' \
     | sed -E 's/_+/_/g; s/^_+//; s/_+$//' \
     | cut -c1-100)"
   REF_SLUG="${REF_SLUG:-pr}"
   BODY_FILE="/tmp/pr-body-${REF_SLUG}.md"
   (
     trap 'rm -f "$BODY_FILE"' EXIT INT TERM
     gh pr create \
       --base "$BASE_REF" \
       --head "$HEAD_REF" \
       --title "<title>" \
       --assignee TinyKitten \
       [--label "<label1>" --label "<label2>" ...] \
       --body-file "$BODY_FILE"
   )
   ```

   - Assignee は常に `TinyKitten`（`CODEOWNERS` で全パスのオーナー）。
   - `labels` 入力があれば、その要素数だけ `--label` を繰り返して渡す。未指定なら `--label` 自体を書かない（`pr_labeler.yml` が自動でラベルを付ける）。
   - 作成後の URL と、ON にしたチェック項目・判定根拠（例: コミット `fix: ...` により「バグ修正」を ON、`data/3!stations.csv` の変更により「データの修正・追加」を ON）、付与したラベルがあればその名前を報告する。

   **更新モード**

   ```bash
   BODY_FILE="/tmp/pr-body-${pr_number}.md"
   (
     trap 'rm -f "$BODY_FILE"' EXIT INT TERM
     gh pr edit <pr-number> \
       [--title "<更新後タイトル>"] \
       --body-file "$BODY_FILE"
   )
   ```

   - **タイトルは原則として既存を維持する**。ただし毎回スコープ整合性を再評価し、手順 1 のタイトル推論ルールと最新のコミット群を照合する。現タイトルが新しい主題（追加路線・大きな機能変更など）を拾えていない**重大な不整合**がある場合のみ、更新案を提示してユーザー承認を取り `--title` で上書きする。整合している、または軽微な差分にとどまる場合は `--title` を付けない。
   - Assignee は既に付いていれば再指定しない（重複操作を避ける）。付いてなければ `--add-assignee TinyKitten`。
   - 実行後、PR URL と「タイトルを変更したか・どの節を書き換えたか・変更の種類チェック差分」を簡潔に報告する。

## 注意事項

- テンプレの節構成は改変しない。追加・削除はメンテナ承認が必要。
- `git push --no-verify` や force push はしない。push が必要ならユーザーに確認。
- **push 済みのコミットを勝手に書き換えない。** `git commit --amend` / `git rebase` は履歴を書き換え、反映には force push が要る。必ずユーザーに確認する。
- 既存 open PR を上書きしない（重複作成禁止）。
- ブランチプレフィックスは `feature/` / `fix/` / `data/` / `chore/` / `release/` のみ使用（`pr-labeler.yml` のラベル自動付与に直結する）。
- 本文は `gh pr create --body` / `gh pr edit --body` のようにインラインで渡さない。必ず `--body-file` で一時ファイル経由で渡す（バッククォートなど特殊文字の escape 事故を構造的に防ぐため）。
- データ変更を伴う PR では `cargo run -p data_validator` の実行結果を「テスト」または「変更内容」節に追記すると `AGENTS.md` のガイドライン（変更内容と検証コマンドの記録）に沿う。
