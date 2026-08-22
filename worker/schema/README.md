# public.graphql

Worker が公開する GraphQL スキーマの「正」です。

もともと TrainLCD/BFF の `schema.graphql` を写したものですが、BFF は廃止予定で、
今後はこのファイルが公開スキーマの基準になります。

`async-graphql` はコードファーストなので、Rust の型を変えると SDL が変わります。
クライアントが壊れる変更に気付けるよう、CI (`build_worker.yml`) が Worker の
`/__schema` から SDL を取得し、このファイルと突き合わせて差分があれば失敗させます。

意図的にスキーマを変更する場合は、このファイルも合わせて更新してください。
その差分がクライアントへの影響範囲そのものになります。

## 由来

https://github.com/TrainLCD/BFF の `schema.graphql` (dev ブランチ)
