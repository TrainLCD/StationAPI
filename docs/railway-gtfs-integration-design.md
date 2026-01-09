# 鉄道GTFSインテグレーション設計書

## 概要

ダイヤ改正への迅速な対応を実現するため、ODPT（公共交通オープンデータセンター）の鉄道GTFSデータを自動インポートする機能を追加する。

## 現状

- バスGTFS（都営バス）のみ対応
- URLがハードコード (`TOEI_BUS_GTFS_URL`)
- 単一ソースのみ

## 目標

1. 複数のGTFSソースに対応
2. 鉄道GTFSデータの自動インポート
3. APIキー認証への対応
4. ライセンス準拠の仕組み

## 対象事業者（首都圏スモールスタート）

| 事業者 | transport_type | 優先度 | 備考 |
|--------|---------------|--------|------|
| 東京メトロ | 2 (rail_gtfs) | 高 | 全線対応 |
| 都営地下鉄 | 2 (rail_gtfs) | 高 | 全線対応 |
| JR東日本 | 2 (rail_gtfs) | 中 | 関東エリア在来線のみ |
| 東武鉄道 | 2 (rail_gtfs) | 中 | 全線対応 |
| 相模鉄道 | 2 (rail_gtfs) | 低 | 全線対応 |
| 東京臨海高速鉄道 | 2 (rail_gtfs) | 低 | りんかい線 |

## 設計

### 1. TransportType の拡張

```rust
// 現状
// 0: 鉄道（CSVデータ）
// 1: バス（GTFS）

// 新規追加
// 2: 鉄道GTFS（ODPT等のGTFSデータ）
```

### 2. GTFSソース設定

```rust
/// GTFSデータソースの設定
#[derive(Debug, Clone)]
pub struct GtfsSource {
    /// ソース識別子（例: "toei_bus", "tokyo_metro"）
    pub id: String,
    /// 表示名
    pub name: String,
    /// ダウンロードURL
    pub url: String,
    /// TransportType (1: バス, 2: 鉄道GTFS)
    pub transport_type: i32,
    /// 会社コード (companies テーブルの company_cd)
    pub company_cd: i32,
    /// APIキーが必要か
    pub requires_api_key: bool,
    /// 有効/無効
    pub enabled: bool,
}
```

### 3. 環境変数

```bash
# 既存
DISABLE_BUS_FEATURE=false

# 新規追加
ODPT_API_KEY=your_api_key_here
ENABLE_RAIL_GTFS=true

# オプション: 個別ソースの有効/無効
GTFS_ENABLE_TOKYO_METRO=true
GTFS_ENABLE_TOEI_SUBWAY=true
GTFS_ENABLE_JR_EAST=false
GTFS_ENABLE_TOBU=false
```

### 4. データフロー

```
┌─────────────────────────────────────────────────┐
│ アプリケーション起動                              │
│                                                  │
│ Phase 1: CSV初期化（同期）                       │
│  └─ import_csv() - 駅データ.jp のCSV            │
│                                                  │
│ Phase 2: GTFS更新（バックグラウンド非同期）      │
│  ├─ get_gtfs_sources() - 有効なソース一覧取得   │
│  ├─ for source in sources:                       │
│  │   ├─ download_gtfs_source(&source)           │
│  │   ├─ import_gtfs_source(&source)             │
│  │   └─ integrate_gtfs_source(&source)          │
│  └─ ANALYZE                                      │
└─────────────────────────────────────────────────┘
```

### 5. ダウンロードURL形式

```rust
// 公開API（APIキー不要）
const TOEI_BUS_GTFS_URL: &str =
    "https://api-public.odpt.org/api/v4/files/Toei/data/ToeiBus-GTFS.zip";

// 認証API（APIキー必要）
// 実際のURLは https://api.odpt.org/api/v4/files/{組織}/data/{ファイル名}.zip?acl:consumerKey={APIキー}
fn build_odpt_url(org: &str, file: &str, api_key: &str) -> String {
    format!(
        "https://api.odpt.org/api/v4/files/{}/data/{}.zip?acl:consumerKey={}",
        org, file, api_key
    )
}
```

### 6. GTFSソース定義（ハードコード版）

