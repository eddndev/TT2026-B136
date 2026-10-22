use super::{CatalogClient, PreparedCatalog};
use postgres::{Client, NoTls, Row, SimpleQueryMessage};

const RELATIONS: &str = "SELECT c.relname::text, c.relkind::text FROM pg_catalog.pg_class c
    WHERE c.relnamespace OPERATOR(pg_catalog.=) 'pg_catalog'::pg_catalog.regnamespace
        AND c.relname LIKE $1 ORDER BY 1";
const IDENTITY: &str = "SELECT current_user::text, ($1::pg_catalog.int4 + 1)::text";
const LOOKUP: &str = "SELECT relname::text FROM pg_catalog.pg_class WHERE relname = $1";
const DIVIDE: &str = "SELECT (10 / $1::pg_catalog.int4)::text";

fn client() -> Option<Client> {
    let url = std::env::var("CASE_TEST_DATABASE_URL").ok()?;
    Some(Client::connect(&url, NoTls).unwrap())
}

fn texts(rows: &[Row]) -> Vec<Vec<Option<String>>> {
    rows.iter()
        .map(|row| (0..row.len()).map(|index| row.get(index)).collect())
        .collect()
}

/// Runs every query shape the catalog validation uses through any client.
fn sample<C: CatalogClient>(client: &mut C) -> Vec<Vec<Vec<Option<String>>>> {
    let mut results = Vec::new();
    for pattern in ["pg_class%", "pg_attr%", "pg_class%"] {
        results.push(texts(&client.query(RELATIONS, &[&pattern]).unwrap()));
    }
    for value in [41_i32, 1, 41] {
        results.push(texts(&[client.query_one(IDENTITY, &[&value]).unwrap()]));
    }
    for name in ["no_such_relation_for_catalog_test", "pg_class"] {
        let row = client.query_opt(LOOKUP, &[&name]).unwrap();
        results.push(texts(row.as_slice()));
    }
    results
}

fn prepared_statements(client: &mut Client) -> String {
    let messages = client
        .simple_query("SELECT count(*) FROM pg_catalog.pg_prepared_statements")
        .unwrap();
    messages
        .into_iter()
        .find_map(|message| match message {
            SimpleQueryMessage::Row(row) => row.get(0).map(str::to_owned),
            _ => None,
        })
        .unwrap()
}

#[test]
fn prepared_catalog_returns_the_same_results_as_direct_clients() {
    let Some(mut client) = client() else { return };
    let direct = sample(&mut client);
    let mut transaction = client.transaction().unwrap();
    assert_eq!(sample(&mut transaction), direct);
    transaction.rollback().unwrap();
    let mut catalog = PreparedCatalog::new(&mut client);
    assert_eq!(sample(&mut catalog), direct);
    assert_eq!(sample(&mut catalog), direct);
    assert!(!direct[0].is_empty());
    assert_eq!(direct[6], Vec::<Vec<Option<String>>>::new());
    assert_eq!(direct[7], vec![vec![Some("pg_class".to_owned())]]);
}

#[test]
fn prepared_catalog_prepares_each_exact_text_once_per_instance() {
    let Some(mut client) = client() else { return };
    let mut catalog = PreparedCatalog::new(&mut client);
    assert_eq!(catalog.prepares(), 0);
    sample(&mut catalog);
    assert_eq!(catalog.prepares(), 3);
    sample(&mut catalog);
    assert_eq!(catalog.prepares(), 3);
    catalog.query_one(LOOKUP, &[&"pg_class"]).unwrap();
    let spaced = format!("{LOOKUP} ");
    catalog.query_opt(&spaced, &[&"pg_class"]).unwrap();
    assert_eq!(catalog.prepares(), 4);
    drop(catalog);
    let mut next = PreparedCatalog::new(&mut client);
    assert_eq!(next.prepares(), 0);
    sample(&mut next);
    assert_eq!(next.prepares(), 3);
}

#[test]
fn prepared_catalog_propagates_the_same_errors_as_direct_clients() {
    let Some(mut client) = client() else { return };
    let syntax = client.query_one("SELEC 1", &[]).unwrap_err();
    let empty = client
        .query_one(LOOKUP, &[&"no_such_relation"])
        .unwrap_err();
    let division = client.query_one(DIVIDE, &[&0_i32]).unwrap_err();
    let mut catalog = PreparedCatalog::new(&mut client);
    for _ in 0..2 {
        let error = catalog.query_one("SELEC 1", &[]).unwrap_err();
        assert_eq!(error.code(), syntax.code());
        assert_eq!(error.to_string(), syntax.to_string());
    }
    assert_eq!(catalog.prepares(), 2);
    let error = catalog
        .query_one(LOOKUP, &[&"no_such_relation"])
        .unwrap_err();
    assert_eq!(error.to_string(), empty.to_string());
    let value: String = catalog.query_one(DIVIDE, &[&5_i32]).unwrap().get(0);
    assert_eq!(value, "2");
    let error = catalog.query_one(DIVIDE, &[&0_i32]).unwrap_err();
    assert_eq!(error.code(), division.code());
    assert_eq!(error.to_string(), division.to_string());
    assert_eq!(catalog.prepares(), 4);
    let value: String = catalog.query_one(DIVIDE, &[&2_i32]).unwrap().get(0);
    assert_eq!(value, "5");
}

#[test]
fn prepared_catalog_never_reuses_results_and_closes_statements_on_drop() {
    let Some(mut client) = client() else { return };
    let mut administrator = self::client().unwrap();
    let mut catalog = PreparedCatalog::new(&mut client);
    let name = "catalog_client_visibility_probe";
    assert!(catalog.query_opt(LOOKUP, &[&name]).unwrap().is_none());
    administrator
        .batch_execute("CREATE TEMPORARY TABLE catalog_client_visibility_probe ()")
        .unwrap();
    assert!(catalog.query_opt(LOOKUP, &[&name]).unwrap().is_some());
    drop(administrator);
    assert_eq!(catalog.prepares(), 1);
    sample(&mut catalog);
    drop(catalog);
    assert_eq!(prepared_statements(&mut client), "0");
}
