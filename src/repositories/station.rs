use async_trait::async_trait;
use sqlx::{MySql, Pool};

type StationEntity = crate::entities::station::Station;
type LineEntity = crate::entities::line::Line;
type CompanyEntity = crate::entities::company::Company;

#[async_trait]
pub trait StationRepository {
    async fn find_one(&self, id: u32) -> Option<StationEntity>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
    ) -> Option<Vec<StationEntity>>;
    async fn get_by_group_id(&self, id: u32) -> Option<Vec<StationEntity>>;
    async fn get_lines_by_station_id(&self, station_id: u32) -> Option<Vec<LineEntity>>;
    async fn get_lines_by_station_ids(&self, station_ids: Vec<u32>) -> Option<Vec<LineEntity>>;
    async fn get_lines_by_line_ids(&self, line_ids: Vec<u32>) -> Option<Vec<LineEntity>>;
    async fn find_one_line_by_station_id(&self, station_id: u32) -> Option<LineEntity>;
    async fn get_companies_by_line_ids(&self, line_ids: Vec<u32>) -> Option<Vec<CompanyEntity>>;
}

pub struct StationRepositoryImplOnMySQL<'a> {
    pub pool: &'a Pool<MySql>,
}

#[async_trait]
impl StationRepository for StationRepositoryImplOnMySQL<'_> {
    async fn find_one(&self, id: u32) -> Option<StationEntity> {
        sqlx::query_as::<_, StationEntity>("SELECT * FROM stations WHERE station_cd = ? LIMIT 1")
            .bind(id)
            .fetch_one(self.pool)
            .await
            .ok()
    }

    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<i32>,
    ) -> Option<Vec<StationEntity>> {
        sqlx::query_as!(
            StationEntity,
            "SELECT *,
        (
          6371 * acos(
          cos(radians(?))
          * cos(radians(lat))
          * cos(radians(lon) - radians(?))
          + sin(radians(?))
          * sin(radians(lat))
          )
        ) AS distance
        FROM
        stations as s1
        WHERE
        e_status = 0
        AND
        station_cd = (
          SELECT station_cd 
          FROM stations as s2
          WHERE s1.station_g_cd = s2.station_g_cd
          LIMIT 1
        )
        ORDER BY
        distance
        LIMIT ?",
            latitude,
            longitude,
            latitude,
            limit.unwrap_or(1)
        )
        .fetch_all(self.pool)
        .await
        .ok()
    }

    async fn get_by_group_id(&self, group_id: u32) -> Option<Vec<StationEntity>> {
        sqlx::query_as::<_, StationEntity>("SELECT * FROM stations WHERE station_g_cd = ?")
            .bind(group_id)
            .fetch_all(self.pool)
            .await
            .ok()
    }

    async fn get_lines_by_station_id(&self, station_id: u32) -> Option<Vec<LineEntity>> {
        sqlx::query_as::<_, LineEntity>(
            "SELECT l.* FROM `lines` AS l WHERE EXISTS
            (SELECT * FROM stations AS s1 WHERE s1.station_g_cd IN
            (SELECT station_g_cd FROM stations WHERE station_cd = ?)
            AND l.line_cd = s1.line_cd AND e_status = 0)
            ORDER BY l.e_sort, l.line_cd",
        )
        .bind(station_id)
        .fetch_all(self.pool)
        .await
        .ok()
    }

    async fn get_lines_by_station_ids(&self, station_ids: Vec<u32>) -> Option<Vec<LineEntity>> {
        let params = format!("?{}", ", ?".repeat(station_ids.len() - 1));
        let query_str = format!(
            "SELECT l.* FROM `lines` AS l WHERE EXISTS
            (SELECT * FROM stations AS s1 WHERE s1.station_g_cd IN
            (SELECT station_g_cd FROM stations WHERE station_cd IN ({})
            AND l.line_cd = s1.line_cd AND e_status = 0)
            ORDER BY l.e_sort, l.line_cd",
            params
        );
        let mut query = sqlx::query_as::<_, LineEntity>(&query_str);

        for id in station_ids {
            query = query.bind(id);
        }

        query.fetch_all(self.pool).await.ok()
    }

    async fn get_lines_by_line_ids(&self, line_ids: Vec<u32>) -> Option<Vec<LineEntity>> {
        let params = format!("?{}", ", ?".repeat(line_ids.len() - 1));
        let query_str = format!(
            "SELECT *
            FROM `lines`
            WHERE line_cd IN ({})
            AND e_status = 0",
            params
        );
        let mut query = sqlx::query_as::<_, LineEntity>(&query_str);

        for id in line_ids {
            query = query.bind(id);
        }

        query.fetch_all(self.pool).await.ok()
    }

    async fn find_one_line_by_station_id(&self, station_id: u32) -> Option<LineEntity> {
        sqlx::query_as::<_, LineEntity>("SELECT * FROM `lines` WHERE line_cd=(SELECT line_cd FROM stations WHERE station_cd = ?) LIMIT 1")
            .bind(station_id)
            .fetch_one(self.pool)
            .await
            .ok()
    }

    async fn get_companies_by_line_ids(&self, line_ids: Vec<u32>) -> Option<Vec<CompanyEntity>> {
        let params = format!("?{}", ", ?".repeat(line_ids.len() - 1));
        let query_str = format!(
            "SELECT c.*, l.line_cd
        FROM `lines` as l, `companies` as c
        WHERE l.line_cd IN ({})
        AND l.e_status = 0
        AND c.company_cd = l.company_cd",
            params
        );
        let mut query = sqlx::query_as::<_, CompanyEntity>(&query_str);

        for id in line_ids {
            query = query.bind(id);
        }

        query.fetch_all(self.pool).await.ok()
    }
}
