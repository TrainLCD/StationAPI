//! BFF のスキーマにある `scalar UInt32` に対応する型。
//!
//! proto 側は uint32 だが GraphQL の Int は 32bit 符号付きなので、
//! 2^31 以上を表すために独自スカラーとして定義されている。
//! 仮想 lineGroupId は uint32 の上位半分を使うため、ここは必須。

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UInt32(pub u32);

#[Scalar(name = "UInt32")]
impl ScalarType for UInt32 {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::Number(n) => n
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .map(UInt32)
                .ok_or_else(|| InputValueError::custom("UInt32 の範囲外です")),
            other => Err(InputValueError::expected_type(other)),
        }
    }

    fn to_value(&self) -> Value {
        Value::Number(self.0.into())
    }
}

impl From<u32> for UInt32 {
    fn from(value: u32) -> Self {
        UInt32(value)
    }
}
