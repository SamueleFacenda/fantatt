//! Database-backed identity tests (goal G1.6): a resolution writes the player
//! and the rating it inherits, an ambiguity queues every candidate, a second
//! rating for the same day is rejected instead of overwritten, and only an
//! audited admin decision resolves a queued ambiguity.

use fantatt_core::identity::{Alias, Evidence};
use fantatt_core::ids::{DomainPrefix, Identifier};
use fantatt_core::rating::{Rating, RatingConfidence, RatingProvenance};
use fantatt_core::reconciliation::{Candidate, MatchScope, ReconciliationRequest, reconcile};
use fantatt_jobs::identity::{
    Context, Decision, IdentityError, IdentityStore, NewPlayer, RatingRow,
};
use fantatt_test_support::TestDatabase;
use time::Date;

const ACTOR_ID: &str = "usr_01ARZ3XK7M0C4V6B8N2QWE9RTA";

const COMPETITION: &str = "01ARZ3XK7M0C4V6B8N2QWE9RTC";

fn day() -> Date {
    Date::from_calendar_date(2026, time::Month::March, 10).expect("a valid test day")
}

fn identity(suffix: char) -> Identifier {
    let ulid = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{suffix}");
    Identifier::new(DomainPrefix::Player, &ulid).expect("valid id")
}

fn competition() -> Identifier {
    Identifier::new(DomainPrefix::Competition, COMPETITION).expect("id")
}

fn entry(classifica_id: &str, points: i64) -> Evidence {
    let region = "TRENTINO";
    let entry = fantatt_core::identity::ClassificaEntry::new(classifica_id, points, region);
    Evidence::new(entry, competition(), 12)
}

/// One candidate a name could resolve to, with the classifica it sits in.
fn candidate(suffix: char, name: &str, points: i64) -> Candidate {
    let id = identity(suffix);
    Candidate::new(id, name, entry("2026-C1", points))
}

fn resolved_candidates(points: i64) -> Vec<Candidate> {
    vec![candidate('A', "BENDONI LORENZO", points)]
}

fn homonyms() -> Vec<Candidate> {
    vec![
        candidate('A', "BENDONI LORENZO", 2400),
        candidate('C', "BENDONI LORENZO", 2500),
    ]
}

fn context() -> Context<'static> {
    context_named("BENDONI LORENZO")
}

