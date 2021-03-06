//! Tests for the `conv` subcommand.

use cli_test_dir::*;
use std::fs;

/// An example Postgres SQL `CREATE TABLE` declaration.
const EXAMPLE_SQL: &str = include_str!("../../fixtures/example.sql");

/// Sample input SQL. We test against this, and not against a running copy of
/// PostgreSQL, because it keeps the test environment much simpler. But this
/// means we don't fully test certain modes of the CLI (though we have unit
/// tests for much of the related code).
const INPUT_SQL: &str = include_str!(
    "../../../dbcrossbarlib/src/drivers/postgres_shared/create_table_sql_example.sql"
);

#[test]
fn conv_help_flag() {
    let testdir = TestDir::new("dbcrossbar", "conv_help_flag");
    let output = testdir.cmd().args(&["conv", "--help"]).expect_success();
    assert!(output.stdout_str().contains("EXAMPLE LOCATORS:"));
}

#[test]
fn conv_pg_sql_to_pg_sql() {
    let testdir = TestDir::new("dbcrossbar", "conv_pg_sql_to_pg_sql");
    let output = testdir
        .cmd()
        .args(&["conv", "postgres-sql:-", "postgres-sql:-"])
        .output_with_stdin(EXAMPLE_SQL)
        .expect_success();
    assert!(output.stdout_str().contains("CREATE TABLE"));
}

#[test]
fn conv_pg_sql_to_dbcrossbar_schema_to_pg_sql() {
    let testdir = TestDir::new("dbcrossbar", "conv_pg_sql_to_pg_sql");
    let output1 = testdir
        .cmd()
        .args(&["conv", "postgres-sql:-", "dbcrossbar-schema:-"])
        .output_with_stdin(EXAMPLE_SQL)
        .expect_success();
    let output2 = testdir
        .cmd()
        .args(&["conv", "dbcrossbar-schema:-", "postgres-sql:-"])
        .output_with_stdin(output1.stdout_str())
        .expect_success();
    assert!(output2.stdout_str().contains("CREATE TABLE"));

    // And make sure it round-trips.
    let output3 = testdir
        .cmd()
        .args(&["conv", "postgres-sql:-", "dbcrossbar-schema:-"])
        .output_with_stdin(output2.stdout_str())
        .expect_success();
    assert_eq!(output3.stdout_str(), output1.stdout_str());
}

#[test]
fn conv_csv_to_pg_sql() {
    let testdir = TestDir::new("dbcrossbar", "conv_csv_to_pg_sql");
    let src = testdir.src_path("fixtures/example.csv");
    let output = testdir
        .cmd()
        .args(&["conv", &format!("csv:{}", src.display()), "postgres-sql:-"])
        .output()
        .expect_success();
    assert!(output.stdout_str().contains("CREATE TABLE"));
    assert!(output.stdout_str().contains("id"));
    assert!(output.stdout_str().contains("first_name"));
    assert!(output.stdout_str().contains("last_name"));
}

#[test]
fn conv_pg_sql_to_bq_schema() {
    let testdir = TestDir::new("dbcrossbar", "conv_pg_sql_to_bq_schema");
    let output = testdir
        .cmd()
        .args(&["conv", "postgres-sql:-", "bigquery-schema:-"])
        .output_with_stdin(INPUT_SQL)
        .expect_success();
    assert!(output.stdout_str().contains("GEOGRAPHY"));
    assert!(output.stdout_str().contains("REPEATED"));
}

#[test]
fn conv_bq_schema_to_pg_sql() {
    let testdir = TestDir::new("dbcrossbar", "conv_bq_schema_to_pg_sql");
    let input_json = testdir.src_path("fixtures/bigquery_schema.json");
    let expected_sql = testdir.src_path("fixtures/bigquery_schema_converted.sql");
    testdir
        .cmd()
        .args(&[
            "conv",
            &format!("bigquery-schema:{}", input_json.display()),
            "postgres-sql:output.sql",
        ])
        .expect_success();
    let expected = fs::read_to_string(&expected_sql).unwrap();
    testdir.expect_file_contents("output.sql", &expected);
}
