use rorm_sql::value;
use rorm_sql::value::NullType;
use sqlx::database::HasArguments;
use sqlx::query::Query;
use sqlx::types::chrono::{NaiveDate, NaiveDateTime, NaiveTime};

type AnyQuery<'q> = Query<'q, sqlx::Any, <sqlx::Any as HasArguments<'q>>::Arguments>;

/**
This helper method is used to bind ConditionValues to the query.
 */
pub fn bind_param<'post_query, 'query>(
    query: AnyQuery<'query>,
    param: value::Value<'post_query>,
) -> AnyQuery<'query>
where
    'post_query: 'query,
{
    match param {
        value::Value::String(x) => query.bind(x),
        value::Value::I64(x) => query.bind(x),
        value::Value::I32(x) => query.bind(x),
        value::Value::I16(x) => query.bind(x),
        value::Value::Bool(x) => query.bind(x),
        value::Value::F32(x) => query.bind(x),
        value::Value::F64(x) => query.bind(x),
        value::Value::Binary(x) => query.bind(x),
        value::Value::NaiveDate(x) => query.bind(x),
        value::Value::NaiveTime(x) => query.bind(x),
        value::Value::NaiveDateTime(x) => query.bind(x),
        value::Value::Null(null_type) => match null_type {
            NullType::String => query.bind(None::<&str>),
            NullType::I64 => query.bind(None::<i64>),
            NullType::I32 => query.bind(None::<i32>),
            NullType::I16 => query.bind(None::<i16>),
            NullType::Bool => query.bind(None::<bool>),
            NullType::F64 => query.bind(None::<f64>),
            NullType::F32 => query.bind(None::<f32>),
            NullType::Binary => query.bind(None::<&[u8]>),
            NullType::NaiveTime => query.bind(None::<NaiveTime>),
            NullType::NaiveDate => query.bind(None::<NaiveDate>),
            NullType::NaiveDateTime => query.bind(None::<NaiveDateTime>),
        },
        value::Value::Ident(_) => query,
        value::Value::Column { .. } => query,
    }
}
