//! GTFS フィードの定義とダウンロード。

use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::{bail, Result};
use zip::ZipArchive;

use crate::{info, warn};

#[derive(Clone, Copy)]
pub struct GtfsFeed {
    /// ID の前置に使う。フィードをまたいだ ID の衝突を防ぐ。
    pub id: &'static str,
    pub name: &'static str,
    pub path: &'static str,
    pub url: &'static str,
    pub requires_consumer_key: bool,
}

pub const GTFS_FEEDS: &[GtfsFeed] = &[
    GtfsFeed {
        id: "toei",
        name: "都営バス",
        path: "data/ToeiBus-GTFS",
        url: "https://api-public.odpt.org/api/v4/files/Toei/data/ToeiBus-GTFS.zip",
        requires_consumer_key: false,
    },
    GtfsFeed {
        id: "seibu",
        name: "西武バス",
        path: "data/SeibuBus-GTFS",
        url: "https://api.odpt.org/api/v4/files/SeibuBus/data/SeibuBus-GTFS.zip",
        requires_consumer_key: true,
    },
    GtfsFeed {
        id: "tokyu_ota",
        name: "東急バス (大田区コミュニティバス)",
        path: "data/TokyuBus-OtaCity-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/TokyuBus/tokyubus_community_OtaCity.zip?date=current",
        requires_consumer_key: true,
    },
    GtfsFeed {
        id: "tokyu_shinagawa",
        name: "東急バス (品川区コミュニティバス)",
        path: "data/TokyuBus-ShinagawaCity-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/TokyuBus/tokyubus_community_ShinagawaCity.zip?date=current",
        requires_consumer_key: true,
    },
    GtfsFeed {
        id: "tokyu_meguro",
        name: "東急バス (目黒区コミュニティバス)",
        path: "data/TokyuBus-MeguroCity-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/TokyuBus/tokyubus_community_MeguroCity.zip?date=current",
        requires_consumer_key: true,
    },
    GtfsFeed {
        // ODPT の files エンドポイントは date の指定が要る。`current` は当日有効な
        // ダイヤを指す (省略すると 404 になる)。
        id: "keio",
        name: "京王バス",
        path: "data/KeioBus-GTFS",
        url: "https://api.odpt.org/api/v4/files/odpt/KeioBus/AllLines.zip?date=current",
        requires_consumer_key: true,
    },
];

/// ID をフィードで名前空間化する。
pub fn scoped_id(feed: &GtfsFeed, id: &str) -> String {
    format!("{}:{}", feed.id, id)
}

/// ODPT の `acl:consumerKey` を URL へ足す。`?date=current` が付いた URL もあるので
/// 区切り文字を選ぶ。
fn append_consumer_key(url: &str, token: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}acl:consumerKey={token}")
}

fn download_url(feed: &GtfsFeed) -> Result<String> {
    if !feed.requires_consumer_key {
        return Ok(feed.url.to_string());
    }
    let Ok(token) = std::env::var("ODPT_ACCESS_TOKEN") else {
        bail!("{} には ODPT_ACCESS_TOKEN が要る", feed.name);
    };
    Ok(append_consumer_key(feed.url, &token))
}

/// フィードを取得して展開する。既に展開済みなら何もしない。
///
/// 途中で失敗したら展開先を消す。中途半端なディレクトリを次回の実行が
/// 正常なフィードと見なしてしまうため。
pub fn download(feed: &GtfsFeed) -> Result<()> {
    let path = Path::new(feed.path);
    if path.exists() {
        info!("{} は取得済みなのでダウンロードを省略する", feed.name);
        return Ok(());
    }

    match download_and_extract(feed, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            if path.exists() {
                if let Err(cleanup) = fs::remove_dir_all(path) {
                    warn!("{} の中途半端な展開先を消せなかった: {cleanup}", feed.name);
                }
            }
            Err(e)
        }
    }
}

fn download_and_extract(feed: &GtfsFeed, path: &Path) -> Result<()> {
    info!("{} を ODPT から取得する", feed.name);

    // `without_url()` で URL を落とす。付けたままだと acl:consumerKey が
    // エラーメッセージ経由でログへ出る。
    let response = reqwest::blocking::get(download_url(feed)?).map_err(|e| e.without_url())?;
    if !response.status().is_success() {
        bail!("{} の取得に失敗: HTTP {}", feed.name, response.status());
    }
    let bytes = response.bytes().map_err(|e| e.without_url())?;

    fs::create_dir_all(path)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(name) = file.enclosed_name().map(|n| n.to_owned()) else {
            continue;
        };
        if file.is_dir() || name.to_string_lossy().starts_with('.') {
            continue;
        }
        // ZIP 内のディレクトリ階層は捨ててファイル名だけを使う。
        let output_name = name
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| name.to_string_lossy().to_string());
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        fs::write(path.join(&output_name), &contents)?;
    }

    info!("{} を展開した", feed.name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumer_key_uses_the_right_separator() {
        assert_eq!(
            append_consumer_key("https://example.com/a.zip", "T"),
            "https://example.com/a.zip?acl:consumerKey=T"
        );
        assert_eq!(
            append_consumer_key("https://example.com/a.zip?date=current", "T"),
            "https://example.com/a.zip?date=current&acl:consumerKey=T"
        );
    }
}
