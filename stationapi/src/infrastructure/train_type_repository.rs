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
            sst.id,
            sst.station_cd,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass
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
            sst.id,
            sst.station_cd,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass
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
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.id,
            sst.station_cd,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass
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
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.id,
            sst.station_cd,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass
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
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.id,
            sst.station_cd,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass
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
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            t.kind,
            sst.id,
            sst.station_cd,
            sst.type_cd,
            sst.line_group_cd,
            sst.pass
            FROM 
            types as t
            JOIN station_station_types AS sst ON sst.line_group_cd IN ( {params} ) AND sst.pass <> 1 AND sst.type_cd = t.type_cd
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
    use sqlx::PgPool;
    use std::env;

    // テスト用のヘルパー関数
    async fn setup_test_db() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://test:test@localhost/stationapi_test".to_string());

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn setup_test_data(pool: &PgPool) {
        // テスト用のテーブルとデータを作成
        sqlx::query("DROP TABLE IF EXISTS station_station_types CASCADE")
            .execute(pool)
            .await
            .unwrap();

        sqlx::query("DROP TABLE IF EXISTS types CASCADE")
            .execute(pool)
            .await
            .unwrap();

        // テーブル作成
        sqlx::query(
            "CREATE TABLE types (
                type_cd INTEGER PRIMARY KEY,
                type_name VARCHAR(255) NOT NULL,
                type_name_k VARCHAR(255) NOT NULL,
                type_name_r VARCHAR(255),
                type_name_zh VARCHAR(255),
                type_name_ko VARCHAR(255),
                color VARCHAR(7) NOT NULL,
                direction INTEGER,
                kind INTEGER
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE station_station_types (
                id SERIAL PRIMARY KEY,
                station_cd INTEGER NOT NULL,
                type_cd INTEGER,
                line_group_cd INTEGER,
                pass INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(pool)
        .await
        .unwrap();

        // テストデータの挿入
        sqlx::query(
            "INSERT INTO types (type_cd, type_name, type_name_k, type_name_r, type_name_zh, type_name_ko, color, direction, kind) VALUES 
            (1, 'Express', '特急', 'Express', '特快', '특급', '#FF0000', 1, 1),
            (2, 'Local', '普通', 'Local', '普通', '보통', '#00FF00', 0, 2),
            (3, 'Rapid', '快速', 'Rapid', '快速', '급행', '#0000FF', 1, 3)"
        )
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO station_station_types (station_cd, type_cd, line_group_cd, pass) VALUES 
            (101, 1, 301, 0),
            (102, 2, 302, 0),
            (103, 3, 303, 1),
            (104, 1, 304, 0),
            (105, 2, 305, 0)",
        )
        .execute(pool)
        .await
        .unwrap();
    }

    async fn cleanup_test_data(pool: &PgPool) {
        sqlx::query("DROP TABLE IF EXISTS station_station_types CASCADE")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DROP TABLE IF EXISTS types CASCADE")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_train_type_row_to_train_type_conversion() {
        let train_type_row = TrainTypeRow {
            id: Some(1),
            station_cd: Some(101),
            type_cd: Some(1),
            line_group_cd: Some(301),
            pass: Some(0),
            type_name: "Express".to_string(),
            type_name_k: "特急".to_string(),
            type_name_r: Some("Express".to_string()),
            type_name_zh: Some("特快".to_string()),
            type_name_ko: Some("특급".to_string()),
            color: "#FF0000".to_string(),
            direction: Some(1),
            kind: Some(1),
        };

        let train_type: TrainType = train_type_row.into();

        assert_eq!(train_type.id, Some(1));
        assert_eq!(train_type.station_cd, Some(101));
        assert_eq!(train_type.type_cd, Some(1));
        assert_eq!(train_type.line_group_cd, Some(301));
        assert_eq!(train_type.pass, Some(0));
        assert_eq!(train_type.type_name, "Express");
        assert_eq!(train_type.type_name_k, "特急");
        assert_eq!(train_type.type_name_r, Some("Express".to_string()));
        assert_eq!(train_type.type_name_zh, Some("特快".to_string()));
        assert_eq!(train_type.type_name_ko, Some("특급".to_string()));
        assert_eq!(train_type.color, "#FF0000");
        assert_eq!(train_type.direction, Some(1));
        assert_eq!(train_type.kind, Some(1));
    }

    #[tokio::test]
    async fn test_train_type_row_to_train_type_conversion_with_nulls() {
        let train_type_row = TrainTypeRow {
            id: None,
            station_cd: None,
            type_cd: None,
            line_group_cd: None,
            pass: None,
            type_name: "Local".to_string(),
            type_name_k: "普通".to_string(),
            type_name_r: None,
            type_name_zh: None,
            type_name_ko: None,
            color: "#00FF00".to_string(),
            direction: None,
            kind: None,
        };

        let train_type: TrainType = train_type_row.into();

        assert_eq!(train_type.id, None);
        assert_eq!(train_type.station_cd, None);
        assert_eq!(train_type.type_cd, None);
        assert_eq!(train_type.line_group_cd, None);
        assert_eq!(train_type.pass, None);
        assert_eq!(train_type.type_name, "Local");
        assert_eq!(train_type.type_name_k, "普通");
        assert_eq!(train_type.type_name_r, None);
        assert_eq!(train_type.type_name_zh, None);
        assert_eq!(train_type.type_name_ko, None);
        assert_eq!(train_type.color, "#00FF00");
        assert_eq!(train_type.direction, None);
        assert_eq!(train_type.kind, None);
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_id_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalTrainTypeRepository::get_by_station_id(101, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // station_cd が設定されていることを確認
        for train_type in &train_types {
            assert_eq!(train_type.station_cd, Some(101));
        }

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_id_not_found() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalTrainTypeRepository::get_by_station_id(999, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        assert_eq!(train_types.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_id_excludes_pass() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let result = InternalTrainTypeRepository::get_by_station_id(103, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        // pass = 1 なので除外される
        assert_eq!(train_types.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_id_vec_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let station_ids = vec![101, 102];
        let result =
            InternalTrainTypeRepository::get_by_station_id_vec(&station_ids, None, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // 指定した駅のみが含まれることを確認
        for train_type in &train_types {
            assert!(train_type.station_cd == Some(101) || train_type.station_cd == Some(102));
        }

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_station_id_vec_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let station_ids = vec![];
        let result =
            InternalTrainTypeRepository::get_by_station_id_vec(&station_ids, None, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        assert_eq!(train_types.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_vec_success() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let line_group_ids = vec![301, 302];
        let result =
            InternalTrainTypeRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        // 指定した路線グループのみが含まれることを確認
        for train_type in &train_types {
            assert!(train_type.line_group_cd == Some(301) || train_type.line_group_cd == Some(302));
        }

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_vec_empty() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let line_group_ids = vec![];
        let result =
            InternalTrainTypeRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        assert_eq!(train_types.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_get_by_line_group_id_vec_excludes_pass() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let mut conn = pool.acquire().await.unwrap();
        let line_group_ids = vec![303]; // pass = 1 のデータ
        let result =
            InternalTrainTypeRepository::get_by_line_group_id_vec(&line_group_ids, &mut conn).await;

        assert!(result.is_ok());
        let train_types = result.unwrap();
        // pass = 1 なので除外される
        assert_eq!(train_types.len(), 0);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_my_train_type_repository_new() {
        let database_url = "postgres://test:test@localhost/stationapi_test";
        let pool = PgPool::connect(database_url).await;

        if let Ok(pool) = pool {
            let pool = Arc::new(pool);
            let repository = MyTrainTypeRepository::new(pool.clone());

            // プールが正しく設定されていることを確認
            assert!(Arc::ptr_eq(&repository.pool, &pool));
        }
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_train_type_repository_get_by_station_id() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyTrainTypeRepository::new(pool);

        let result = repository.get_by_station_id(101).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_train_type_repository_get_by_station_id_vec() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyTrainTypeRepository::new(pool);

        let station_ids = vec![101, 102];
        let result = repository.get_by_station_id_vec(&station_ids, None).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_train_type_repository_get_by_line_group_id_vec() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyTrainTypeRepository::new(pool);

        let line_group_ids = vec![301, 302];
        let result = repository.get_by_line_group_id_vec(&line_group_ids).await;
        assert!(result.is_ok());

        let train_types = result.unwrap();
        assert!(!train_types.is_empty());

        cleanup_test_data(&repository.pool).await;
    }

    #[tokio::test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    async fn test_my_train_type_repository_type_conversion() {
        let pool = setup_test_db().await;
        setup_test_data(&pool).await;

        let pool = Arc::new(pool);
        let repository = MyTrainTypeRepository::new(pool);

        // u32 から i64 への変換をテスト
        let result = repository.get_by_station_id(101u32).await;
        assert!(result.is_ok());

        let station_ids: Vec<u32> = vec![101, 102];
        let result = repository.get_by_station_id_vec(&station_ids, None).await;
        assert!(result.is_ok());

        let line_group_ids: Vec<u32> = vec![301, 302];
        let result = repository.get_by_line_group_id_vec(&line_group_ids).await;
        assert!(result.is_ok());

        cleanup_test_data(&repository.pool).await;
    }
}
