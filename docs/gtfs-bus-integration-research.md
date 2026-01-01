# GTFS都営バスデータ導入に関する調査報告書

## 概要

本ドキュメントは、既存のStationAPI（日本の鉄道駅データを扱うgRPC API）に、GTFSフォーマットの都営バスデータを導入する際の懸念点をまとめたものである。

---

## 1. 現在のStationAPIの構造

### 1.1 技術スタック

| 項目 | 技術 |
|------|------|
| 言語 | Rust (edition 2021) |
| 非同期ランタイム | Tokio |
| API | gRPC (Tonic) + gRPC-Web |
| データベース | PostgreSQL 18 |
| ORM | SQLx |

### 1.2 データモデル

```text
companies (鉄道会社)
    ↓
lines (路線)
    ↓
stations (駅)
    ↓
station_station_types (駅と列車種別の関連)
    ↓
types (列車種別)
```

### 1.3 主要テーブル

| テーブル | レコード数 | 説明 |
|----------|-----------|------|
| companies | 173 | 鉄道会社情報 |
| lines | 623 | 路線情報 |
| stations | 11,141 | 駅情報 |
| types | 317 | 列車種別 |
| station_station_types | 41,005 | 駅と列車種別の関連 |
| connections | 17,664 | 駅間接続情報 |

### 1.4 主要なAPIエンドポイント

- `get_station_by_id` - ID指定で駅取得
- `get_stations_by_coordinates` - 座標から周辺駅を取得
- `get_stations_by_line_id` - 路線内の駅を取得
- `get_stations_by_name` - 駅名検索（複数言語対応）
- `get_train_types_by_station_id` - 駅の列車種別を取得
- `get_routes` - ルート検索

---

## 2. GTFSフォーマットの構造

### 2.1 標準ファイル構成

#### 必須ファイル

| ファイル | 説明 |
|----------|------|
| agency.txt | 交通事業者情報 |
| stops.txt | 停留所・駅情報 |
| routes.txt | 路線情報 |
| trips.txt | 便（トリップ）情報 |
| stop_times.txt | 停留所での到着・出発時刻 |

#### 条件付き必須ファイル

| ファイル | 説明 |
|----------|------|
| calendar.txt | サービス日（週単位のスケジュール） |
| calendar_dates.txt | サービス日の例外 |

#### オプショナルファイル

| ファイル | 説明 |
|----------|------|
| shapes.txt | 路線の地理的形状 |
| frequencies.txt | 便の頻度情報 |
| transfers.txt | 乗換情報 |
| translations.txt | 多言語対応 |

### 2.2 GTFSデータモデル

```text
agency (事業者)
    ↓
routes (路線)
    ↓
trips (便) ← calendar (サービスカレンダー)
    ↓
stop_times (時刻表)
    ↓
stops (停留所)
```

### 2.3 都営バスGTFSデータの特徴

- **提供元**: ODPT（公共交通オープンデータセンター）
- **フォーマット**: GTFS-JP（国土交通省標準）
- **多言語対応**: 日本語、英語、中国語、韓国語
- **リアルタイムデータ**: GTFS-RT形式でバス位置情報を配信

---

## 3. 懸念点

### 3.1 データモデルの根本的な違い

#### 概念の比較

| 概念 | 鉄道（現在） | バス（GTFS） | 差異 |
|------|-------------|-------------|------|
| 時刻表 | なし | trips + stop_times | **新規追加が必要** |
| 便（Trip） | 存在しない | 核心概念 | **新規追加が必要** |
| サービスカレンダー | 停車条件で簡易対応 | calendar.txtで詳細管理 | **新規追加が必要** |
| 運行パターン | train_type | tripごとに定義 | 設計変更が必要 |

#### 影響

- 時刻表データを扱うための新しいエンティティ（Trip, StopTime, Calendar）の追加が必要
- 既存の `train_type` モデルではバスの運行パターンを表現しきれない

---

### 3.2 ID体系の衝突リスク

#### 現在のID体系

```rust
station_cd: u32  // 数値型（例: 1130101）
line_cd: u32     // 数値型
company_cd: u32  // 数値型
```

#### GTFSのID体系

