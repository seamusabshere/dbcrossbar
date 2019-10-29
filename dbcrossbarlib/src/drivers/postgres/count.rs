//! Implementation of `count`, but as a real `async` function.

use super::{connect, PostgresLocator};
use crate::common::*;
use crate::drivers::postgres_shared::PgCreateTable;

/// Implementation of `count`, but as a real `async` function.
pub(crate) async fn count_helper(
    ctx: Context,
    locator: PostgresLocator,
    shared_args: SharedArguments<Unverified>,
    source_args: SourceArguments<Unverified>,
) -> Result<usize> {
    let shared_args = shared_args.verify(PostgresLocator::features())?;
    let source_args = source_args.verify(PostgresLocator::features())?;

    // Get the parts of our locator.
    let url = locator.url.clone();
    let table_name = locator.table_name.clone();

    // Look up the arguments we'll need.
    let schema = shared_args.schema();

    // Convert our schema to a native PostgreSQL schema.
    let pg_create_table =
        PgCreateTable::from_name_and_columns(table_name.clone(), &schema.columns)?;

    // Generate SQL for query.
    let mut sql_bytes: Vec<u8> = vec![];
    pg_create_table.write_count_sql(&mut sql_bytes, &source_args)?;
    let sql = String::from_utf8(sql_bytes).expect("should always be UTF-8");
    debug!(ctx.log(), "count SQL: {}", sql);

    // Run our query.
    let mut conn = connect(ctx.clone(), url).await?;
    let stmt = conn.prepare(&sql).compat().await?;
    let rows = conn
        .query(&stmt, &[])
        .collect()
        .compat()
        .await
        .context("error running count query")?;
    if rows.len() != 1 {
        Err(format_err!(
            "expected 1 row of count output, got {}",
            rows.len(),
        ))
    } else {
        let count: i64 = rows[0].get("count");
        Ok(cast::usize(count).context("count out of range")?)
    }
}
