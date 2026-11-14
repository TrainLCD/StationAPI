//! 県境を越える群馬・神奈川・大阪フィードの停留所を、座標から都道府県に割り当てる。

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
            && ring_location(&self.outer, lon, lat) != Location::Outside
            && !self
                .holes
                .iter()
                .any(|ring| ring_location(ring, lon, lat) == Location::Inside)
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Location {
    Outside,
    Inside,
    Boundary,
}

fn ring_location(ring: &[[f64; 2]], lon: f64, lat: f64) -> Location {
    let mut inside = false;
    for (&[x1, y1], &[x2, y2]) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        if (x1.min(x2)..=x1.max(x2)).contains(&lon)
            && (y1.min(y2)..=y1.max(y2)).contains(&lat)
            && ((x2 - x1) * (lat - y1) - (y2 - y1) * (lon - x1)).abs() <= 1e-12
        {
            return Location::Boundary;
        }
        if (y1 > lat) != (y2 > lat) && lon < (x2 - x1) * (lat - y1) / (y2 - y1) + x1 {
            inside = !inside;
        }
    }
    if inside {
        Location::Inside
    } else {
        Location::Outside
    }
}

pub(super) fn uses_coordinates(stop_id: &str) -> bool {
    stop_id
        .strip_prefix("zz_gtfs_")
        .and_then(|id| id.split_once('_'))
        .is_some_and(|(_, namespace)| {
            namespace.starts_with("gunma_")
                || namespace.starts_with("kanagawa_")
                || namespace.starts_with("osaka_")
        })
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
            .filter(|polygon| polygon.contains(lon, lat))
            .map(|polygon| polygon.code)
            .min() // 共有境界は小さい方の都道府県コードに揃える。
    }
}

#[cfg(test)]
mod tests {
    use super::{ring_location, uses_coordinates, Location, Prefectures};

    #[test]
    fn coordinate_lookup_is_scoped_to_supported_namespaces() {
        assert!(uses_coordinates("zz_gtfs_0261_kanagawa_0259_yokohama:s1"));
        assert!(uses_coordinates(
            "zz_gtfs_0260_gunma_0177_nippon_chuo_bus:s1"
        ));
        assert!(!uses_coordinates("zz_gtfs_0237_chuo_edobus:s1"));
        assert!(!uses_coordinates("tokyu:s1"));
    }

    #[test]
    fn uses_stop_coordinates_across_prefecture_borders() {
        let index = Prefectures::load().unwrap();
        assert_eq!(index.code_at(139.01234, 36.32218), Some(10)); // 高崎駅
        assert_eq!(index.code_at(139.19004, 36.23697), Some(11)); // 本庄駅
        assert_eq!(index.code_at(138.63519, 36.34237), Some(20)); // 軽井沢駅
        assert_eq!(index.code_at(139.622, 35.466), Some(14)); // 横浜駅
        assert_eq!(index.code_at(139.446, 35.543), Some(13)); // 町田駅（県境を越える路線）
        assert_eq!(index.code_at(0.0, 0.0), None);
    }

    #[test]
    fn accepts_the_boundary_of_an_unclosed_ring() {
        let ring = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert_eq!(ring_location(&ring, 0.0, 0.5), Location::Boundary);
        assert_eq!(ring_location(&ring, 0.5, 0.5), Location::Inside);
        assert_eq!(ring_location(&ring, -0.5, 0.5), Location::Outside);
    }
}
