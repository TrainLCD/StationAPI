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
- `gh` CLI が認証済みで、Python 3 がある（`prepare.py` は標準ライブラリしか使わない）。
- `head` ブランチが origin に push 済み。未 push の場合はユーザーに push の可否を確認する（勝手に push しない）。未 push かどうかは `prepare.py` がローカルと origin の commit ID を突き合わせて検出する。
- **ref の検証・解決、差分の取得、変更の種類の判定は `prepare.py` に任せ、シェルで書き直さない。** `prepare.py` は git と gh をシェルを通さずに起動する。そのため `'` / `$( )` / `;` を含む ref 名が実行されることも、zsh が `"$BASE_REF:refs/..."` の `:r` を修飾子として展開して ref を壊すこともない。スキル本文に位置引数（`$` と数字）を書くと、Claude Code がスキルを読み込む時点で呼び出し時の引数に置き換えるので、シェル断片でも使わない。

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
   - コミット前に `python3 .claude/skills/create-pr/prepare.py --worktree` を実行し、origin の base から作業ツリーまでの変更（未コミット・未追跡・未 push を含む）を判定する。`code_changed` が true なら下記の品質チェックを通す（`CONTRIBUTING.md` ルール）:
     - `cargo fmt --all -- --check`
     - `make clippy`
     - `make test`
   - `data_changed` が true なら `cargo run -p data_validator` も流す。
   - push は新規ブランチなので安全だが、実行前にユーザーへ要約（ブランチ名・含めるファイル・コミットメッセージ案）を提示して承認を取る。

   以降の手順では推論後の head を使う。

2. **状態確認とモード決定（新規作成 / 更新）**

   ```bash
   python3 .claude/skills/create-pr/prepare.py    # --base / --head で上書きできる
   ```

   `prepare.py` は次の順に確かめる。途中で止まったら理由を標準エラーに出し、終了コード 1 で終わる。その場合は理由をユーザーに伝え、指示に従う。
   - base / head の ref 名が `^[A-Za-z0-9._/-]+$` に一致するか（先頭の `-` は不可）。head が `HEAD`（detached）や base と同じなら、手順 1 で切り出す。
   - 両ブランチを refspec で明示して fetch し、`refs/remotes/origin/<名前>` の完全形で解決する。refspec を省くと remote-tracking ref の更新が `remote.origin.fetch` の設定次第になり、短縮名だと同名のタグやローカルブランチが優先される。
   - ローカルの head が origin と一致するか。一致しない、あるいはローカルに無い場合は、未 push のコミットを検証できないので止まる。
   - コミットとファイル差分がどちらも空でないか（空コミットだけのブランチを通さない）。
   - 既存の open PR を探し、「変更の種類」を判定する（手順 3）。

   出力は JSON で、以降の手順は `base` / `head` / `commits` / `files` / `code_changed` / `data_changed` / `types` / `checklist` / `existing_pr` を使う。手順 5 のシェルでは `base` と `head` の値を `BASE_REF` / `HEAD_REF` に代入し、`"${BASE_REF}"` のように中括弧付きで引用する。
   - `existing_pr` が null: 新規作成モード。手順 5 で `gh pr create`。
   - `existing_pr` がある: 更新モード。既存本文（`existing_pr.body`）を最新差分で再生成し、手順 5 で `gh pr edit`。タイトルは既存を**原則尊重**（ユーザー推論より優先）。ただし手順 5 の整合性チェックで主題が大きくズレていると判断した場合のみ更新案を提示する。

3. **変更の種類を判定**

   判定は `prepare.py` の `classify()` が行い、`types`（項目ごとの ON / OFF と根拠）と `checklist`（テンプレ順のチェック欄）を返す。規則を変えるときは `prepare.py` を直して `--self-test` を回す。この文書に判定表を書き戻さない（二か所に置くと食い違う）。

   規則の骨子:
   - **判定はアプリの挙動やデータに対する変更かどうかで決める。** `CODE_PATHS`（Worker 本体の `src/`、`stationapi/src/`、`preprocessor/src/` など）に変更が無ければ、コミット件名に fix / feat とあってもバグ修正・新機能・リファクタリングは OFF にする。スキルやドキュメントの手入れを「新機能」と誤分類しないため。
   - バグ修正・新機能・リファクタリングはコミット件名のトリガ語句で決める。英字の語句は単語として一致したときだけ数えるので、`line_cd` の `cd` や `data_validator` の `data` には当たらない。
   - データの修正・追加は `data/**/*.csv` の変更で決める（`data/README.md` だけならドキュメント）。
   - ドキュメントは、コードも CSV も含まず、変更がドキュメントだけか、コミット件名がドキュメントを示すときに ON。CI/CD は `.github/` のワークフローと action、`Makefile` の変更、またはコミット件名で決める。
   - どれも OFF のときだけ「その他」を ON にする。

   判定が PR の実態と合わないと思ったら、手で書き換えずに根拠（`types[].reasons`）をユーザーに示して確認する。

4. **本文組み立て**

   `.github/pull_request_template.md` の節構成をそのまま使い、下の置換だけを行う。節の追加・削除は禁止。

   節は見出し（`## 概要` / `## 変更の種類` / `## 変更内容` / `## テスト` / `## 関連Issue` / `## スクリーンショット（任意）`）で区切られる。各節の内容を下のルールで決める。

   **新規作成モード**
   - 「概要」節: `summary` があれば挿入。無ければテンプレのコメントだけ残す。
   - 「変更の種類」節: `checklist` をそのまま使う（テンプレ通りの順序で並んでいる）。
   - 「変更内容」節: コミット件名と変更ファイルから短い箇条書きを生成。`summary` があればそれを優先。データのみの PR では追加・修正した路線・駅などを箇条書きで列挙すると親切。
   - 「テスト」節:
     - **判定基準: `code_changed` が false なら Step 1 の `cargo` チェックを省略したとみなし、3 項目すべて OFF**（`skip_checks` より優先）。本文末尾に「省略: コード変更なし」等の短い注記を残す。
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
   REF_SLUG="$(printf '%s' "${HEAD_REF}" \
     | tr -d '\r\n' \
     | tr -c 'A-Za-z0-9._-' '_' \
     | sed -E 's/_+/_/g; s/^_+//; s/_+$//' \
     | cut -c1-100)"
   REF_SLUG="${REF_SLUG:-pr}"
   BODY_FILE="/tmp/pr-body-${REF_SLUG}.md"
   (
     trap 'rm -f "$BODY_FILE"' EXIT INT TERM
     gh pr create \
       --base "${BASE_REF}" \
       --head "${HEAD_REF}" \
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
