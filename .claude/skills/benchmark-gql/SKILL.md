---
name: benchmark-gql
description: Benchmark the TrainLCD StationAPI production GraphQL endpoint (gql.trainlcd.app) against staging (gql-stg.trainlcd.app) across every query in schema/public.graphql, measuring both client-side response time and Cloudflare Worker CPU Time, and append the result as a Markdown report under benchmarks/. Use whenever the user asks to compare, benchmark, or profile the two endpoints, or to check the performance impact of a change before it reaches master.
---

# benchmark-gql

本番 (`https://gql.trainlcd.app` / Worker `stationapi`) とステージング
(`https://gql-stg.trainlcd.app` / Worker `stationapi-stg`) の GraphQL 性能を、
`schema/public.graphql` の全 Query フィールドについて比較する。

両環境は同じデータを積んでいる (`/__health` が返す駅数・路線数・会社数が一致する) ので、
出てくる差は実装差だけ。ステージングは `dev`、本番は `master` から出るので、
このベンチは「次のリリースで本番の性能がどう変わるか」を先に見るものになる。

## 何を測るか

| 指標 | 出どころ | 何が見えるか |
| --- | --- | --- |
| **CPU Time** | `wrangler tail --format json` の `cpuTime` | Worker が実際に計算に使った時間。ネットワークもコロの当たり外れも含まないので、**実装差はここに出る**。判定はこの列で行う |
| Worker wall time | 同 `wallTime` | Worker 内の実時間。この Worker は I/O をしないので CPU Time とほぼ一致し、乖離したら外部待ちが混ざったサイン |
| クライアント応答時間 | keep-alive を張った 1 本の接続での往復時間 | 利用者から見た体感。回線とエッジ処理と転送を含む |
| 応答サイズ | レスポンスのバイト数 | 転送時間の効き方を読むための補助 |

CPU Time はリクエストと `cf-ray` で突き合わせる。`wrangler tail` 側には
`--header x-stationapi-bench:<実行 ID>` を渡すので、本番に実ユーザーのトラフィックが
流れていてもこの実行のリクエストだけが降ってくる。

## 前提条件

- wrangler の認証が済んでいて、権限に **`workers_tail (read)`** があること。
  無いと CPU Time が欠測になる (応答時間の計測だけは続行する)。
  版は `Makefile` の `WRANGLER_VERSION` に合わせる。版を指定しない `npx wrangler` は
  その時点の最新を取ってくるので、Cloudflare の認証情報を持つ環境で走らせるものとしては
  固定しておく。`bench.py` も同じ値を読んで `wrangler tail` を起動する。

  ```bash
  npx --yes wrangler@"$(sed -n 's/^WRANGLER_VERSION := //p' Makefile)" whoami
  ```
- Python 3。依存は標準ライブラリのみ。
- リポジトリルートで実行すること (`wrangler tail` の cwd に使う)。

## 手順

1. **両環境が同じデータかを先に確認する。** 違えば差は実装起因ではない。

   ```bash
   for u in https://gql.trainlcd.app https://gql-stg.trainlcd.app; do curl -s "$u/__health"; echo " <- $u"; done
   ```

   食い違っていたら、その旨をユーザーに伝えてから続けるか止めるかを決める。

2. **ベンチを回す。** 既定は全 23 ケース × 15 反復 × 2 環境で、2〜3 分。
   本番側が遅いクエリを抱えているとその分伸びる。

   ```bash
   python3 .claude/skills/benchmark-gql/bench.py
   # make bench でも同じ (追加引数は make bench BENCH_ARGS="--repeat 30")
   ```

   `wrangler tail` の接続待ちだけで最大 90 秒かかるので、**バックグラウンド実行にして待つ**こと。
   フォアグラウンドだとツールのタイムアウトに当たる。

3. **レポートの「所見」節を埋める。** ここだけは自動生成しない。差が出たクエリについて、
   `src/graphql/query.rs` や `stationapi/src/use_case/interactor/query.rs` の実装、
   および `jj diff --from 'master@origin' --to 'dev@origin'` を見て、
   **どの変更が効いているか**を書く。差が出なかったこと自体が結論なら、それも明記する。

   仮説を確かめたいときは、一時的なケース定義を作って `--queries` と `--out-dir` を
   スクラッチ領域へ向けて回す。正式なカタログと `benchmarks/` を汚さずに試せる。

4. **ユーザーに要約を返す。** レポートのパスと、CPU Time で有意差 (±10% 超) が出た
   クエリだけを挙げる。全部の表を会話に貼らない。

## オプション

