//! GraphQL スキーマ。BFF が担っていた変換を Worker 側で直接行う。
//! 公開スキーマの正は worker/schema/public.graphql。

pub mod enums;
pub mod query;
pub mod scalar;
pub mod types;

use async_graphql::{EmptyMutation, EmptySubscription, Schema};

use crate::Interactor;
use query::QueryRoot;

pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(interactor: Interactor) -> AppSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(interactor)
        .finish()
}