fn context_named(name: &'static str) -> Context<'static> {
    Context {
        alias: Alias::new(name),
        gender: "MALE",
        as_of: day(),
        source: "portal",
    }
}

fn request(name: &str, candidates: Vec<Candidate>) -> ReconciliationRequest {
    ReconciliationRequest {
        alias: Alias::new(name),
        candidates,
        scope: MatchScope::default(),
        provisional_rating: Rating::provisional(day(), "portal"),
    }
}

async fn store() -> (TestDatabase, IdentityStore) {
    let database = TestDatabase::new().await.expect("boot");
    let store = IdentityStore::new(database.pool().clone());
    (database, store)
}

/// The deciding admin the foreign keys name; the store only records a
/// decision for a user that exists.
async fn seed_actor(pool: &sqlx::PgPool) {
    sqlx::query!(
        "INSERT INTO fantatt.users (user_id, email, display_name) VALUES ($1, $2, $3)",
        ACTOR_ID,
        "admin@fantatt.example",
        "Administrator"
    )
    .execute(pool)
    .await
    .expect("the deciding admin exists");
}

/// The two identities a homonymy could resolve to, as stored players.
async fn seed_candidates(store: &IdentityStore, suffixes: [char; 2]) {
    for suffix in suffixes {
        let name = "Bendoni Lorenzo";
        let player = NewPlayer {
            player_id: &identity(suffix).to_string(),
            full_name: name,
            gender: "MALE",
            fitet_person_id: None,
        };
        store.ensure_player(player).await.expect("candidate player");
    }
}

async fn rating_value(pool: &sqlx::PgPool, player_id: &str) -> i32 {
    let row = sqlx::query!(
        "SELECT value, confidence FROM fantatt.player_ratings WHERE player_id = $1",
        player_id
    )
    .fetch_one(pool)
    .await
    .expect("rating");
    row.value
}

async fn apply_resolved(store: &IdentityStore) -> fantatt_jobs::identity::Applied {
    let outcome = reconcile(request("BENDONI LORENZO", resolved_candidates(2400)));
    store.apply(outcome, context()).await.expect("apply")
}

async fn apply_unresolved(store: &IdentityStore) -> fantatt_jobs::identity::Applied {
    let outcome = reconcile(request("ROSSI MARCO", resolved_candidates(2400)));
    store
        .apply(outcome, context_named("ROSSI MARCO"))
        .await
        .expect("apply")
}

#[tokio::test]
async fn an_unresolved_alias_stays_provisional_on_rating_2000() {
    let (_database, store) = store().await;
    let applied = apply_unresolved(&store).await;
    assert_eq!(
        applied.state,
        fantatt_core::identity::IdentityState::Unresolved
    );
    assert!(applied.queue_id.is_none());
    assert_eq!(rating_value(store.pool(), &applied.player_id).await, 2000);
}

#[tokio::test]
async fn a_resolved_alias_inherits_the_classifica_rating() {
    let (_database, store) = store().await;
    let applied = apply_resolved(&store).await;
    assert_classifica_rating(&store, &applied).await;
    assert_resolved_alias(&store, &applied.alias_id, &applied.player_id).await;
}

/// The inherited classifica rating: the value, the classifica it came from,
/// and the day it applies to.
async fn assert_classifica_rating(
    store: &IdentityStore,
    applied: &fantatt_jobs::identity::Applied,
) {
    let row = sqlx::query!(
        "SELECT value, source, as_of FROM fantatt.player_ratings WHERE player_id = $1",
        applied.player_id
    )
    .fetch_one(store.pool())
    .await
    .expect("rating");
    assert_eq!(row.value, 2400);
    assert_eq!(row.source, "classifica:2026-C1");
    assert_eq!(row.as_of, day());
}

/// The alias row one identity resolved to, and the player it now names.
async fn assert_resolved_alias(store: &IdentityStore, alias_id: &str, player_id: &str) {
    let alias = sqlx::query!(
        "SELECT player_id, status FROM fantatt.identity_aliases WHERE alias_id = $1",
        alias_id
    )
    .fetch_one(store.pool())
    .await
    .expect("alias");
    assert_eq!(alias.player_id, player_id);
    assert_eq!(alias.status, "RESOLVED");
}

#[tokio::test]
async fn an_ambiguous_alias_queues_every_candidate() {
    let (_database, store) = store().await;
    seed_candidates(&store, ['A', 'C']).await;
    let outcome = reconcile(request("BENDONI LORENZO", homonyms()));
    let applied = store.apply(outcome, context()).await.expect("apply");
    assert_queue_names(&store, &applied.queue_id.expect("queued")).await;

    let pending = store.pending_ambiguities(10).await.expect("pending");
    assert_eq!(pending.len(), 1);
    let listed = pending[0].candidates.as_array().expect("candidate list");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["player_id"], identity('A').to_string());
    assert_eq!(listed[0]["full_name"], "Bendoni Lorenzo");
}

/// The candidates a queued ambiguity names, in the order it lists them.
async fn assert_queue_names(store: &IdentityStore, queue_id: &str) {
    let queued = sqlx::query!(
        "SELECT candidate_player_ids FROM fantatt.identity_ambiguity_queue WHERE queue_id = $1",
        queue_id
    )
    .fetch_one(store.pool())
    .await
    .expect("queue row");
    let expected = vec![identity('A').to_string(), identity('C').to_string()];
    assert_eq!(queued.candidate_player_ids, expected);
}

