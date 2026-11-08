# 追加バスGTFS

フィードごとに `zz_gtfs_<連番>_<地域名>.json` を追加します。
ファイル名は路線・停留所の永続IDに使うため、追加後は変更しないでください。
連番は、既存の設定の後に並ぶ値を選びます。

`name` はフィード名、`url` は現行ZIPのHTTPS URL、`source` は配布元、
`license` は利用条件です。 `company_cd` は事業者コードです。
既存事業者のときは `company` を省略します。新規事業者には
`company` で `name`、`kana`、`english`、`url` を指定します。
正式名称が `name` と異なる場合は `name_full` に指定します。
省略すると `name` が正式名称にも使われます。
同じ事業者の複数のフィードは同じ定義を使えます。

`https://api.odpt.org/` のフィードには既存の `ODPT_ACCESS_TOKEN` を使います。
展開済みのキャッシュは `data/gtfs-cache/<設定ファイル名から拡張子を除いたもの>/` に置きます。
