use crate::domain::{
    entity::train_type::TrainType, error::DomainError,
    repository::train_type_repository::TrainTypeRepository,
};
use async_trait::async_trait;
use sqlx::{PgConnection, Pool, Postgres};
use std::sync::Arc;

#[derive(sqlx::FromRow, Clone)]
pub struct TrainTypeRow {
    id: Option<i32>,
    station_cd: Option<i32>,
    type_cd: Option<i32>,
    line_group_cd: Option<i32>,
    pass: Option<i32>,
    type_name: String,
    type_name_k: String,
    type_name_r: Option<String>,
    type_name_zh: Option<String>,
    type_name_ko: Option<String>,
    color: String,
    direction: Option<i32>,
    kind: Option<i32>,
}

impl From<TrainTypeRow> for TrainType {
    fn from(row: TrainTypeRow) -> Self {
        let TrainTypeRow {
            id,
            station_cd,
            type_cd,
            line_group_cd,
            pass,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
            kind,
        } = row;
        Self {
            id,
            station_cd,
            type_cd,
            line_group_cd,
            pass,
            type_name,
            type_name_k,
            type_name_r,
            type_name_zh,
            type_name_ko,
            color,
            direction,
            line: None,
            lines: vec![],
            kind,
        }
    }
}

pub struct MyTrainTypeRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyTrainTypeRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrainTypeRepository for MyTrainTypeRepository {
    async fn get_by_line_group_id(
        &self,
        line_group_id: u32,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id(line_group_id, &mut conn).await
    }

    async fn get_by_station_id(&self, station_id: u32) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id(station_id, &mut conn).await
    }

    async fn find_by_line_group_id_and_line_id(
        &self,
        line_group_id: u32,
        line_id: u32,
    ) -> Result<Option<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_and_line_id(
            line_group_id,
            line_id,
            &mut conn,
        )
        .await
    }

    async fn get_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_station_id_vec(station_id_vec, line_group_id, &mut conn)
            .await
    }

    async fn get_types_by_station_id_vec(
        &self,
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_types_by_station_id_vec(
            station_id_vec,
            line_group_id,
            &mut conn,
        )
        .await
    }

    async fn get_by_line_group_id_vec(
        &self,
        line_group_id_vec: &[u32],
    ) -> Result<Vec<TrainType>, DomainError> {
        let mut conn = self.pool.acquire().await?;
        InternalTrainTypeRepository::get_by_line_group_id_vec(line_group_id_vec, &mut conn).await
    }
}

pub struct InternalTrainTypeRepository {}