#[tokio::test]
async fn a_second_rating_for_the_same_day_is_rejected_not_overwritten() {
    let (_database, store) = store().await;
    let applied = apply_resolved(&store).await;
    let clash = duplicate_classifica_rating();
    let error = store
        .record_rating(RatingRow {
            player_id: &applied.player_id,
            rating: &clash,
            valid_from: day(),
        })
        .await
        .expect_err("the same day again");
    assert!(
        matches!(error, IdentityError::DuplicateRating { .. }),
        "{error}"
    );
    assert_rating_count(&store, &applied.player_id).await;
}

/// The one value a player's ratings hold after a rejected second write.
async fn assert_rating_count(store: &IdentityStore, player_id: &str) {
    let rows = sqlx::query!(
        "SELECT value FROM fantatt.player_ratings WHERE player_id = $1",
        player_id
    )
    .fetch_all(store.pool())
    .await
    .expect("ratings");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, 2400);
}

/// A second value for the classifica a link already inherited.
fn duplicate_classifica_rating() -> Rating {
    let provenance = RatingProvenance::new("classifica:2026-C1", RatingConfidence::High);
    Rating::confirmed(2600, day(), provenance).expect("a valid rating")
}

#[tokio::test]
async fn resolving_an_ambiguity_moves_the_alias_and_audits_the_decision() {
    let (_database, store) = store().await;
    seed_candidates(&store, ['A', 'C']).await;
    let applied = queued_apply(&store).await;
    let queue_id = applied.queue_id.expect("queued");
    seed_actor(store.pool()).await;
    let chosen = identity('C').to_string();
    resolve_to(&store, &queue_id, &chosen).await;

    assert_resolved_alias(&store, &applied.alias_id, &chosen).await;
    assert_settled_queue(&store, &queue_id).await;
    assert_player_settled(&store).await;
    assert_audited(&store, &chosen).await;
}

/// The decision an admin takes on a queued ambiguity.
async fn resolve_to(store: &IdentityStore, queue_id: &str, chosen: &str) {
    let decision = Decision {
        resolved_player_id: chosen,
        actor_id: ACTOR_ID,
        note: Some("the Veneto federation confirms the second"),
    };
    store
        .resolve_ambiguity(queue_id, decision)
        .await
        .expect("resolve");
}

/// The settled queue row: who decided, and on what note.
async fn assert_settled_queue(store: &IdentityStore, queue_id: &str) {
    let queue = sqlx::query!(
        "SELECT status, resolved_by, resolution_note FROM fantatt.identity_ambiguity_queue WHERE queue_id = $1",
        queue_id
    )
    .fetch_one(store.pool())
    .await
    .expect("queue");
    assert_eq!(queue.status, "RESOLVED");
    assert_eq!(queue.resolved_by, Some(ACTOR_ID.to_owned()));
    assert_eq!(
        queue.resolution_note,
        Some("the Veneto federation confirms the second".to_owned())
    );
}

/// The chosen player stopped being provisional and left the queue.
async fn assert_player_settled(store: &IdentityStore) {
    let chosen = sqlx::query!(
        "SELECT provisional FROM fantatt.players WHERE player_id = $1",
        identity('C').to_string()
    )
    .fetch_one(store.pool())
    .await
    .expect("chosen player");
    assert!(!chosen.provisional);
    assert!(
        store
            .pending_ambiguities(10)
            .await
            .expect("pending")
            .is_empty()
    );
}

/// The append-only entry the decision leaves behind.
async fn assert_audited(store: &IdentityStore, chosen: &str) {
    let audit = sqlx::query!(
        "SELECT entity_id FROM fantatt.audit_log WHERE action = 'identity.ambiguity.resolved'"
    )
    .fetch_one(store.pool())
    .await
    .expect("audit entry");
    assert_eq!(audit.entity_id, Some(chosen.to_owned()));
}

