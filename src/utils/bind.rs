use rorm_sql::value;
use sqlx::database::HasArguments;
use sqlx::query::Query;

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
        value::Value::Null => {
            static NULL: Option<bool> = None;
            query.bind(NULL)
        }
        value::Value::Ident(_) => query,
    }
}