```text
stop_id: String    // 文字列型（例: "0001_01"）
route_id: String   // 文字列型
agency_id: String  // 文字列型
```

#### 懸念点

- 数値型 vs 文字列型の違いによる型変換の必要性
- 既存の `station_cd` と GTFS `stop_id` を統一するか分離するかの設計判断
- グローバル一意性を確保するためのプレフィックス戦略の検討

#### 対応案

```rust
// 案1: 統一ID型
enum TransportId {
    Rail(u32),
    Bus(String),
}

// 案2: 文字列に統一
station_id: String  // "rail_1130101" or "bus_0001_01"
```

---

### 3.3 「駅」と「停留所」の概念の違い

| 属性 | 鉄道駅 | バス停留所 |
|------|--------|-----------|
| 数量 | 約11,000 | 都営バスだけで約4,000以上 |
| 密度 | 比較的疎 | 非常に密集（数百m間隔） |
| グループ化 | `station_g_cd`で統合 | 統合基準が曖昧 |
| 永続性 | 比較的安定 | 頻繁に移設・廃止 |
| 命名規則 | 「○○駅」 | 「○○」「○○前」など多様 |

#### 懸念点

- データ量の大幅増加（約1.5〜2倍）
- 座標検索時のパフォーマンス劣化
- バス停同士のグループ化ロジックの新規実装
- 鉄道駅とバス停の乗り換え判定基準

---

### 3.4 路線の概念の違い

#### 鉄道路線の特徴

- 明確な起点・終点
- 駅の並び順が固定
- 路線シンボル（最大4個）で識別
- `line_type`: 新幹線、在来線、地下鉄、モノレール等

#### バス路線の特徴

- 循環路線、枝分かれ路線が多い
- 同一路線番号で複数の経路パターン
- 行き先（headsign）による区別が重要
- 系統番号による管理

#### 懸念点

- 現在の `lines` テーブルの `line_type` に「バス」を追加するだけでは不十分
- バス特有の「系統」概念のモデル化
- 経路パターン（shapes.txt）の保存・活用方法

---

### 3.5 列車種別 vs 運行パターン

#### 現在の train_type モデル

```sql
-- types テーブル
type_cd      -- 列車種別コード
type_name    -- 種別名（快速、急行等）
color        -- 表示色
direction    -- 方向（0:双方向, 1:上り, 2:下り）
kind         -- 種別（0:通常, 1:快速, 2:急行等）

-- 停車条件（pass フィールド）
0: 全停車
1: 停車なし（通過）
2: 一部停車
3: 平日のみ
4: 休日のみ
5: 部分停車
```

#### バスの運行パターン

- 急行・各停の概念が薄い（一部路線を除く）
- 時間帯依存（深夜バス、早朝便等）
- 曜日・祝日による運行有無
- GTFSでは `trip` 単位 + `calendar` で管理

#### 懸念点

- 既存の `station_station_types` の設計ではバスの運行パターンを表現困難
- カレンダーベースの運行管理モデルの新規追加が必要

---

### 3.6 APIエンドポイントへの影響

#### 既存エンドポイントの課題

| エンドポイント | 課題 |
|---------------|------|
| `get_station_by_id` | バス停も含めるか？ID体系の違いは？ |
| `get_stations_by_coordinates` | バス停の大量返却によるレスポンス肥大化 |
| `get_stations_by_line_id` | バス系統IDの扱い方 |
| `get_stations_by_name` | 「○○バス停」「○○前」等の検索対応 |
| `get_train_types_by_station_id` | バスには適用不可 |
| `get_routes` | 鉄道・バス横断の乗換検索の複雑化 |

#### 対応案

```protobuf
// 案1: フィルタパラメータの追加
message GetStationsByCoordinatesRequest {
  double latitude = 1;
  double longitude = 2;
  int32 limit = 3;
  TransportType transport_type = 4;  // RAIL, BUS, ALL
}

// 案2: バス専用エンドポイントの追加
service BusStopApi {
  rpc GetBusStopById(GetBusStopByIdRequest) returns (BusStopResponse);
  rpc GetBusStopsByRouteId(GetBusStopsByRouteIdRequest) returns (MultipleBusStopResponse);
}
```

---

### 3.7 データベースへの影響

#### スキーマ拡張案