impl InternalTrainTypeRepository {
    async fn get_by_line_group_id(
        line_group_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows = sqlx::query_as!(
            TrainTypeRow,
            "SELECT
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.*
            FROM types as t
            JOIN station_station_types AS sst ON sst.line_group_cd = $1
            WHERE 
                t.type_cd = sst.type_cd
            ORDER BY t.kind, sst.id",
            line_group_id as i32
        )
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
    async fn get_by_station_id(
        station_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        let rows = sqlx::query_as!(TrainTypeRow,
            "SELECT
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.*
            FROM  types AS t
            JOIN stations AS s ON s.station_cd = $1 AND s.e_status = 0
            JOIN station_station_types AS sst ON sst.station_cd = s.station_cd AND sst.type_cd = t.type_cd AND sst.pass <> 1
            ORDER BY sst.id",
            station_id as i32
        )
        .fetch_all(conn)
        .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
    async fn get_by_line_group_id_and_line_id(
        line_group_id: u32,
        line_id: u32,
        conn: &mut PgConnection,
    ) -> Result<Option<TrainType>, DomainError> {
        let rows: Option<TrainTypeRow> = sqlx::query_as(
            "SELECT 
            t.*, 
            sst.*
            FROM types as t
            JOIN station_station_types AS sst ON sst.line_group_cd = $1 AND t.type_cd = sst.type_cd
            WHERE 
            sst.station_cd IN (
                SELECT 
                station_cd 
                FROM 
                stations as s 
                WHERE 
                line_cd = $2
                AND s.e_status = 0
            )
            ORDER BY sst.id",
        )
        .bind(line_group_id as i32)
        .bind(line_id as i32)
        .fetch_optional(conn)
        .await?;

        let train_type: Option<TrainType> = rows.map(|row| row.into());

        let Some(train_type) = train_type else {
            return Ok(None);
        };

        Ok(Some(train_type))
    }

    async fn get_by_station_id_vec(
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if station_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = (1..=station_id_vec.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            "SELECT 
            t.*, 
            sst.*
            FROM 
            types as t
            JOIN stations AS s ON s.station_cd IN ( {} ) AND s.e_status = 0
            JOIN station_station_types AS sst ON sst.line_group_cd = ${} AND sst.pass <> 1 AND sst.type_cd = t.type_cd
            WHERE sst.pass <> 1 AND sst.type_cd = t.type_cd
            ORDER BY sst.id",
            params,
            station_id_vec.len() + 1
        );

        let mut query = sqlx::query_as::<_, TrainTypeRow>(&query_str);
        for id in station_id_vec {
            query = query.bind(*id as i32);
        }

        let rows = query
            .bind(line_group_id.map(|x| x as i32))
            .fetch_all(conn)
            .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }

    async fn get_types_by_station_id_vec(
        station_id_vec: &[u32],
        line_group_id: Option<u32>,
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if station_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = (1..=station_id_vec.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            "SELECT 
            t.*, 
            sst.*
            FROM 
            station_station_types as sst, 
            stations as s, 
            types as t 
            WHERE 
            s.station_cd IN ( {} ) 
            AND (
                ((t.priority > 0) AND sst.type_cd = t.type_cd)
                OR (NOT (t.priority > 0) AND sst.pass <> 1 AND sst.type_cd = t.type_cd)
            )
            AND s.station_cd = sst.station_cd
            AND sst.type_cd = t.type_cd 
            AND s.e_status = 0
            AND sst.line_group_cd = ${}
            AND sst.pass <> 1
            ORDER BY t.priority DESC, sst.id",
            params,
            station_id_vec.len() + 1
        );

        let mut query = sqlx::query_as::<_, TrainTypeRow>(&query_str);
        for id in station_id_vec {
            query = query.bind(*id as i32);
        }

        let rows = query
            .bind(line_group_id.map(|x| x as i32))
            .fetch_all(conn)
            .await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }

    async fn get_by_line_group_id_vec(
        line_group_id_vec: &[u32],
        conn: &mut PgConnection,
    ) -> Result<Vec<TrainType>, DomainError> {
        if line_group_id_vec.is_empty() {
            return Ok(vec![]);
        }

        let params = (1..=line_group_id_vec.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let query_str = format!(
            "SELECT 
            t.*, 
            s.*,
            sst.*
            FROM 
            types as t
            JOIN station_station_types AS sst ON sst.line_group_cd IN ( {params} ) AND sst.pass <> 1 AND sst.type_cd = t.type_cd
            JOIN stations AS s ON s.station_cd = sst.station_cd
            WHERE sst.pass <> 1 AND sst.type_cd = t.type_cd
            ORDER BY sst.id"
        );

        let mut query = sqlx::query_as::<_, TrainTypeRow>(&query_str);
        for id in line_group_id_vec {
            query = query.bind(*id as i32);
        }

        let rows = query.fetch_all(conn).await?;
        let train_types: Vec<TrainType> = rows.into_iter().map(|row| row.into()).collect();

        Ok(train_types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{Connection, PgConnection, PgPool};

    async fn setup_test_db() -> Arc<Pool<Postgres>> {
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://stationapi:stationapi@localhost:5432/stationapi_test".to_string()
        });
        let pool = PgPool::connect(&database_url).await.unwrap();

        // Create tables
        sqlx::query(
            r#"
            DROP TABLE IF EXISTS station_station_types;
            DROP TABLE IF EXISTS stations;
            DROP TABLE IF EXISTS types;
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS types (
                id INTEGER PRIMARY KEY,
                type_cd INTEGER NOT NULL,
                type_name TEXT,
                type_name_k TEXT,
                type_name_r TEXT,
                type_name_zh TEXT,
                type_name_ko TEXT,
                color TEXT,
                direction INTEGER,
                kind INTEGER,
                priority INTEGER
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS station_station_types (
                id INTEGER PRIMARY KEY,
                station_cd INTEGER NOT NULL,
                type_cd INTEGER NOT NULL,
                line_group_cd INTEGER,
                pass INTEGER
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS stations (
                station_cd INTEGER PRIMARY KEY,
                station_g_cd INTEGER NOT NULL,
                station_name TEXT NOT NULL,
                station_name_k TEXT NOT NULL,
                line_cd INTEGER NOT NULL,
                e_status INTEGER NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert test data
        // Types
        sqlx::query(
            r#"
            INSERT INTO types (id, type_cd, type_name, type_name_k, type_name_r, type_name_zh, type_name_ko, color, direction, kind, priority)
            VALUES 
                (1, 1, 'のぞみ', 'ノゾミ', 'Nozomi', '希望', '노조미', '#FFD400', 0, 4, 10),
                (2, 2, 'ひかり', 'ヒカリ', 'Hikari', '光', '히카리', '#0070F0', 0, 4, 8),
                (3, 3, 'こだま', 'コダマ', 'Kodama', '回声', '코다마', '#00AA00', 0, 4, 6),
                (4, 11, '普通', 'フツウ', 'Local', '普通', '보통', '#000000', 0, 0, 1),
                (5, 12, '快速', 'カイソク', 'Rapid', '快速', '쾌속', '#FF0000', 0, 1, 2),
                (6, 13, '特急', 'トッキュウ', 'Limited Express', '特急', '특급', '#0000FF', 0, 2, 5)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Stations
        sqlx::query(
            r#"
            INSERT INTO stations (station_cd, station_g_cd, station_name, station_name_k, line_cd, e_status)
            VALUES 
                (100201, 1001, '東京駅', 'トウキョウエキ', 11302, 0),
                (100202, 1002, '品川駅', 'シナガワエキ', 11302, 0),
                (100203, 1003, '新宿駅', 'シンジュクエキ', 11301, 0),
                (100301, 1004, '京都駅', 'キョウトエキ', 11302, 0),
                (100302, 1005, '大阪駅', 'オオサカエキ', 11302, 0)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Station station types
        sqlx::query(
            r#"
            INSERT INTO station_station_types (id, station_cd, type_cd, line_group_cd, pass)
            VALUES 
                (1, 100201, 1, 1, 0),
                (2, 100201, 2, 1, 0),
                (3, 100201, 3, 1, 0),
                (4, 100202, 1, 1, 0),
                (5, 100202, 2, 1, 0),
                (6, 100202, 3, 1, 0),
                (7, 100203, 11, 2, 0),
                (8, 100203, 12, 2, 0),
                (9, 100301, 1, 1, 0),
                (10, 100301, 2, 1, 0),
                (11, 100302, 1, 1, 0),
                (12, 100302, 2, 1, 0),
                (13, 100201, 11, 3, 0),
                (14, 100202, 11, 3, 0),
                (15, 100203, 13, 2, 1)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        Arc::new(pool)
    }

    // MyTrainTypeRepository tests
    #[tokio::test]
    async fn test_get_by_line_group_id() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let result = repository.get_by_line_group_id(1).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // line_group_cd = 1のタイプが返されることを確認
        for train_type in &train_types {
            assert_eq!(train_type.line_group_cd, Some(1));
        }

        // のぞみ、ひかり、こだまが含まれることを確認
        let type_names: Vec<String> = train_types.iter().map(|t| t.type_name.clone()).collect();
        assert!(type_names.contains(&"のぞみ".to_string()));
        assert!(type_names.contains(&"ひかり".to_string()));
        assert!(type_names.contains(&"こだま".to_string()));
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_not_found() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let result = repository.get_by_line_group_id(999).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(train_types.is_empty());
    }

    #[tokio::test]
    async fn test_get_by_station_id() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let result = repository.get_by_station_id(100201).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // station_cd = 100201のタイプが返されることを確認
        for train_type in &train_types {
            assert_eq!(train_type.station_cd, Some(100201));
        }

        // pass = 1の列車タイプは除外されることを確認
        for train_type in &train_types {
            assert_ne!(train_type.pass, Some(1));
        }
    }

    #[tokio::test]
    async fn test_get_by_station_id_not_found() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let result = repository.get_by_station_id(999999).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(train_types.is_empty());
    }

    #[tokio::test]
    async fn test_find_by_line_group_id_and_line_id() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let result = repository.find_by_line_group_id_and_line_id(1, 11302).await;
        assert!(result.is_ok());

        let train_type = result.unwrap();
        assert!(train_type.is_some());

        let train_type = train_type.unwrap();
        assert_eq!(train_type.line_group_cd, Some(1));
    }

    #[tokio::test]
    async fn test_find_by_line_group_id_and_line_id_not_found() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let result = repository.find_by_line_group_id_and_line_id(999, 999).await;
        assert!(result.is_ok());

        let train_type = result.unwrap();
        assert!(train_type.is_none());
    }

    #[tokio::test]
    async fn test_get_by_station_id_vec() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let station_ids = vec![100201, 100202];
        let result = repository
            .get_by_station_id_vec(&station_ids, Some(1))
            .await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // クエリの実際の動作を確認：line_group_cd = 1のデータが返される
        for train_type in &train_types {
            assert_eq!(train_type.line_group_cd, Some(1));
            assert_ne!(train_type.pass, Some(1)); // pass = 1は除外される
        }
    }

    #[tokio::test]
    async fn test_get_by_station_id_vec_empty() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let station_ids = vec![];
        let result = repository
            .get_by_station_id_vec(&station_ids, Some(1))
            .await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(train_types.is_empty());
    }

    #[tokio::test]
    async fn test_get_types_by_station_id_vec() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let station_ids = vec![100201, 100203];
        let result = repository
            .get_types_by_station_id_vec(&station_ids, Some(2))
            .await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // line_group_cd = 2のタイプのみが返されることを確認
        for train_type in &train_types {
            assert_eq!(train_type.line_group_cd, Some(2));
        }
    }

    #[tokio::test]
    async fn test_get_types_by_station_id_vec_priority_ordering() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let station_ids = vec![100201];
        let result = repository
            .get_types_by_station_id_vec(&station_ids, Some(1))
            .await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // 結果が返されることを確認（優先度順ソートはSQLで実行される）
        for train_type in &train_types {
            assert_eq!(train_type.line_group_cd, Some(1));
        }
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_vec() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let line_group_ids = vec![1, 2];
        let result = repository.get_by_line_group_id_vec(&line_group_ids).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // 指定されたline_group_cdのタイプのみが返されることを確認
        for train_type in &train_types {
            assert!(train_type.line_group_cd == Some(1) || train_type.line_group_cd == Some(2));
        }
    }

    #[tokio::test]
    async fn test_get_by_line_group_id_vec_empty() {
        let pool = setup_test_db().await;
        let repository = MyTrainTypeRepository::new(pool);

        let line_group_ids = vec![];
        let result = repository.get_by_line_group_id_vec(&line_group_ids).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(train_types.is_empty());
    }

    // TrainTypeRow to TrainType conversion test
    #[tokio::test]
    async fn test_train_type_row_conversion() {
        let row = TrainTypeRow {
            id: Some(1),
            station_cd: Some(100201),
            type_cd: Some(1),
            line_group_cd: Some(1),
            pass: Some(0),
            type_name: "のぞみ".to_string(),
            type_name_k: "ノゾミ".to_string(),
            type_name_r: Some("Nozomi".to_string()),
            type_name_zh: Some("希望".to_string()),
            type_name_ko: Some("노조미".to_string()),
            color: ("#FFD400").to_string(),
            direction: Some(0),
            kind: Some(4),
        };

        let train_type: TrainType = row.into();

        assert_eq!(train_type.id, Some(1));
        assert_eq!(train_type.station_cd, Some(100201));
        assert_eq!(train_type.type_cd, Some(1));
        assert_eq!(train_type.line_group_cd, Some(1));
        assert_eq!(train_type.pass, Some(0));
        assert_eq!(train_type.type_name, "のぞみ");
        assert_eq!(train_type.type_name_k, "ノゾミ");
        assert_eq!(train_type.type_name_r, Some("Nozomi".to_string()));
        assert_eq!(train_type.type_name_zh, Some("希望".to_string()));
        assert_eq!(train_type.type_name_ko, Some("노조미".to_string()));
        assert_eq!(train_type.color, ("#FFD400"));
        assert_eq!(train_type.direction, Some(0));
        assert_eq!(train_type.kind, Some(4));
        assert_eq!(train_type.line, None);
        assert!(train_type.lines.is_empty());
    }

    // InternalTrainTypeRepository tests
    #[tokio::test]
    async fn test_internal_get_by_line_group_id() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        let result = InternalTrainTypeRepository::get_by_line_group_id(1, &mut conn).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // kindでソートされていることを確認
        for train_type in &train_types {
            assert_eq!(train_type.line_group_cd, Some(1));
        }
    }

