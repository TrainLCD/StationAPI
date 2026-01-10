# 鉄道データ自動更新設計書

## 概要

ダイヤ改正への迅速な対応を実現するため、ODPT（公共交通オープンデータセンター）のAPIを活用した**停車パターン差分検知システム**を構築する。

## 課題の整理

### 本当の課題
- **駅マスタデータ**: ほぼ変わらない（新駅追加、駅名変更は稀）
- **種別停車パターン**: 頻繁に変わる（ダイヤ改正のたびに変更される）

### GTFSの限界
- GTFSは「時刻表データ」であり、「列車種別」という概念がない
- `odpt:TrainTimetable`（時刻表API）で個別列車の停車駅は取得可能
- しかし「快速はこの駅に停車する」というマスタデータは存在しない

### 解決アプローチ
時刻表データから停車パターンを**集計・推論**し、前回との**差分を検知**する。

## システム設計

### アーキテクチャ

```
┌─────────────────────────────────────────────────────────┐
│  ODPT API (TrainTimetable)                              │
│  定期的に時刻表データを取得                               │
└─────────────────┬───────────────────────────────────────┘
                  ▼
┌─────────────────────────────────────────────────────────┐
│  停車パターン抽出エンジン                                │
│  種別×路線ごとに「どの駅に停車する便があるか」を集計      │
│  例: 中央線快速 → {東京, 神田, 御茶ノ水, 四ツ谷, ...}    │
└─────────────────┬───────────────────────────────────────┘
                  ▼
┌─────────────────────────────────────────────────────────┐
│  差分検知エンジン                                        │
│  前回スナップショットと比較                              │
│  → 新規停車駅、停車取りやめ駅を検出                      │
└─────────────────┬───────────────────────────────────────┘
                  ▼
┌─────────────────────────────────────────────────────────┐
│  通知/ログ出力                                          │
│  変更があれば管理者に通知                                │
│  将来的にはSlack/Discord連携も可能                       │
└─────────────────┬───────────────────────────────────────┘
                  ▼
┌─────────────────────────────────────────────────────────┐
│  人間が確認                                             │
│  station_station_types.csv を手動更新                   │
└─────────────────────────────────────────────────────────┘
```

### データベーススキーマ

```sql
-- 停車パターンのスナップショット
CREATE TABLE stop_pattern_snapshots (
    id SERIAL PRIMARY KEY,
    railway_id VARCHAR(100) NOT NULL,      -- odpt.Railway:JR-East.ChuoRapid
    train_type_id VARCHAR(100) NOT NULL,   -- odpt.TrainType:JR-East.Rapid
    train_type_name VARCHAR(100),          -- 快速
    station_ids TEXT[] NOT NULL,           -- 停車駅IDの配列
    station_names TEXT[],                  -- 停車駅名の配列（参照用）
    captured_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(railway_id, train_type_id, captured_at::date)
);

-- 停車パターン変更ログ
CREATE TABLE stop_pattern_changes (
    id SERIAL PRIMARY KEY,
    railway_id VARCHAR(100) NOT NULL,
    train_type_id VARCHAR(100) NOT NULL,
    train_type_name VARCHAR(100),
    change_type VARCHAR(20) NOT NULL,      -- 'added' or 'removed'
    station_id VARCHAR(100) NOT NULL,
    station_name VARCHAR(100),
    detected_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    acknowledged BOOLEAN DEFAULT FALSE,    -- 確認済みフラグ
    acknowledged_at TIMESTAMP
);

-- インデックス
CREATE INDEX idx_stop_pattern_snapshots_railway ON stop_pattern_snapshots(railway_id, train_type_id);
CREATE INDEX idx_stop_pattern_changes_detected ON stop_pattern_changes(detected_at DESC);
CREATE INDEX idx_stop_pattern_changes_unack ON stop_pattern_changes(acknowledged) WHERE acknowledged = FALSE;
```

### ODPT API エンドポイント

```
# 列車時刻表（メイン）
GET https://api.odpt.org/api/v4/odpt:TrainTimetable
    ?odpt:operator=odpt.Operator:TokyoMetro
    &acl:consumerKey={API_KEY}

# 路線マスタ
GET https://api.odpt.org/api/v4/odpt:Railway
    ?odpt:operator=odpt.Operator:TokyoMetro
    &acl:consumerKey={API_KEY}

# 列車種別マスタ
GET https://api.odpt.org/api/v4/odpt:TrainType
    ?odpt:operator=odpt.Operator:TokyoMetro
    &acl:consumerKey={API_KEY}

# 駅マスタ
GET https://api.odpt.org/api/v4/odpt:Station
    ?odpt:operator=odpt.Operator:TokyoMetro
    &acl:consumerKey={API_KEY}
```

### TrainTimetable レスポンス例

```json
{
  "@id": "urn:ucode:_00001C000000000000010000030E9A5F",
  "@type": "odpt:TrainTimetable",
  "odpt:railway": "odpt.Railway:TokyoMetro.Marunouchi",
  "odpt:trainNumber": "A0601",
  "odpt:trainType": "odpt.TrainType:TokyoMetro.Local",
  "odpt:trainTimetableObject": [
    {
      "odpt:departureTime": "05:00",
      "odpt:departureStation": "odpt.Station:TokyoMetro.Marunouchi.Ogikubo"
    },
    {
      "odpt:departureTime": "05:02",
      "odpt:departureStation": "odpt.Station:TokyoMetro.Marunouchi.Minami-asagaya"
    },
    ...
  ]
}
```

### 停車パターン抽出ロジック

