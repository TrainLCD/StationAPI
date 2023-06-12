use async_trait::async_trait;
use sqlx::{MySql, Pool};

type LineEntity = crate::entities::line::Line;

#[async_trait]
pub trait LineRepository {
    async fn find_one(&self, id: u32) -> Result<LineEntity, sqlx::Error>;
}

pub struct LineRepositoryImplOnMySQL<'a> {
    pub pool: &'a Pool<MySql>,
}

#[async_trait]
impl LineRepository for LineRepositoryImplOnMySQL<'_> {
    async fn find_one(&self, id: u32) -> Result<LineEntity, sqlx::Error> {
        match sqlx::query_as!(
            LineEntity,
            "SELECT *
        FROM `lines`
        WHERE line_cd = ?
        AND e_status = 0
        LIMIT 1",
            id
        )
        .fetch_one(self.pool)
        .await
        {
            Ok(line) => Ok(line),
            Err(err) => {
                log::error!("Error: {}", err);
                Err(err)
            }
        }
    }
}