```rust
fn get_default_gtfs_sources(api_key: Option<&str>) -> Vec<GtfsSource> {
    let mut sources = vec![
        // 都営バス（公開API）
        GtfsSource {
            id: "toei_bus".into(),
            name: "都営バス".into(),
            url: "https://api-public.odpt.org/api/v4/files/Toei/data/ToeiBus-GTFS.zip".into(),
            transport_type: 1,
            company_cd: 119,
            requires_api_key: false,
            enabled: true,
        },
    ];

    // APIキーがある場合は鉄道GTFSを追加
    if let Some(key) = api_key {
        sources.extend(vec![
            GtfsSource {
                id: "tokyo_metro".into(),
                name: "東京メトロ".into(),
                url: format!(
                    "https://api.odpt.org/api/v4/files/TokyoMetro/data/TokyoMetro-GTFS.zip?acl:consumerKey={}",
                    key
                ),
                transport_type: 2,
                company_cd: 28,  // 東京メトロの company_cd
                requires_api_key: true,
                enabled: env_bool("GTFS_ENABLE_TOKYO_METRO", true),
            },
            GtfsSource {
                id: "toei_subway".into(),
                name: "都営地下鉄".into(),
                url: format!(
                    "https://api.odpt.org/api/v4/files/Toei/data/ToeiSubway-GTFS.zip?acl:consumerKey={}",
                    key
                ),
                transport_type: 2,
                company_cd: 119, // 都営地下鉄の company_cd
                requires_api_key: true,
                enabled: env_bool("GTFS_ENABLE_TOEI_SUBWAY", true),
            },
            // JR東日本、東武鉄道等は後続フェーズで追加
        ]);
    }

    sources
}
```

### 7. データベース変更

```sql
-- transport_type の値の拡張
-- 0: 鉄道（CSVデータ - 駅データ.jp）
-- 1: バス（GTFS）
-- 2: 鉄道GTFS（ODPT等）

-- gtfs_feed_info にソースID追加
ALTER TABLE gtfs_feed_info ADD COLUMN IF NOT EXISTS source_id VARCHAR(50);

-- stations/lines にソースIDを追加（どのGTFSソースから来たか）
ALTER TABLE stations ADD COLUMN IF NOT EXISTS gtfs_source_id VARCHAR(50);
ALTER TABLE lines ADD COLUMN IF NOT EXISTS gtfs_source_id VARCHAR(50);
```

### 8. 重複データの処理

駅データ.jpのCSVデータとGTFS鉄道データには重複がある（同じ駅が両方に存在）。

**方針:**
1. CSVデータを基本とする（station_cd の一貫性維持）
2. GTFS鉄道は時刻表データ取得用として別テーブルに保持
3. 駅の紐付けは station_g_cd（グループID）と位置情報で行う

または:

**代替方針（段階的移行）:**
1. まずGTFSの時刻表データのみを利用
2. 駅・路線マスタはCSVを継続使用
3. 将来的にGTFS優先への段階的移行

## 実装計画

### Phase 1（今回実装）
- [ ] config.rs に ODPT_API_KEY 取得関数追加
- [ ] GtfsSource 構造体の定義
- [ ] 複数ソース対応の download_gtfs() 実装
- [ ] transport_type = 2 の鉄道GTFS統合

### Phase 2（将来）
- [ ] feed_info のバージョン比較による差分更新
- [ ] 定期更新スケジューラ
- [ ] JR東日本、東武鉄道等の追加

### Phase 3（将来）
- [ ] GTFS-RT（リアルタイム情報）対応
- [ ] 時刻表API公開

## ライセンス・帰属表示

ODPT データの利用にはライセンス準拠が必要:

1. 出典表示: 「公共交通オープンデータセンター」の表示
2. 各事業者のライセンス条項に準拠
3. APIアクセスキーの適切な管理

```rust
/// GTFSデータの出典情報
pub const ODPT_ATTRIBUTION: &str = "データ提供: 公共交通オープンデータセンター";
```

## 参考URL

- ODPT 公式: https://www.odpt.org/
- 開発者登録: https://developer.odpt.org/
- データカタログ: https://ckan.odpt.org/