#[tokio::test]
async fn a_player_outside_the_candidates_cannot_resolve_the_ambiguity() {
    let (_database, store) = store().await;
    seed_candidates(&store, ['A', 'C']).await;
    let queue_id = queued_apply(&store).await.queue_id.expect("queued");
    seed_actor(store.pool()).await;
    let outside = identity('B').to_string();

    let error = refuse_outside_player(&store, &queue_id, &outside).await;
    assert!(
        matches!(error, IdentityError::CandidateNotListed),
        "{error}"
    );
    assert_queue_pending(&store, &queue_id).await;
}

/// The refusal one player outside the candidates collects.
async fn refuse_outside_player(
    store: &IdentityStore,
    queue_id: &str,
    outside: &str,
) -> IdentityError {
    let decision = Decision {
        resolved_player_id: outside,
        actor_id: ACTOR_ID,
        note: None,
    };
    store
        .resolve_ambiguity(queue_id, decision)
        .await
        .expect_err("outside the candidates")
}

/// A refused decision leaves the queue exactly as it was.
async fn assert_queue_pending(store: &IdentityStore, queue_id: &str) {
    let queued = sqlx::query!(
        "SELECT status FROM fantatt.identity_ambiguity_queue WHERE queue_id = $1",
        queue_id
    )
    .fetch_one(store.pool())
    .await
    .expect("queue");
    assert_eq!(queued.status, "PENDING");
}

#[tokio::test]
async fn an_ambiguity_is_resolved_once() {
    let (_database, store) = store().await;
    seed_candidates(&store, ['A', 'C']).await;
    let applied = queued_apply(&store).await;
    let queue_id = applied.queue_id.expect("queued");
    seed_actor(store.pool()).await;
    let decision = Decision {
        resolved_player_id: &identity('A').to_string(),
        actor_id: ACTOR_ID,
        note: None,
    };
    store
        .resolve_ambiguity(&queue_id, decision.clone())
        .await
        .expect("resolve");
    let error = store
        .resolve_ambiguity(&queue_id, decision)
        .await
        .expect_err("already resolved");
    assert!(matches!(error, IdentityError::QueueNotPending), "{error}");
}

#[tokio::test]
async fn a_repeated_alias_updates_one_row() {
    let (_database, store) = store().await;
    for _ in 0..2 {
        let outcome = reconcile(request("ROSSI MARCO", resolved_candidates(2400)));
        store
            .apply(outcome, context_named("ROSSI MARCO"))
            .await
            .expect("apply");
    }
    assert_alias_rows(&store, "Rossi Marco").await;
    assert_provisional_ratings(&store).await;
}

/// The one alias row a repeat application leaves.
async fn assert_alias_rows(store: &IdentityStore, normalised: &str) {
    let rows = sqlx::query!(
        "SELECT alias_id FROM fantatt.identity_aliases WHERE normalized_alias = $1",
        normalised
    )
    .fetch_all(store.pool())
    .await
    .expect("aliases");
    assert_eq!(rows.len(), 1);
}

/// The single provisional rating a repeat application re-asserts.
async fn assert_provisional_ratings(store: &IdentityStore) {
    let ratings = sqlx::query!(
        "SELECT value FROM fantatt.player_ratings WHERE as_of = $1",
        day()
    )
    .fetch_all(store.pool())
    .await
    .expect("ratings");
    assert_eq!(ratings.len(), 1);
    assert_eq!(ratings[0].value, 2000);
}

#[tokio::test]
async fn an_unresolved_player_stays_provisional_without_a_queue_row() {
    let (_database, store) = store().await;
    let applied = apply_unresolved(&store).await;
    assert!(applied.queue_id.is_none());

    let player = sqlx::query!(
        "SELECT provisional, full_name FROM fantatt.players WHERE player_id = $1",
        applied.player_id
    )
    .fetch_one(store.pool())
    .await
    .expect("player");
    assert!(player.provisional);
    assert_eq!(player.full_name, "Rossi Marco");
}

/// The resolution two plausible homonyms produce, already written.
async fn queued_apply(store: &IdentityStore) -> fantatt_jobs::identity::Applied {
    let outcome = reconcile(request("BENDONI LORENZO", homonyms()));
    store.apply(outcome, context()).await.expect("apply")
}
