#!/usr/bin/env python3
"""create-pr スキルの下ごしらえ。ref の検証と解決、差分の取得、「変更の種類」の判定をする。

    python3 .claude/skills/create-pr/prepare.py                     # base = 既定ブランチ, head = カレント
    python3 .claude/skills/create-pr/prepare.py --base dev --head feature/foo
    python3 .claude/skills/create-pr/prepare.py --worktree          # 未 push の作業 (手順 1) を判定する
    python3 .claude/skills/create-pr/prepare.py --self-test         # 判定規則の自己診断 (git も gh も呼ばない)

結果は JSON で標準出力へ、中断の理由は標準エラーへ出す。中断したときの終了コードは 1。

git と gh は引数リストで直接起動し、シェルを通さない。ref 名がシェルに解釈されることも、
zsh が `$BASE_REF:r` のような修飾子を展開することも無い。依存は Python 3 標準ライブラリのみ。
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from fnmatch import fnmatchcase

# ref 名として受け付ける集合。gh --head にもファイル名 slug にもそのまま使える文字だけに絞る。
# 先頭の `-` はオプションと取り違えられるので弾く。
REF_PATTERN = re.compile(r"^(?!-)[A-Za-z0-9._/-]+$")

# アプリの挙動を変えるパス。ここに当たる変更が無ければ、コミット件名に fix / feat と
# 書いてあってもバグ修正・新機能・リファクタリングにはしない (スキルやドキュメントの
# 手入れを「新機能」と誤分類しないため)。cargo のチェックを走らせるかもこれで決まる。
CODE_PATHS = [
    "src/*",
    "stationapi/src/*",
    "preprocessor/src/*",
    "data_validator/src/*",
    "tools/*",
    "build.rs",
    "schema/*",
    "Cargo.toml",
    "*/Cargo.toml",
    "Cargo.lock",
    "wrangler.jsonc",
]
DATA_PATHS = ["data/*.csv"]
DOC_PATHS = ["*.md", "docs/*", "README*", ".claude/*"]
CI_PATHS = [".github/workflows/*", ".github/*.yml", ".github/*.yaml", "Makefile"]

# コミット件名のトリガ語句。英字は単語として一致したときだけ数える (`cd` が `line_cd` に、
# `data` が `data_validator` に当たらないように)。語尾の s / es / ed / d / ing は許す。
# 日本語は部分一致。
CODE_SUBJECT_TRIGGERS = {
    "バグ修正": ["fix", "hotfix", "バグ", "修正", "不具合"],
    "新機能": ["feat", "add", "新機能", "追加", "導入", "対応"],
    "リファクタリング": ["refactor", "リファクタ", "整理", "clean", "tidy"],
}
DOC_SUBJECT_TRIGGERS = ["docs", "ドキュメント", "readme", "changelog", "agents", "contributing"]
CI_SUBJECT_TRIGGERS = ["ci", "cd", "workflow", "release", "bump version", "labeler"]

# .github/pull_request_template.md の並び順。
LABELS = ["バグ修正", "新機能", "データの修正・追加", "リファクタリング", "ドキュメント", "CI/CD", "その他"]


class Abort(Exception):
    """ユーザーに確認してから進めるべき状態。メッセージをそのまま伝える。"""


# --------------------------------------------------------------------------- 判定


def matches(path: str, patterns: list[str]) -> bool:
    # fnmatch の `*` は `/` もまたぐので、`src/*` は src 以下すべてに当たる。
    return any(fnmatchcase(path, p) for p in patterns)


def find_trigger(subject: str, triggers: list[str]) -> str | None:
    for token in triggers:
        if token.isascii():
            pattern = rf"(?<![A-Za-z0-9_]){re.escape(token)}(?:s|es|ed|d|ing)?(?![A-Za-z0-9_])"
            if re.search(pattern, subject, re.IGNORECASE):
                return token
        elif token in subject:
            return token
    return None


def subject_hits(subjects: list[str], triggers: list[str]) -> list[str]:
    hits = []
    for subject in subjects:
        token = find_trigger(subject, triggers)
        if token:
            hits.append(f"コミット「{subject}」の `{token}`")
    return hits


def classify(subjects: list[str], files: list[str]) -> dict:
    code_files = [f for f in files if matches(f, CODE_PATHS)]
    data_files = [f for f in files if matches(f, DATA_PATHS)]
    doc_files = [f for f in files if matches(f, DOC_PATHS)]
    ci_files = [f for f in files if matches(f, CI_PATHS)]

    reasons: dict[str, list[str]] = {label: [] for label in LABELS}

    if code_files:
        for label, triggers in CODE_SUBJECT_TRIGGERS.items():
            reasons[label] = subject_hits(subjects, triggers)

    reasons["データの修正・追加"] = [f"`{f}` の変更" for f in data_files]

    # コードや CSV と混ざった PR は、その主目的の欄で表す。
    if not code_files and not data_files and doc_files:
        if len(doc_files) == len(files):
            reasons["ドキュメント"] = ["変更がドキュメントだけ"]
        else:
            reasons["ドキュメント"] = subject_hits(subjects, DOC_SUBJECT_TRIGGERS)

    reasons["CI/CD"] = [f"`{f}` の変更" for f in ci_files] + subject_hits(subjects, CI_SUBJECT_TRIGGERS)

    if not any(reasons[label] for label in LABELS[:-1]):
        reasons["その他"] = ["どの項目にも当たらない"]

    types = [{"label": label, "checked": bool(reasons[label]), "reasons": reasons[label]} for label in LABELS]
    return {
        "code_changed": bool(code_files),
        "data_changed": bool(data_files),
        "types": types,
        "checklist": "\n".join(f"- [{'x' if t['checked'] else ' '}] {t['label']}" for t in types),
    }


# --------------------------------------------------------------------------- git / gh


def run(*argv: str) -> str:
    result = subprocess.run(argv, capture_output=True, text=True)
    if result.returncode != 0:
        raise Abort(f"`{' '.join(argv)}` が失敗した:\n{result.stderr.strip()}")
    return result.stdout


def lines(text: str) -> list[str]:
    return [line for line in text.splitlines() if line]


def validate_ref(name: str, role: str) -> None:
    if not REF_PATTERN.match(name):
        raise Abort(f"{role} の ref 名に想定外の文字が含まれる: {name!r}。正しいブランチ名をユーザーに確認する")


def rev(ref: str) -> str | None:
    # 短縮名だと同名のタグやローカルブランチが優先されるので、完全形で解決する。
    result = subprocess.run(["git", "rev-parse", "--verify", "--quiet", ref], capture_output=True, text=True)
    return result.stdout.strip() or None


def default_base() -> str:
    return run("gh", "repo", "view", "--json", "defaultBranchRef", "-q", ".defaultBranchRef.name").strip()


def fetch(*branches: str) -> None:
    # ブランチ名だけを渡すと remote-tracking ref の更新が remote.origin.fetch の設定に
    # 左右されるので、refspec で明示する。
    run("git", "fetch", "origin", *(f"+refs/heads/{b}:refs/remotes/origin/{b}" for b in branches))


def prepare_range(base: str, head: str) -> dict:
    validate_ref(base, "base")
    if head == "HEAD" or head == base:
        raise Abort(f"PR の head にできるブランチに居ない ({head})。手順 1 でブランチを切り出す")
    validate_ref(head, "head")

    # 解決は必ず fetch の後。先に解決すると古い commit で差分を測る
    fetch(base)
    try:
        fetch(head)
    except Abort:
        raise Abort(f"origin に {head} が無い。push の可否をユーザーに確認する") from None
    base_rev = rev(f"refs/remotes/origin/{base}")
    head_rev = rev(f"refs/remotes/origin/{head}")
    if not base_rev or not head_rev:
        raise Abort(f"fetch したはずの origin/{base} か origin/{head} を解決できない")

    local_rev = rev(f"refs/heads/{head}")
    if not local_rev:
        raise Abort(f"ローカルに {head} が無いので未 push のコミットを検証できない。"
                    f"`git switch --track origin/{head}` で取り込んでからやり直す")
    if local_rev != head_rev:
        raise Abort(f"ローカルの {head} が origin と一致しない (local {local_rev} / origin {head_rev})。"
                    "未 push のコミットがあるので、push の可否をユーザーに確認する")

    subjects = lines(run("git", "log", "--pretty=%s", f"{base_rev}..{head_rev}"))
    files = lines(run("git", "diff", "--name-only", base_rev, head_rev))
    # コミットだけを見ると空コミットのブランチが通るので、ファイル差分も確かめる。
    if not subjects or not files:
        raise Abort("PR 対象の差分が無い")

    prs = json.loads(run("gh", "pr", "list", "--base", base, "--head", head, "--state", "open",
                         "--json", "number,url,body"))
    return {
        "base": base, "head": head, "base_rev": base_rev, "head_rev": head_rev,
        "commits": subjects, "files": files,
        "existing_pr": prs[0] if prs else None,
        **classify(subjects, files),
    }


def prepare_worktree(base: str) -> dict:
    """手順 1 用。origin の base から作業ツリーまでの変更 (未コミット・未追跡・未 push を含む) を判定する。"""
    validate_ref(base, "base")
    fetch(base)
    base_ref = f"refs/remotes/origin/{base}"
    files = sorted(set(lines(run("git", "diff", "--name-only", base_ref)))
                   | set(lines(run("git", "ls-files", "--others", "--exclude-standard"))))
    subjects = lines(run("git", "log", "--pretty=%s", f"{base_ref}..HEAD"))
    if not files:
        raise Abort("PR 対象の差分が無い")
    return {
        "base": base, "branch": run("git", "rev-parse", "--abbrev-ref", "HEAD").strip(),
        "commits": subjects, "files": files,
        **classify(subjects, files),
    }


# --------------------------------------------------------------------------- 自己診断

# (説明, コミット件名, 変更ファイル, ON になるべき項目, code_changed)
_CASES = [
    ("Worker 本体だけの変更はコード変更として数える",
     ["近傍バス停の絞り込みを件数の上限より先に行う"],
     ["src/index.rs", "src/repository.rs", "docs/nearby-bus-stops.md"], {"その他"}, True),
    ("識別子の一部 (line_cd / data_validator) には CI/CD もデータも当たらない",
     ["feat(data_validator): line_cd が 2!lines.csv に存在するか検証する"],
     ["data_validator/src/main.rs"], {"新機能"}, True),
    ("ドキュメントだけなら fix と書いてあってもバグ修正にしない",
     ["fix: スキルの誤記を直す"], [".claude/skills/create-pr/SKILL.md", "AGENTS.md"], {"ドキュメント"}, False),
    ("gRPC の中の RPC やカタカナには当たらない",
     ["エージェント向けガイドとスキルに残っていたgRPC時代の記述と誤った参照を直した"],
     ["AGENTS.md", "CONTRIBUTING.md"], {"ドキュメント"}, False),
    ("CSV の変更はデータ",
     ["都営大江戸線 都庁前のstation_g_cd統一とe_sortの連番化"], ["data/3!stations.csv"],
     {"データの修正・追加"}, False),
    ("data/README.md だけならドキュメント",
     ["data/README.md の表を更新"], ["data/README.md"], {"ドキュメント"}, False),
    ("日本語に続く CI も語として数える",
     ["CIでWorkerのユニットテストを実行する"], [".github/workflows/ci.yml"], {"CI/CD"}, False),
    ("composite action の変更は CI/CD",
     ["build-worker の既定値を揃える"], [".github/actions/build-worker/action.yml"], {"CI/CD"}, False),
    ("コードとドキュメントが混ざればドキュメントは付けない",
     ["駅番号の照合漏れを修正", "README を更新"],
     ["stationapi/src/use_case/interactor/query.rs", "README.md"], {"バグ修正"}, True),
    ("語尾の活用は数える",
     ["Added sortBy to connectedRoutes"], ["src/graphql/enums.rs"], {"新機能"}, True),
    ("依存更新はコード変更だが種類はその他",
     ["依存を上げる"], ["Cargo.lock", "preprocessor/Cargo.toml"], {"その他"}, True),
]

_REF_CASES = [("feature/foo-bar", True), ("release/v1.2.0", True), ("a;b", False),
              ("$(x)", False), ("-x", False), ("", False), ("日本語", False)]


def self_test() -> int:
    failures = 0
    for label, subjects, files, expected, code_changed in _CASES:
        result = classify(subjects, files)
        got = {t["label"] for t in result["types"] if t["checked"]}
        ok = got == expected and result["code_changed"] == code_changed
        failures += not ok
        print(f"  {'ok  ' if ok else 'FAIL'}  {label}"
              + ("" if ok else f"\n        期待 {sorted(expected)} code={code_changed}"
                               f" / 実際 {sorted(got)} code={result['code_changed']}"),
              file=sys.stderr)
    for name, valid in _REF_CASES:
        ok = bool(REF_PATTERN.match(name)) == valid
        failures += not ok
        print(f"  {'ok  ' if ok else 'FAIL'}  ref {name!r} を{'受け付ける' if valid else '弾く'}", file=sys.stderr)
    total = len(_CASES) + len(_REF_CASES)
    print(f"全 {total} 件中 {total - failures} 件 ok", file=sys.stderr)
    return 1 if failures else 0


# --------------------------------------------------------------------------- main


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--base", help="PR の base (既定はリポジトリの既定ブランチ)")
    parser.add_argument("--head", help="PR の head (既定はカレントブランチ)")
    parser.add_argument("--worktree", action="store_true",
                        help="origin の base から作業ツリーまでの変更を判定する (手順 1)")
    parser.add_argument("--self-test", action="store_true", help="判定規則の自己診断だけ走らせる")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    try:
        base = args.base or default_base()
        if args.worktree:
            result = prepare_worktree(base)
        else:
            head = args.head or run("git", "rev-parse", "--abbrev-ref", "HEAD").strip()
            result = prepare_range(base, head)
    except Abort as e:
        print(e, file=sys.stderr)
        return 1

    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