```rust
/// 時刻表データから停車パターンを抽出
async fn extract_stop_patterns(
    timetables: &[TrainTimetable],
) -> HashMap<(RailwayId, TrainTypeId), HashSet<StationId>> {
    let mut patterns: HashMap<(RailwayId, TrainTypeId), HashSet<StationId>> = HashMap::new();

    for timetable in timetables {
        let key = (timetable.railway.clone(), timetable.train_type.clone());
        let stations = patterns.entry(key).or_insert_with(HashSet::new);

        for stop in &timetable.train_timetable_object {
            // 出発駅または到着駅を停車駅として記録
            if let Some(station) = &stop.departure_station {
                stations.insert(station.clone());
            }
            if let Some(station) = &stop.arrival_station {
                stations.insert(station.clone());
            }
        }
    }

    patterns
}
```

### 差分検知ロジック

```rust
/// 前回のパターンと比較して差分を検出
fn detect_changes(
    previous: &HashMap<(RailwayId, TrainTypeId), HashSet<StationId>>,
    current: &HashMap<(RailwayId, TrainTypeId), HashSet<StationId>>,
) -> Vec<StopPatternChange> {
    let mut changes = Vec::new();

    for (key, current_stations) in current {
        if let Some(prev_stations) = previous.get(key) {
            // 新規停車駅
            for station in current_stations.difference(prev_stations) {
                changes.push(StopPatternChange {
                    railway_id: key.0.clone(),
                    train_type_id: key.1.clone(),
                    change_type: ChangeType::Added,
                    station_id: station.clone(),
                });
            }
            // 停車取りやめ駅
            for station in prev_stations.difference(current_stations) {
                changes.push(StopPatternChange {
                    railway_id: key.0.clone(),
                    train_type_id: key.1.clone(),
                    change_type: ChangeType::Removed,
                    station_id: station.clone(),
                });
            }
        } else {
            // 新規種別
            for station in current_stations {
                changes.push(StopPatternChange {
                    railway_id: key.0.clone(),
                    train_type_id: key.1.clone(),
                    change_type: ChangeType::Added,
                    station_id: station.clone(),
                });
            }
        }
    }

    changes
}
```

## 対応事業者

ODPT TrainTimetable API で取得可能な事業者：

| 事業者 | Operator ID | 優先度 | 備考 |
|--------|-------------|--------|------|
| 東京メトロ | TokyoMetro | 高 | 全線対応 |
| 都営地下鉄 | Toei | 高 | 全線対応 |
| JR東日本 | JR-East | 高 | 関東在来線（新幹線除く） |
| 東武鉄道 | Tobu | 中 | 全線対応 |
| 西武鉄道 | Seibu | 中 | 全線対応 |
| 京王電鉄 | Keio | 中 | 全線対応 |
| 小田急電鉄 | Odakyu | 中 | 全線対応 |
| 東急電鉄 | Tokyu | 中 | 全線対応 |
| 京急電鉄 | Keikyu | 中 | 全線対応 |
| 京成電鉄 | Keisei | 低 | 全線対応 |
| 相鉄 | Sotetsu | 低 | 全線対応 |

## 環境変数

```bash
# ODPT APIキー（必須）
ODPT_API_KEY=your_api_key_here

# 差分検知の有効化
ENABLE_STOP_PATTERN_DETECTION=true

# 検知間隔（時間）
STOP_PATTERN_CHECK_INTERVAL_HOURS=24

# 通知設定（将来）
# SLACK_WEBHOOK_URL=https://hooks.slack.com/...
# DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
```

## 実装計画

### Phase 1: 基本機能（今回）
- [x] 複数GTFSソース対応（バス用）
- [ ] ODPT TrainTimetable API クライアント実装
- [ ] 停車パターン抽出ロジック
- [ ] 差分検知ロジック
- [ ] DBスキーマ追加
- [ ] CLIコマンド追加（手動実行用）

### Phase 2: 自動化
- [ ] 定期実行スケジューラ
- [ ] 通知機能（ログ → Slack/Discord）
- [ ] Webダッシュボード（変更一覧表示）

### Phase 3: 自動適用
- [ ] station_station_types との自動マッピング
- [ ] 変更の自動適用（要承認フロー）

## 検出例

```
[2025-03-15] 停車パターン変更を検出しました:

路線: JR東日本 中央線快速
種別: 通勤快速

新規停車:
  + 中野駅 (odpt.Station:JR-East.Nakano)

停車取りやめ:
  - なし

---
路線: 東京メトロ 副都心線
種別: 急行

新規停車:
  + 雑司が谷駅 (odpt.Station:TokyoMetro.Zoshigaya)

停車取りやめ:
  - なし
```

## 制限事項

1. **完全な自動化は困難**
   - ODPT の種別ID と StationAPI の type_cd のマッピングが必要
   - 同じ「快速」でも事業者によってIDが異なる

2. **データの鮮度**
   - ODPT データは毎日更新されるわけではない
   - ダイヤ改正直後は反映に時間がかかる可能性

3. **新幹線非対応**
   - JR東日本の新幹線はODPT対象外
   - 引き続き手動管理が必要

## 既存GTFS機能との関係

### 現在のGTFS機能（バス用）
- 都営バスのGTFSデータをインポート
- gtfs_routes, gtfs_trips, gtfs_stops 等のテーブルに保存
- transport_type = 1 として stations/lines に統合

### 停車パターン検知（新規）
- GTFS とは別系統
- TrainTimetable API を直接利用
- 差分検知・通知が目的（データ統合ではない）

両者は独立して動作し、干渉しない。

## 参考URL

- ODPT 公式: https://www.odpt.org/
- 開発者登録: https://developer.odpt.org/
- データカタログ: https://ckan.odpt.org/
- API仕様: https://developer.odpt.org/documents
