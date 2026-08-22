# bff.graphql

TrainLCD/BFF の `schema.graphql` の写しです。

Worker は GraphQL をそのまま返すため、BFF が公開しているスキーマと
一致している必要があります。CI (`build_worker.yml`) が Worker の生成した
SDL とこのファイルを突き合わせ、差分があればビルドを失敗させます。

BFF 側でスキーマが変わったら、このファイルを更新してください。

取得元:
https://raw.githubusercontent.com/TrainLCD/BFF/refs/heads/dev/schema.graphql