```sql
-- 案1: GTFSテーブルを別途追加
CREATE TABLE gtfs_agencies (
    agency_id VARCHAR PRIMARY KEY,
    agency_name VARCHAR NOT NULL,
    agency_url VARCHAR,
    agency_timezone VARCHAR
);

CREATE TABLE gtfs_stops (
    stop_id VARCHAR PRIMARY KEY,
    stop_code VARCHAR,
    stop_name VARCHAR NOT NULL,
    stop_lat DOUBLE PRECISION,
    stop_lon DOUBLE PRECISION,
    location_type INT  -- 0:停留所, 1:駅
);

CREATE TABLE gtfs_routes (
    route_id VARCHAR PRIMARY KEY,
    agency_id VARCHAR REFERENCES gtfs_agencies,
    route_short_name VARCHAR,
    route_long_name VARCHAR,
    route_type INT,  -- 3:バス
    route_color VARCHAR
);

CREATE TABLE gtfs_trips (
    trip_id VARCHAR PRIMARY KEY,
    route_id VARCHAR REFERENCES gtfs_routes,
    service_id VARCHAR,
    trip_headsign VARCHAR,
    direction_id INT
);

CREATE TABLE gtfs_stop_times (
    trip_id VARCHAR REFERENCES gtfs_trips,
    stop_id VARCHAR REFERENCES gtfs_stops,
    arrival_time TIME,
    departure_time TIME,
    stop_sequence INT,
    PRIMARY KEY (trip_id, stop_sequence)
);

CREATE TABLE gtfs_calendar (
    service_id VARCHAR PRIMARY KEY,
    monday BOOLEAN,
    tuesday BOOLEAN,
    wednesday BOOLEAN,
    thursday BOOLEAN,
    friday BOOLEAN,
    saturday BOOLEAN,
    sunday BOOLEAN,
    start_date DATE,
    end_date DATE
);
```

```sql
-- 案2: 既存テーブルの拡張
ALTER TABLE stations ADD COLUMN transport_type INT DEFAULT 0;  -- 0:鉄道, 1:バス
ALTER TABLE stations ADD COLUMN gtfs_stop_id VARCHAR;
ALTER TABLE lines ADD COLUMN is_bus BOOLEAN DEFAULT FALSE;
ALTER TABLE lines ADD COLUMN gtfs_route_id VARCHAR;
```

#### パフォーマンス懸念

| 項目 | 現在 | バス追加後（推定） |
|------|------|-------------------|
| stations レコード数 | 11,141 | 15,000〜20,000 |
| インデックスサイズ | - | 1.5〜2倍 |
| stop_times レコード数 | 0 | 数百万〜数千万 |

---

### 3.8 座標検索のパフォーマンス

#### 現在の実装

```sql
-- idx_performance_stations_point インデックス使用
SELECT * FROM stations
ORDER BY point(lon, lat) <-> point($1, $2)
LIMIT $3;
```

#### 懸念点

- バス停追加で検索対象が1.5〜2倍に増加
- 都心部ではバス停が密集（半径500m内に数十箇所）
- 駅とバス停の混在表示の是非

#### 対応案

```sql
-- transport_type でフィルタリング
SELECT * FROM stations
WHERE transport_type = $4  -- または transport_type IN (...)
ORDER BY point(lon, lat) <-> point($1, $2)
LIMIT $3;

-- パーティショニングの検討
CREATE TABLE stations_rail PARTITION OF stations FOR VALUES IN (0);
CREATE TABLE stations_bus PARTITION OF stations FOR VALUES IN (1);
```

---

### 3.9 データ更新・同期の問題

| 項目 | 鉄道データ | GTFSバスデータ |
|------|-----------|---------------|
| 更新頻度 | 年数回（ダイヤ改正時） | 週次〜月次 |
| データソース | 独自収集・手動更新 | ODPT API |
| フォーマット | 独自CSV | GTFS標準（ZIP） |
| 認証 | 不要 | ODPT APIキー必要 |

#### 必要な追加実装

1. **GTFSフィードのダウンロード処理**
   - ODPT APIからのデータ取得
   - ZIP解凍・パース処理

2. **差分更新ロジック**
   - 既存データとの比較
   - 追加・更新・削除の判定