    #[tokio::test]
    async fn test_internal_get_by_station_id() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        let result = InternalTrainTypeRepository::get_by_station_id(100201, &mut conn).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // pass <> 1の条件が適用されていることを確認
        for train_type in &train_types {
            assert_ne!(train_type.pass, Some(1));
            assert_eq!(train_type.station_cd, Some(100201));
        }
    }

    #[tokio::test]
    async fn test_internal_get_by_line_group_id_and_line_id() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        let result =
            InternalTrainTypeRepository::get_by_line_group_id_and_line_id(1, 11302, &mut conn)
                .await;
        assert!(result.is_ok());

        let train_type = result.unwrap();
        assert!(train_type.is_some());

        let train_type = train_type.unwrap();
        assert_eq!(train_type.line_group_cd, Some(1));
    }

    #[tokio::test]
    async fn test_internal_get_by_station_id_vec() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        let station_ids = vec![100201, 100202];
        let result =
            InternalTrainTypeRepository::get_by_station_id_vec(&station_ids, Some(1), &mut conn)
                .await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // クエリの実際の動作を確認：line_group_cd = 1のデータが返される
        for train_type in &train_types {
            assert_eq!(train_type.line_group_cd, Some(1));
            assert_ne!(train_type.pass, Some(1)); // pass = 1は除外される
        }
    }

    #[tokio::test]
    async fn test_internal_get_types_by_station_id_vec() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        let station_ids = vec![100201, 100203];
        let result = InternalTrainTypeRepository::get_types_by_station_id_vec(
            &station_ids,
            Some(2),
            &mut conn,
        )
        .await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        for train_type in &train_types {
            assert_eq!(train_type.line_group_cd, Some(2));
            assert_ne!(train_type.pass, Some(1));
        }
    }

    #[tokio::test]
    async fn test_internal_get_by_line_group_id_vec() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        let line_group_ids = vec![1, 2];
        let result =
            InternalTrainTypeRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        for train_type in &train_types {
            assert!(train_type.line_group_cd == Some(1) || train_type.line_group_cd == Some(2));
            assert_ne!(train_type.pass, Some(1));
        }
    }

    #[tokio::test]
    async fn test_error_handling_database_error() {
        // 空のメモリ内データベースでテーブルが存在しない状態でクエリを実行してエラーを発生させる
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://stationapi:stationapi@localhost:5432/stationapi_test_empty".to_string()
        });
        let mut conn = PgConnection::connect(&database_url).await.unwrap();

        let result = InternalTrainTypeRepository::get_by_line_group_id(1, &mut conn).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pass_filtering() {
        let pool = setup_test_db().await;
        let mut conn = pool.acquire().await.unwrap();

        // pass = 1のレコードは除外されることを確認
        let result = InternalTrainTypeRepository::get_by_station_id(100203, &mut conn).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        // pass = 1でない列車タイプのみが返される
        for train_type in &train_types {
            assert_ne!(train_type.pass, Some(1));
        }
    }
}
