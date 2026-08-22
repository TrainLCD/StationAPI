//! StationAPI のドメインとユースケース。
//!
//! 配信 (GraphQL) は Cloudflare Workers 上で動くルートの crate が、
//! Worker が読むデータの生成は preprocessor crate が担う。この crate は
//! どちらからも参照されるため、wasm32 でビルドできる範囲に留めている。

pub mod domain;
pub mod model;
pub mod use_case;
