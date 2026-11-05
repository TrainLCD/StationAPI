# 駅の登録と新経路の反映 (#1655)

上所 (1140833) は越後線の普通系統 900/902 に、羽田空港第2ターミナル
(9933610) は東京モノレールの既存系統に登録済み。

箱根登山電車は塔ノ沢 → 出山信号場 → 大平台 → 上大平台信号場 →
仙人台信号場 → 宮ノ下の順とする。既存 ID は維持し、新しい信号場には
9933912–9933914 を割り当てる。系統975にも同じ順で運転停車地点を入れる。
駅番号は付与しない。開設日は今回の確認対象としないため未設定。

停車フラグは3か所とも `pass=0` とする。出山・上大平台の停止は公式の
スイッチバック案内に基づく。仙人台は下記二次資料の「列車交換を行わない
列車も必ず一旦停車する」という記述に基づく。仙人台の全列車停止について
公式資料での確認はできておらず、根拠の確度は同一ではない。

- 仙人台の運転停車（二次資料）: https://w.atwiki.jp/kantoushitetsu/pages/318.html

- 信号場の存在とスイッチバック: https://www.hakonenavi.jp/hakone-tozan/
- 仙人台を含む公式資料: https://www.hakonenavi.jp/_wp/wp-content/uploads/2025/09/d3f53da6390150014059d6fb32fce430.pdf
- 座標照合: https://zh.wikipedia.org/wiki/出山信號場 、https://zh.wikipedia.org/wiki/上大平台信號場 、https://zh.wikipedia.org/wiki/仙人台信號場

広電1・2・5・6号線は広島駅の次を稲荷町とし、5号線は稲荷町 → 松川町 →
比治山下とする。旧経路の的場町・段原一丁目のレコードは履歴として残して
2025-08-02の営業終了で閉じる。物理的な電停の廃止ではなく、その系統での
営業終了を表す。的場町・段原一丁目は新設する循環線 (99719) に登録する。
循環線は広電本社前を両端に配置し、両端で同じ駅グループを参照する。
普通系統1000099719を明示登録する。

- 駅前大橋ルート: https://www.hiroden.co.jp/topics/2025/pdf/0708_tramdia/tramdia1.pdf
- 循環線の開業日・経路: https://www.hiroden.co.jp/topics/2026/pdf/0328-loopline/0212_release.pdf

`data/*.csv` の変更後は、`make data` で `generated/` を更新し、
`cargo run -p data_validator` で検証してから `make build` でWorkerを再ビルドする。
既存の `generated/` がある場合、Workerはそちらを優先するため、再生成が必要になる。

連続駅順チェックは、`data_validator/src/main.rs` の
`EXPECTED_CONSECUTIVE_STATION_ORDERS` に登録した区間について、
`3!stations.csv` の有効駅を `e_sort`・`station_cd` 順に並べて検査する。
`5!station_station_types.csv` の系統所属 (`line_group_cd`)・系統内の駅順・
停車フラグ (`pass`) の退行は、このチェックの対象外である。
