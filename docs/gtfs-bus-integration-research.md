# GTFS都営バスデータ導入に関する調査報告書

> **追記 (2026-05、2026-09 更新)**: 本書は実装に着手する前にまとめた調査・設計検討の資料である。実際の統合方針は実装の段階で確定しており、本書の案とは異なる部分が多い。主な違いは次のとおり。
>
> - GTFS 用のテーブル群は設けず、バスのデータを既存の `lines` / `stations` / `types` / `station_station_types` に投影した。`stations` と `lines` には `transport_type` 列 (0: 鉄道、1: バス) を加えた。
> - バス用のコードは FNV-1a ハッシュで決定的に割り当てる。値域は `line_cd` / `type_cd` / `line_group_cd` が 100,000,000 以上、`station_cd` / `station_g_cd` が 200,000,000 以上で、鉄道のコードとは重ならない。
> - `(route_id, shape_id)` ごとの運行パターンを、`TrainTypeKind::BusRoute` (= 7) の列車種別として登録した。
> - GTFS から読むのは routes・stops・trips・stop_times・translations だけで、calendar による運行日の管理は取り込んでいない。
> - 取り込み対象は都営バスだけでなく、GTFS フィード 6 本 (都営バス・西武バス・京王バスと、東急バスが運行する大田区・品川区・目黒区のコミュニティバス) と、東急バスの一般路線の ODPT JSON に広がった。
>
> 最新の実装は [`architecture.md` のデータパイプライン節](./architecture.md#データパイプライン) (特に[バスのコード生成](./architecture.md#バスのコード生成)) と `preprocessor/src/gtfs/` を参照のこと。
> また、本書が前提としている gRPC + PostgreSQL の構成は、その後 Cloudflare Workers 上の GraphQL に置き換えられた。

## 概要

本書は、日本の鉄道駅データを提供する gRPC API である StationAPI に、GTFS 形式の都営バスデータを導入する際の懸念点をまとめたものである。

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
| companies | 173 | 鉄道会社 |
| lines | 623 | 路線 |
| stations | 11,141 | 駅 |
| types | 317 | 列車種別 |
| station_station_types | 41,005 | 駅と列車種別の関連 |
| connections | 17,664 | 駅と駅の接続 |

### 1.4 主要なAPIエンドポイント

- `get_station_by_id` - ID を指定して駅を取得する
- `get_stations_by_coordinates` - 座標から周辺の駅を取得する
- `get_stations_by_line_id` - 路線に属する駅を取得する
- `get_stations_by_name` - 駅名で検索する (複数言語に対応)
- `get_train_types_by_station_id` - 駅に停車する列車種別を取得する
- `get_routes` - 経路を検索する

---

## 2. GTFSフォーマットの構造

### 2.1 標準ファイル構成

#### 必須ファイル

| ファイル | 説明 |
|----------|------|
| agency.txt | 交通事業者 |
| stops.txt | 停留所・駅 |
| routes.txt | 路線 |
| trips.txt | 便 (トリップ) |
| stop_times.txt | 各便の停留所ごとの到着・出発時刻 |

#### 条件付き必須ファイル

| ファイル | 説明 |
|----------|------|
| calendar.txt | 運行日 (曜日単位の定期パターン)。すべての運行日を calendar_dates.txt で定義する場合は省略できる |
| calendar_dates.txt | 運行日の例外。calendar.txt を省略する場合は必須 |
| feed_info.txt | フィード自体の情報。translations.txt を含める場合は必須 |

#### オプショナルファイル

| ファイル | 説明 |
|----------|------|
| shapes.txt | 車両が走る経路の形状 |
| frequencies.txt | 運行間隔 (ヘッドウェイ) による運行の定義 |
| transfers.txt | 乗り換えの規則 |
| translations.txt | 多言語の翻訳 |

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

- **提供元**: ODPT (公共交通オープンデータセンター)
- **フォーマット**: GTFS-JP (国土交通省が定めた標準的なバス情報フォーマット)
- **多言語対応**: 日本語、英語、中国語、韓国語
  - 注 (2026-09 追記): 2026-09 時点で取得した都営バス GTFS の translations.txt に含まれる言語は `ja` / `ja-Hrkt` (読み) / `en` だけで、中国語と韓国語は含まれていない。
- **リアルタイムデータ**: バスの位置情報を GTFS-RT 形式で配信している

---

## 3. 懸念点

### 3.1 データモデルの根本的な違い

#### 概念の比較

| 概念 | 鉄道（現在） | バス（GTFS） | 差異 |
|------|-------------|-------------|------|
| 時刻表 | なし | trips + stop_times | **新規追加が必要** |
| 便（Trip） | 存在しない | 中心となる概念 | **新規追加が必要** |
| サービスカレンダー | 停車条件で簡易的に表現 | calendar.txt で詳細に管理 | **新規追加が必要** |
| 運行パターン | train_type | trip ごとに定義 | 設計変更が必要 |

#### 影響

- 時刻表を扱うために、新しいエンティティ (Trip、StopTime、Calendar) を追加しなければならない。
- 既存の `train_type` モデルでは、バスの運行パターンを表現しきれない。

---

### 3.2 ID体系の衝突リスク

#### 現在のID体系

```rust
station_cd: i32  // 数値型（例: 1130101）
line_cd: i32     // 数値型
company_cd: i32  // 数値型
```

#### GTFSのID体系

```text
stop_id: String    // 文字列型（例: "0001-01"）
route_id: String   // 文字列型
agency_id: String  // 文字列型
```

#### 懸念点

- 数値型と文字列型の違いから、型の変換が必要になる。
- 既存の `station_cd` と GTFS の `stop_id` を統一するか、分けて持つかを決めなければならない。
- ID を全体で一意に保つため、プレフィックスの付け方を検討する必要がある。

#### 対応案

```rust
// 案1: 統一ID型
enum TransportId {
    Rail(i32),
    Bus(String),
}

// 案2: 文字列に統一
station_id: String  // "rail_1130101" or "bus_0001-01"
```

---

### 3.3 「駅」と「停留所」の概念の違い

| 属性 | 鉄道駅 | バス停留所 |
|------|--------|-----------|
| 数 | 約11,000 | 都営バスだけで約4,000以上 |
| 密度 | 比較的疎 | 非常に密 (数百 m 間隔) |
| グループ化 | `station_g_cd` でまとめる | まとめる基準がはっきりしない |
| 永続性 | 比較的安定 | 移設・廃止が多い |
| 命名 | 「○○駅」 | 「○○」「○○前」などさまざま |

#### 懸念点

- データ量が大きく増える (約1.5〜2倍)。
- 座標検索の性能が落ちる。
- バス停同士をまとめるロジックを新たに作る必要がある。
- 鉄道駅とバス停の間で乗り換えられるかを判定する基準が要る。

---

### 3.4 路線の概念の違い

#### 鉄道路線の特徴

- 起点と終点がはっきりしている
- 駅の並び順が決まっている
- 路線シンボル (最大 4 個) で識別できる
- `line_type` で新幹線・一般・地下鉄・路面電車・モノレール等を区別する

#### バス路線の特徴

- 循環する路線や枝分かれする路線が多い
- 同じ系統番号でも経路のパターンが複数ある
- 行き先 (headsign) による区別が重要になる
- 系統番号で管理されている

#### 懸念点

- 現在の `lines` テーブルの `line_type` に「バス」を足すだけでは足りない。
- バス特有の「系統」という概念をどうモデル化するか。
- 経路の形状 (shapes.txt) をどう保存し、どう活用するか。

---

### 3.5 列車種別 vs 運行パターン

#### 現在の train_type モデル

```sql
-- types テーブル
type_cd      -- 列車種別コード
type_name    -- 種別名（快速、急行等）
color        -- 表示色
direction    -- 方向（0:方向制限なし, 1:上り, 2:下り）
kind         -- 種別区分（0:基本, 1:支線, 2:快速, 3:急行, 4:特急, 5:高速運転快速）

-- 停車条件（station_station_types テーブルの pass フィールド）
0: 停車
1: 通過
2: 一部通過
3: 平日停車
4: 休日停車
5: 一部停車
```

#### バスの運行パターン

- 急行・各停といった区別はあまりない (一部の路線を除く)
- 時間帯によって運行が変わる (深夜バス、早朝便など)
- 曜日や祝日によって運行するかどうかが変わる
- GTFS では `trip` 単位と `calendar` の組み合わせで管理する

#### 懸念点

- 既存の `station_station_types` の設計では、バスの運行パターンを表現しにくい。
- カレンダーに基づいて運行を管理するモデルを新たに追加する必要がある。

---

### 3.6 APIエンドポイントへの影響

#### 既存エンドポイントの課題

| エンドポイント | 課題 |
|---------------|------|
| `get_station_by_id` | バス停も返すか。ID 体系の違いをどう扱うか |
| `get_stations_by_coordinates` | バス停が大量に返り、レスポンスが肥大化する |
| `get_stations_by_line_id` | バスの系統 ID をどう扱うか |
| `get_stations_by_name` | 「○○バス停」「○○前」などの検索にどう対応するか |
| `get_train_types_by_station_id` | バスには当てはまらない |
| `get_routes` | 鉄道とバスをまたぐ乗換検索が複雑になる |

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
ORDER BY point(lat, lon) <-> point($1, $2)
LIMIT $3;
```

#### 懸念点

- バス停を加えると、検索対象が 1.5〜2 倍に増える。
- 都心部ではバス停が密集している (半径 500 m 以内に数十か所)。
- 駅とバス停を混ぜて表示してよいかを判断する必要がある。

#### 対応案

```sql
-- transport_type でフィルタリング
SELECT * FROM stations
WHERE transport_type = $4  -- または transport_type IN (...)
ORDER BY point(lat, lon) <-> point($1, $2)
LIMIT $3;

-- パーティショニングの検討
CREATE TABLE stations_rail PARTITION OF stations FOR VALUES IN (0);
CREATE TABLE stations_bus PARTITION OF stations FOR VALUES IN (1);
```

---

### 3.9 データ更新・同期の問題

| 項目 | 鉄道データ | GTFSバスデータ |
|------|-----------|---------------|
| 更新頻度 | 年に数回 (ダイヤ改正時) | 週次〜月次 |
| データソース | 独自に収集し、手動で更新 | ODPT API |
| フォーマット | 独自 CSV | GTFS 標準 (ZIP) |
| 認証 | 不要 | ODPT API キーが必要 (注) |

注 (2026-09 追記): 都営バスの GTFS は ODPT の公開用エンドポイント (`api-public.odpt.org`) から認証なしで取得できる。API キー (`ODPT_ACCESS_TOKEN`) が必要なのは、後から加えた西武バス・京王バス・東急バスのデータである。

#### 必要な追加実装

1. **GTFSフィードのダウンロード処理**
   - ODPT API からデータを取得する
   - ZIP を展開してパースする

2. **差分更新ロジック**
   - 既存のデータと比較する
   - 追加・更新・削除を判定する

3. **バージョン管理**
   - フィードのバージョンを追跡する
   - ロールバックできるようにする

4. **定期実行基盤**
   - cron ジョブまたはスケジューラで定期的に実行する
   - 更新の通知とログを残す

---

### 3.10 多言語対応の差異

#### 現在の多言語フィールド

```rust
station_name: String,              // 日本語
station_name_k: String,            // カタカナ
station_name_r: Option<String>,    // ローマ字
station_name_zh: Option<String>,   // 中国語
station_name_ko: Option<String>,   // 韓国語
```

#### GTFSの多言語対応

- 翻訳は `translations.txt` で提供されるが、このファイル自体がオプショナルである。
- 都営バスの GTFS にすべての言語が含まれる保証はない。

#### 懸念点

- 多言語データが欠けている場合の扱い (NULL を許容するか)。
- 既存の言語サポートの水準とどう揃えるか。
- ローマ字を自動生成するロジックを検討する必要がある。

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

- 徒歩圏内にあるバス停をまとめる
- 時刻表に基づいて乗り換えられるかを判定する
- 乗り換えにかかる時間を推定する
- GTFS の `transfers.txt` を活用する

---

## 4. 対応アプローチ案

### 4.1 アプローチ比較

| アプローチ | 概要 | メリット | デメリット |
|-----------|------|---------|-----------|
| **A. 完全分離** | GTFS データを別の DB で管理し、API も分ける | 既存部分に影響しない。段階的に開発できる | コードが重複する。まとめて検索しにくい |
| **B. 統合拡張** | 既存のスキーマを拡張し、統一した API で提供する | API が一本になる。乗換検索がしやすい | 大規模なリファクタリングが要る。複雑になる |
| **C. アダプタ層** | GTFS 標準の形のまま保持し、変換層を設ける | GTFS 標準に準拠し、外部との互換性がある | 変換のオーバーヘッドがかかる |

### 4.2 推奨アプローチ

#### 段階的な統合拡張（B案のバリエーション）

#### Phase 1: 基盤整備

- transport_type を導入する (鉄道 = 0、バス = 1)
- ID 体系の統一を検討する
- GTFS のパーサーを実装する

#### Phase 2: バス停留所の導入

- stations テーブルを拡張する
- 座標検索を最適化する
- バス停用のインデックスを追加する

#### Phase 3: 路線・時刻表の導入

- GTFS のテーブル群を追加する
- 時刻表を検索する API を追加する
- 運行カレンダーに対応する

#### Phase 4: 統合検索

- 鉄道とバスをまたぐ乗換検索
- 最適な経路の探索

---

## 5. まとめ

### 主要懸念点

1. **データモデルの拡張**: 時刻表・便・カレンダーという概念を加える必要がある
2. **ID体系**: 数値と文字列の違いを吸収し、名前空間の衝突を避ける
3. **データ量**: バス停の追加による DB の肥大化と性能の低下
4. **API設計**: 後方互換性と新機能のバランス
5. **更新運用**: GTFS データを定期的に取り込むパイプライン
6. **乗り換え検索**: 鉄道とバスをまたぐ複雑な経路検索

### 次のステップ

1. 都営バス GTFS の実データを取得して分析する
2. ID 体系の統一方針を決める
3. スキーマ設計を詳細化する
4. プロトタイプを実装して検証する

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