3. **バージョン管理**
   - フィードバージョンの追跡
   - ロールバック機能

4. **定期実行基盤**
   - cronジョブまたはスケジューラ
   - 更新通知・ログ

---

### 3.10 多言語対応の差異

#### 現在の多言語フィールド

```rust
station_name: String,      // 日本語
station_name_k: String,    // カタカナ
station_name_r: String,    // ローマ字
station_name_zh: String,   // 中国語
station_name_ko: String,   // 韓国語
```

#### GTFSの多言語対応

- `translations.txt` でオプショナル対応
- 都営バスGTFSに全言語が含まれる保証なし

#### 懸念点

- 多言語データの欠損処理（NULLable対応）
- 既存の言語サポートレベルとの整合性
- ローマ字の自動生成ロジック検討

---

### 3.11 「乗り換え」の複雑化

#### 現在の接続モデル

```sql
-- connections テーブル
station_cd1  -- 駅コード1
station_cd2  -- 駅コード2
distance     -- 駅間距離（メートル）
```

#### バス導入後の複雑性

| 乗り換えパターン | 現在 | バス導入後 |
|-----------------|------|-----------|
| 鉄道 ↔ 鉄道 | 対応済み | 継続 |
| 鉄道 ↔ バス | - | **新規対応必要** |
| バス ↔ バス | - | **新規対応必要** |

#### 追加考慮事項

- 徒歩圏内のバス停グループ化
- 時刻表ベースの乗り換え可否判定
- 乗り換え時間の推定
- GTFSの `transfers.txt` の活用

---

## 4. 対応アプローチ案

### 4.1 アプローチ比較

| アプローチ | 概要 | メリット | デメリット |
|-----------|------|---------|-----------|
| **A. 完全分離** | GTFSデータを別DBで管理し、APIも分離 | 既存影響なし、段階的開発可能 | コード重複、統合検索困難 |
| **B. 統合拡張** | 既存スキーマを拡張し、統一APIで提供 | 統一API、乗換検索容易 | 大規模リファクタ、複雑化 |
| **C. アダプタ層** | GTFS標準のまま保持し、変換層を設ける | GTFS標準準拠、外部互換性 | 変換オーバーヘッド |

### 4.2 推奨アプローチ

#### 段階的な統合拡張（B案のバリエーション）

#### Phase 1: 基盤整備

- transport_type の導入（鉄道=0, バス=1）
- ID体系の統一検討
- GTFSパーサーの実装

#### Phase 2: バス停留所の導入

- stations テーブルの拡張
- 座標検索の最適化
- バス停用インデックス追加

#### Phase 3: 路線・時刻表の導入

- GTFSテーブル群の追加
- 時刻表検索API追加
- 運行カレンダー対応

#### Phase 4: 統合検索

- 鉄道・バス横断の乗換検索
- 最適経路探索

---

## 5. まとめ

### 主要懸念点

1. **データモデルの拡張**: 時刻表・便・カレンダーの概念追加が必要
2. **ID体系**: 数値 vs 文字列、名前空間の衝突回避
3. **データ量**: バス停追加によるDB肥大化とパフォーマンス
4. **API設計**: 後方互換性 vs 新機能のバランス
5. **更新運用**: GTFSデータの定期取り込みパイプライン
6. **乗り換え検索**: 鉄道・バス横断の複雑なルート検索

### 次のステップ

1. 都営バスGTFSデータの実データ取得・分析
2. ID体系の統一方針決定
3. スキーマ設計の詳細化
4. プロトタイプ実装による検証

---

## 参考資料

### GTFS関連

- [GTFS.org - General Transit Feed Specification](https://gtfs.org/)
- [GTFS Reference](https://gtfs.org/documentation/schedule/reference/)
- [GTFS.JP - 標準的なバス情報フォーマット](https://www.gtfs.jp/)

### 都営バス・東京交通データ

- [公共交通オープンデータセンター (ODPT)](https://www.odpt.org/)
- [東京公共交通オープンデータチャレンジ](https://tokyochallenge.odpt.org/)

### 国土交通省

- [静的バス情報フォーマット（GTFS-JP）仕様書](https://www.mlit.go.jp/sogoseisaku/transport/sosei_transport_tk_000112.html)