| オプション | 用途 |
| --- | --- |
| `--repeat N` | 反復数 (既定 15)。CPU Time はミリ秒の整数なので、軽いクエリの分解能を上げたいときは 30〜50 に増やす |
| `--warmup N` | 破棄するウォームアップ回数 (既定 3)。ここで応答の妥当性も検証し、GraphQL エラーが出たら即座に止まる |
| `--only a,b,c` | ケースを絞る。特定のクエリを追い込むとき用 |
| `--skip-baseline` | `ping` / `health` を除く。ただし `ping` を外すと応答時間の回線補正列が出なくなる |
| `--no-cpu` | `wrangler tail` を使わない。認証が無い環境や、応答時間だけ手早く見たいとき |
| `--pause SEC` | リクエスト間にスリープを入れる。連続実行でアイソレートが暖まりすぎるのを避けたいとき |
| `--dry-run` | ファイルを書かず Markdown を標準出力へ。動作確認用 |
| `--self-test` | クエリ解析とカバレッジ判定の自己診断だけ走らせる。**リクエストは送らない**。失敗があれば終了コード 1 |
| `--rerender PATH` | `benchmarks/raw/*.json` からレポートを作り直す。**リクエストは一切送らない**。集計や表の書き方を直したときに、本番へ投げ直さずに過去のレポートを更新できる。手で書いた「所見」節は残す |
| `--queries PATH` | 別のケース定義ファイルを使う。仮説を追い込む一時的なケースを、正式なカタログを汚さずに試せる (`--out-dir` と組み合わせる) |
| `--note "..."` | レポート冒頭に一言添える (例: 「#1647 のマージ後」) |

## 出力

| パス | 内容 |
| --- | --- |
| `benchmarks/YYYYMMDD-HHMMSS.md` | レポート本体 |
| `benchmarks/raw/YYYYMMDD-HHMMSS.json` | 全リクエストの生データ。あとから別の切り口で集計し直せる |
| `benchmarks/index.md` | 実行履歴。1 実行 1 行が追記される |
| `benchmarks/.logs/` | `wrangler tail` の生ログ (Git 管理外) |

## ケースを足す・直す

定義は [`queries.json`](./queries.json)。

- `fragments` に置いた GraphQL フラグメントを、各ケースの `uses` で参照すると本文に連結される。
  `Station` を返すクエリは `StationCore`、`StationNested` を返すクエリ
  (`routes` / `connectedRoutes` / `trainRoute`) は `StationNestedCore` を使う。両者は
  同じフィールドを並べた別型なので、取り違えるとフラグメントの型不一致で GraphQL エラーになる。
- `weight` は `baseline` / `light` / `medium` / `heavy` の目安。`baseline` は
  `--skip-baseline` の対象になる。
- **既存ケースの `variables` は変えない。** 過去のレポートと比較できなくなる。
  条件を変えたいときは新しい `name` のケースを足す。
- `Query` にフィールドを追加したら、ここにもケースを足す。CI が SDL を突き合わせるので、
  スキーマ変更は必ずこのファイルの更新とセットで考える。足りているかは
  `bench.py --self-test` で確かめられる (起動時にも同じ検査が走り、欠けていれば警告する)。
- **`queries.json` や `bench.py` を触ったら `--self-test` を回す。** この検査は
  「ケースの足し忘れを警告する」ためだけのもので、壊れても黙って警告が出なくなるだけなので
  気付けない。過去に取りこぼした条件 (ネストした同名フィールド、コメント内の括弧、
  ディレクティブ名、ブロック文字列) を assert で固定してある。`make test` は Rust 専用。
- 変数に使う ID は `data/*.csv` の実在レコードから採ること。`e_status` が `0` 以外の路線
  (例: `11328` 成田エクスプレスは `3`) は `lines` から返らないので、固定値には使わない。

## 読み方と落とし穴

- **判定は CPU Time の平均比。** ±10% を超えたら「速い / 遅い」、それ以内は「同等」。
  ミリ秒整数の丸めがあるので、1 反復ぶんの値は信用しない。両環境とも平均 1 ms を切る
  ケース (`ping` / `line` など) は比が丸め誤差になるため「分解能未満」として判定を保留する。
  ここを詰めたいときは `--repeat` を増やす。
- **コールドスタートは別枠。** WASM の実体化と索引構築で CPU 250 ms 超になる。
  固定閾値だと本来重いクエリを巻き込む (本番の `trainRoute` は定常で 500 ms 以上使う) ので、
  同じケース・同じ環境の中央値の 2.5 倍かつ +150 ms を超えた標本だけを外し、
  件数を「コールドスタート」節に出す。ウォームアップを入れてもアイソレートの
  割り当て先が変わると出るので、出ること自体は異常ではない。
- **応答時間の環境間比較は、そのままでは使えない。** 本番とステージングは別ドメインで
  経路も別なので、`__ping` の時点で 10 ms 単位の固定差がつくことがあり、
  その向きは実行ごとに変わる。レポートの `サーバ分` 列が、同じ環境の `__ping` の
  最小往復時間を引いた補正済み値。引いた残りが 2 ms を切ったら回線のゆらぎと
  区別がつかないので「誤差内」になる。
- **2 倍に満たない差は 2 回まわして確かめる。** `bench.py` は計測前に全ケースを一巡させて
  両環境を暖め (直前に叩いていなかった側だけ 4〜5 割高く出るのを防ぐため)、
  標本も「全ケースを 1 巡」の繰り返しで集めて実行全体へばらしている。それでも実行を
  またぐと平均は動く。実測で、同じクエリのステージング平均が別実行で 89 ms と 52 ms に
  割れたことがある。桁で違う差 (trainRoute の -98% など) はそのまま信じてよいが、
  ±数十パーセントは 1 回では結論にしない。
- **本番に負荷をかけている自覚を持つ。** 既定でもウォームアップ込みで本番へ 400 リクエスト
  以上飛ばし、そのうち何十件かは CPU を 500 ms 以上使う。`--repeat` を大きくするときや
  短時間に繰り返すときはユーザーに一声かける。
