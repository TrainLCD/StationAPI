//! 県境を越える群馬フィードの停留所を、座標から都道府県に割り当てる。

use anyhow::{Context, Result};
use serde::Deserialize;

type Ring = Vec<[f64; 2]>;

#[derive(Deserialize)]
struct FeatureCollection {
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    id: String,
    geometry: Geometry,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "coordinates")]
enum Geometry {
    Polygon(Vec<Ring>),
    MultiPolygon(Vec<Vec<Ring>>),
}

struct Polygon {
    code: i32,
    outer: Ring,
    holes: Vec<Ring>,
    min: [f64; 2],
    max: [f64; 2],
}

impl Polygon {
    fn contains(&self, lon: f64, lat: f64) -> bool {
        (self.min[0]..=self.max[0]).contains(&lon)
            && (self.min[1]..=self.max[1]).contains(&lat)
            && contains(&self.outer, lon, lat)
            && !self.holes.iter().any(|ring| contains(ring, lon, lat))
    }
}

fn contains(ring: &[[f64; 2]], lon: f64, lat: f64) -> bool {
    let mut inside = false;
    for edge in ring.windows(2) {
        let [x1, y1] = edge[0];
        let [x2, y2] = edge[1];
        if (y1 > lat) != (y2 > lat) && lon < (x2 - x1) * (lat - y1) / (y2 - y1) + x1 {
            inside = !inside;
        }
    }
    inside
}

pub(super) fn uses_coordinates(stop_id: &str) -> bool {
    stop_id
        .strip_prefix("zz_gtfs_")
        .and_then(|id| id.split_once('_'))
        .is_some_and(|(_, namespace)| namespace.starts_with("gunma_"))
}

pub(super) struct Prefectures(Vec<Polygon>);

impl Prefectures {
    pub(super) fn load() -> Result<Self> {
        let data: FeatureCollection =
            serde_json::from_str(include_str!("../../../data/prefectures.geojson"))
                .context("都道府県境界を読み込めない")?;
        let mut index = Vec::new();
        for feature in data.features {
            let code = feature.id.parse()?;
            let polygons = match feature.geometry {
                Geometry::Polygon(rings) => vec![rings],
                Geometry::MultiPolygon(polygons) => polygons,
            };
            for rings in polygons {
                let mut rings = rings.into_iter();
                let outer = rings.next().context("都道府県境界に外周が無い")?;
                let mut min = [f64::INFINITY; 2];
                let mut max = [f64::NEG_INFINITY; 2];
                for &[lon, lat] in &outer {
                    min = [min[0].min(lon), min[1].min(lat)];
                    max = [max[0].max(lon), max[1].max(lat)];
                }
                index.push(Polygon {
                    code,
                    outer,
                    holes: rings.collect(),
                    min,
                    max,
                });
            }
        }
        Ok(Self(index))
    }

    pub(super) fn code_at(&self, lon: f64, lat: f64) -> Option<i32> {
        self.0
            .iter()
            .find(|polygon| polygon.contains(lon, lat))
            .map(|polygon| polygon.code)
    }
}

#[cfg(test)]
mod tests {
    use super::Prefectures;

    #[test]
    fn uses_stop_coordinates_across_prefecture_borders() {
        let index = Prefectures::load().unwrap();
        assert_eq!(index.code_at(139.01234, 36.32218), Some(10)); // 高崎駅
        assert_eq!(index.code_at(139.19004, 36.23697), Some(11)); // 本庄駅
        assert_eq!(index.code_at(138.63519, 36.34237), Some(20)); // 軽井沢駅
        assert_eq!(index.code_at(0.0, 0.0), None);
    }
}
