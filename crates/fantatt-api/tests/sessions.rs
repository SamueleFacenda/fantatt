//! Round trip through the real store on the real table (ADR-0009 decision 3).

use fantatt_test_support::TestDatabase;
use serde_json::{Value, json};
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::SessionStore;

use fantatt_api::sessions::PostgresSessionStore;

fn data(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
}

fn record(id: &Id, data: HashMap<String, Value>, expiry: OffsetDateTime) -> Record {
    Record {
        id: *id,
        data,
        expiry_date: expiry,
    }
}

#[tokio::test]
async fn a_saved_session_loads_back_unchanged() {
    let database = TestDatabase::new().await.expect("boot");
    let store = PostgresSessionStore::new(database.pool().clone());
    let id = Id::default();
    let expiry = OffsetDateTime::now_utc() + Duration::hours(1);
    let saved = record(&id, data(&[("counter", json!(1))]), expiry);
    store.save(&saved).await.expect("save");

    let loaded = store.load(&id).await.expect("load").expect("present");
    assert_eq!(loaded.id, id);
    assert_eq!(loaded.data, data(&[("counter", json!(1))]));
    assert_eq!(loaded.expiry_date.unix_timestamp(), expiry.unix_timestamp());
}

#[tokio::test]
async fn an_unknown_id_loads_nothing() {
    let database = TestDatabase::new().await.expect("boot");
    let store = PostgresSessionStore::new(database.pool().clone());
    assert!(store.load(&Id::default()).await.expect("load").is_none());
}

#[tokio::test]
async fn a_deleted_session_loads_nothing() {
    let database = TestDatabase::new().await.expect("boot");
    let store = PostgresSessionStore::new(database.pool().clone());
    let id = Id::default();
    let expiry = OffsetDateTime::now_utc() + Duration::hours(1);
    store
        .save(&record(&id, data(&[]), expiry))
        .await
        .expect("save");
    store.delete(&id).await.expect("delete");
    assert!(store.load(&id).await.expect("load").is_none());
}

#[tokio::test]
async fn an_expired_record_loads_nothing() {
    let database = TestDatabase::new().await.expect("boot");
    let store = PostgresSessionStore::new(database.pool().clone());
    let id = Id::default();
    let expiry = OffsetDateTime::now_utc() - Duration::hours(1);
    store
        .save(&record(&id, data(&[]), expiry))
        .await
        .expect("save");
    assert!(store.load(&id).await.expect("load").is_none());
}

#[tokio::test]
async fn a_colliding_create_is_refused() {
    let database = TestDatabase::new().await.expect("boot");
    let store = PostgresSessionStore::new(database.pool().clone());
    let id = Id::default();
    let expiry = OffsetDateTime::now_utc() + Duration::hours(1);
    let mut first = record(&id, data(&[]), expiry);
    store.create(&mut first).await.expect("create");
    let mut second = record(&id, data(&[]), expiry);
    assert!(store.create(&mut second).await.is_err());
}
