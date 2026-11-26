//! The database side of identity reconciliation (docs/06-data-pipeline.md
//! section 3.6).
//!
//! Reconciliation writes players, bitemporal ratings, aliases, and the
//! ambiguity queue. A rating is stored once per (player, source, as_of) and
//! never overwritten in place (INV-1); an ambiguity is queued with its
//! candidates and only an admin decision, audited in the append-only log,
//! resolves it.

use sqlx::{PgPool, Postgres, Transaction};
use time::Date;

use fantatt_core::identity::{Alias, IdentityState};
use fantatt_core::ids::Identifier;
use fantatt_core::rating::{Rating, RatingConfidence, RatingParseError};
use fantatt_core::reconciliation::{Candidate, Resolution};

use crate::ids::ulid;

/// One player row reconciliation writes: the identity it belongs to, the
/// spelling the source serves, and the FITET person id when one exists.
#[derive(Debug, Clone)]
pub struct NewPlayer<'a> {
    pub player_id: &'a str,
    pub full_name: &'a str,
    pub gender: &'a str,
    pub fitet_person_id: Option<&'a str>,
}

/// One alias row: the spelling the source served and the canonical form it
/// normalises to.
#[derive(Debug, Clone)]
pub struct NewAlias<'a> {
    pub player_id: &'a str,
    pub alias: Alias,
    pub state: IdentityState,
    pub evidence: serde_json::Value,
}

/// One ambiguity waiting for a human decision.
#[derive(Debug, Clone)]
pub struct NewAmbiguity<'a> {
    pub alias_id: &'a str,
    pub candidate_player_ids: &'a [String],
}

/// The admin decision on a queued ambiguity.
#[derive(Debug, Clone)]
pub struct Decision<'a> {
    pub resolved_player_id: &'a str,
    pub actor_id: &'a str,
    pub note: Option<&'a str>,
}

/// One queued ambiguity as the admin surface reads it.
#[derive(Debug, Clone)]
pub struct QueuedAmbiguity {
    pub queue_id: String,
    pub alias_id: String,
    pub raw_alias: String,
    pub normalized_alias: String,
    pub status: String,
    pub candidates: serde_json::Value,
}

/// What one reconciliation wrote.
#[derive(Debug, Clone)]
pub struct Applied {
    pub player_id: String,
    pub alias_id: String,
    pub queue_id: Option<String>,
    pub state: IdentityState,
}

/// The context one reconciliation applies in: the alias it is about, the day
/// the classifica states its value, and the channel the evidence came from.
#[derive(Debug, Clone)]
pub struct Context<'a> {
    pub alias: Alias,
    pub gender: &'a str,
    pub as_of: Date,
    pub source: &'a str,
}

/// One rating row to store: the player it belongs to and the value.
#[derive(Debug, Clone)]
pub struct RatingRow<'a> {
    pub player_id: &'a str,
    pub rating: &'a Rating,
    pub valid_from: Date,
}

/// The single column one insert returns, so the row stays a typed value.
struct AliasId {
    alias_id: String,
}

/// The single column the queue insert returns.
struct QueueId {
    queue_id: String,
}

/// The classifica entry a resolved link inherits its rating from.
struct ClassificaLink<'a> {
    player_id: &'a str,
    candidate: &'a Candidate,
    as_of: Date,
}

/// One ambiguity row as the resolution path reads it.
struct PendingQueue {
    queue_id: String,
    alias_id: String,
    candidate_player_ids: Vec<String>,
}

/// The chosen player an alias moves to.
struct ChosenPlayer<'a> {
    alias_id: &'a str,
    player_id: &'a str,
}

/// One resolved outcome, bundled so the apply path takes one parameter.
struct ResolvedLink<'a> {
    identity: Identifier,
    candidate: Candidate,
    context: Context<'a>,
}

/// The alias and rating a resolved link writes, bundled the same way.
struct LinkWrite<'a> {
    player_id: &'a str,
    candidate: &'a Candidate,
    alias: NewAlias<'a>,
    as_of: Date,
}

/// Every way writing an identity can fail.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("a rating for {as_of} from {rating_source} is already recorded for this player")]
    DuplicateRating { as_of: Date, rating_source: String },
    #[error("the ambiguity is not awaiting a decision any more")]
    QueueNotPending,
    #[error("the chosen player is not a candidate of this ambiguity")]
    CandidateNotListed,
    #[error(transparent)]
    Rating(#[from] RatingParseError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Players, ratings, aliases, and the ambiguity queue over one pool.
pub struct IdentityStore {
    pool: PgPool,
}

impl IdentityStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The pool the identity tables live on.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Writes one player row for a known identity, keeping the spelling the
    /// source serves.
    pub async fn ensure_player(&self, player: NewPlayer<'_>) -> Result<String, IdentityError> {
        insert_player(&self.pool, &player).await
    }

    /// Flags a player as provisional until an identity resolves.
    pub async fn mark_provisional(&self, player_id: &str) -> Result<(), IdentityError> {
        sqlx::query!(
            "UPDATE fantatt.players SET provisional = true, updated_at = now() WHERE player_id = $1",
            player_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The player an already-recorded canonical alias names.
    pub async fn player_by_alias(&self, normalised: &str) -> Result<Option<String>, IdentityError> {
        let row = sqlx::query!(
            "SELECT player_id FROM fantatt.identity_aliases
              WHERE normalized_alias = $1 ORDER BY created_at LIMIT 1",
            normalised
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| row.player_id))
    }

    /// Stores one bitemporal rating: asserted now, valid from the day the
    /// value applies. A second value for the same (player, source, as_of) is
    /// rejected, never overwritten (INV-1).
    pub async fn record_rating(&self, row: RatingRow<'_>) -> Result<(), IdentityError> {
        insert_rating(&self.pool, &row).await.map_err(|error| {
            duplicate_rating(&error, row.rating).unwrap_or_else(|| IdentityError::from(error))
        })
    }

    /// Records one alias for a player; a repeat of the same canonical form
    /// refreshes the status instead of inserting a second row.
    pub async fn record_alias(&self, alias: NewAlias<'_>) -> Result<String, IdentityError> {
        Ok(insert_alias(&self.pool, &alias).await?.alias_id)
    }

    /// Queues one ambiguity for a human decision.
    pub async fn open_ambiguity(&self, queued: NewAmbiguity<'_>) -> Result<String, IdentityError> {
        let row = sqlx::query_as!(
            QueueId,
            "INSERT INTO fantatt.identity_ambiguity_queue
                (queue_id, alias_id, candidate_player_ids)
             VALUES ($1, $2, $3) RETURNING queue_id",
            ulid(),
            queued.alias_id,
            queued.candidate_player_ids
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.queue_id)
    }

    /// The ambiguities still awaiting a decision, newest first, each with the
    /// candidate players it waits on.
    pub async fn pending_ambiguities(
        &self,
        limit: i64,
    ) -> Result<Vec<QueuedAmbiguity>, IdentityError> {
        let rows = sqlx::query_as!(
            QueuedAmbiguity,
            "SELECT q.queue_id, q.alias_id, q.status, a.raw_alias, a.normalized_alias,
                    (SELECT jsonb_agg(jsonb_build_object(
                                'player_id', p.player_id, 'full_name', p.full_name)
                                ORDER BY p.player_id)
                       FROM fantatt.players p
                      WHERE p.player_id = ANY (q.candidate_player_ids)) AS candidates
               FROM fantatt.identity_ambiguity_queue q
               JOIN fantatt.identity_aliases a ON a.alias_id = q.alias_id
              WHERE q.status = 'PENDING'
              ORDER BY q.created_at DESC
              LIMIT $1",
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Resolves a queued ambiguity the way an admin decided: the alias moves
    /// to the chosen player, the player stops being provisional, and the
    /// append-only log records who decided.
    pub async fn resolve_ambiguity(
        &self,
        queue_id: &str,
        decision: Decision<'_>,
    ) -> Result<(), IdentityError> {
        let mut transaction = self.pool.begin().await?;
        let queued = fetch_pending(&mut transaction, queue_id).await?;
        if !is_listed(&queued, decision.resolved_player_id) {
            return Err(IdentityError::CandidateNotListed);
        }
        settle(&mut transaction, &queued, &decision).await?;
        transaction.commit().await?;
        Ok(())
    }

    /// Applies one reconciliation outcome: a resolved alias links to its
    /// player and inherits the classifica rating, an ambiguous one queues its
    /// candidates, an unresolved one stays provisional on rating 2000.
    pub async fn apply(
        &self,
        outcome: Resolution,
        context: Context<'_>,
    ) -> Result<Applied, IdentityError> {
        match outcome {
            Resolution::Resolved {
                identity,
                candidate,
            } => {
                self.apply_resolved(ResolvedLink {
                    identity,
                    candidate,
                    context,
                })
                .await
            }
            queued => self.apply_queued(queued, context).await,
        }
    }

    async fn apply_resolved(&self, link: ResolvedLink<'_>) -> Result<Applied, IdentityError> {
        let player_id = self.ensure_resolved_player(&link).await?;
        let write = resolved_link(&player_id, &link.candidate, link.context);
        self.link_resolved(write).await
    }

    /// Writes the player a resolved identity names, with the spelling the
    /// source served.
    async fn ensure_resolved_player(
        &self,
        link: &ResolvedLink<'_>,
    ) -> Result<String, IdentityError> {
        let identity = link.identity.to_string();
        let full_name = link.candidate.normalised_name().to_owned();
        let player = NewPlayer {
            player_id: &identity,
            full_name: &full_name,
            gender: link.context.gender,
            fitet_person_id: None,
        };
        self.ensure_player(player).await
    }

    /// Writes the alias of a resolved link and the classifica rating it
    /// inherits, in that order so the link exists before its rating.
    async fn link_resolved(&self, write: LinkWrite<'_>) -> Result<Applied, IdentityError> {
        let alias_id = self.record_alias(write.alias).await?;
        let link = ClassificaLink {
            player_id: write.player_id,
            candidate: write.candidate,
            as_of: write.as_of,
        };
        store_classifica_link(&self.pool, link).await?;
        Ok(Applied {
            player_id: write.player_id.to_owned(),
            alias_id,
            queue_id: None,
            state: IdentityState::Resolved,
        })
    }

    async fn apply_queued(
        &self,
        outcome: Resolution,
        context: Context<'_>,
    ) -> Result<Applied, IdentityError> {
        let candidates = ambiguous_candidates(&outcome);
        let state = outcome.state();
        let player_id = self.provisional_player(&context).await?;
        let alias = provisional_alias(&player_id, &context.alias, state);
        let alias_id = self.record_alias(alias).await?;
        let queue_id = self.queue_candidates(&alias_id, &candidates).await?;
        Ok(Applied {
            player_id,
            alias_id,
            queue_id,
            state,
        })
    }

    /// The player an unresolved or ambiguous alias attaches to, with the
    /// provisional rating it plays on until an identity resolves.
    async fn provisional_player(&self, context: &Context<'_>) -> Result<String, IdentityError> {
        let alias = Alias::new(context.alias.name());
        let normalised = alias.normalised().to_owned();
        let player_id = self.player_of_alias(&normalised, context.gender).await?;
        self.attach_provisional(&player_id, context).await?;
        Ok(player_id)
    }

    /// why: a player with no resolved identity plays on rating 2000 and stays
    /// flagged provisional until an admin resolves it. A repeat application
    /// refreshes the alias instead of re-asserting the rating.
    async fn attach_provisional(
        &self,
        player_id: &str,
        context: &Context<'_>,
    ) -> Result<(), IdentityError> {
        if rating_recorded(&self.pool, player_id, context.as_of).await? {
            return self.mark_provisional(player_id).await;
        }
        let rating = Rating::provisional(context.as_of, context.source);
        self.record_rating(RatingRow {
            player_id,
            rating: &rating,
            valid_from: context.as_of,
        })
        .await?;
        self.mark_provisional(player_id).await
    }

    /// The player an unresolved or ambiguous alias attaches to, created when
    /// the alias is first seen.
    async fn player_of_alias(
        &self,
        normalised: &str,
        gender: &str,
    ) -> Result<String, IdentityError> {
        match self.player_by_alias(normalised).await? {
            Some(known) => Ok(known),
            None => {
                self.ensure_player(NewPlayer {
                    player_id: &format!("plr_{}", ulid()),
                    full_name: normalised,
                    gender,
                    fitet_person_id: None,
                })
                .await
            }
        }
    }

    async fn queue_candidates(
        &self,
        alias_id: &str,
        candidates: &[Candidate],
    ) -> Result<Option<String>, IdentityError> {
        if candidates.is_empty() {
            return Ok(None);
        }
        let ids = candidates
            .iter()
            .map(|candidate| candidate.identity().to_string())
            .collect::<Vec<_>>();
        self.open_ambiguity(NewAmbiguity {
            alias_id,
            candidate_player_ids: &ids,
        })
        .await
        .map(Some)
    }
}

/// The alias and rating one resolved link writes: the alias of the incoming
/// spelling beside the classifica entry the rating inherits from.
fn resolved_link<'a>(
    player_id: &'a str,
    candidate: &'a Candidate,
    context: Context<'a>,
) -> LinkWrite<'a> {
    let alias = NewAlias {
        player_id,
        alias: context.alias,
        state: IdentityState::Resolved,
        evidence: serde_json::json!({ "identity": player_id }),
    };
    LinkWrite {
        player_id,
        candidate,
        alias,
        as_of: context.as_of,
    }
}

/// The alias a queued outcome records, carrying no evidence until a decision
/// lands.
fn provisional_alias<'a>(player_id: &'a str, alias: &Alias, state: IdentityState) -> NewAlias<'a> {
    NewAlias {
        player_id,
        alias: Alias::new(alias.name()),
        state,
        evidence: serde_json::json!({}),
    }
}

async fn insert_player(pool: &PgPool, player: &NewPlayer<'_>) -> Result<String, IdentityError> {
    let row = sqlx::query!(
        "INSERT INTO fantatt.players (player_id, fitet_person_id, full_name, gender)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (player_id)
         DO UPDATE SET full_name = EXCLUDED.full_name, updated_at = now()
         RETURNING player_id",
        player.player_id,
        player.fitet_person_id,
        player.full_name,
        player.gender
    )
    .fetch_one(pool)
    .await?;
    Ok(row.player_id)
}

/// The single insert of a rating row, separated so the caller stays small and
/// the constraint violation is reported where the caller is.
async fn insert_rating(pool: &PgPool, row: &RatingRow<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO fantatt.player_ratings
            (rating_id, player_id, value, as_of, source, confidence, valid_from)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        ulid(),
        row.player_id,
        row.rating.value() as i32,
        row.rating.as_of(),
        row.rating.source(),
        confidence_label(row.rating.confidence()),
        row.valid_from
    )
    .execute(pool)
    .await
    .map(|_| ())
}

/// The upsert of one alias row, separated so the reporting method stays small
/// and the conflict branch is read where it happens.
async fn insert_alias(pool: &PgPool, alias: &NewAlias<'_>) -> Result<AliasId, sqlx::Error> {
    sqlx::query_as!(
        AliasId,
        "INSERT INTO fantatt.identity_aliases (alias_id, player_id, raw_alias, normalized_alias, evidence, confidence, status) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (player_id, normalized_alias) DO UPDATE SET raw_alias = EXCLUDED.raw_alias, evidence = EXCLUDED.evidence, confidence = EXCLUDED.confidence, status = EXCLUDED.status, resolved_at = CASE WHEN EXCLUDED.status = 'RESOLVED' THEN now() ELSE NULL END RETURNING alias_id",
        ulid(),
        alias.player_id,
        alias.alias.name(),
        alias.alias.normalised(),
        alias.evidence,
        alias_confidence(alias.state),
        alias.state.to_string()
    )
    .fetch_one(pool)
    .await
}

/// why: reconciliation re-runs over the same page, and a rating is never
/// overwritten in place, so a link only stores the classifica rating when the
/// player holds none for that day yet.
async fn store_classifica_link(
    pool: &PgPool,
    link: ClassificaLink<'_>,
) -> Result<(), IdentityError> {
    if rating_recorded(pool, link.player_id, link.as_of).await? {
        return Ok(());
    }
    let entry = link.candidate.evidence().entry();
    let provenance = fantatt_core::rating::RatingProvenance::new(
        &format!("classifica:{}", entry.classifica_id()),
        RatingConfidence::High,
    );
    let rating = Rating::confirmed(entry.points(), link.as_of, provenance)?;
    let row = RatingRow {
        player_id: link.player_id,
        rating: &rating,
        valid_from: link.as_of,
    };
    insert_rating(pool, &row).await.map_err(IdentityError::from)
}

async fn rating_recorded(pool: &PgPool, player_id: &str, as_of: Date) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT EXISTS (
            SELECT 1 FROM fantatt.player_ratings WHERE player_id = $1 AND as_of = $2
         ) AS recorded",
        player_id,
        as_of
    )
    .fetch_one(pool)
    .await?;
    Ok(row.recorded.unwrap_or(false))
}

async fn fetch_pending(
    transaction: &mut Transaction<'_, Postgres>,
    queue_id: &str,
) -> Result<PendingQueue, IdentityError> {
    let row = sqlx::query!(
        "SELECT queue_id, alias_id, candidate_player_ids
           FROM fantatt.identity_ambiguity_queue
          WHERE queue_id = $1 AND status = 'PENDING' FOR UPDATE",
        queue_id
    )
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(row) = row else {
        return Err(IdentityError::QueueNotPending);
    };
    Ok(PendingQueue {
        queue_id: row.queue_id,
        alias_id: row.alias_id,
        candidate_player_ids: row.candidate_player_ids,
    })
}

/// The three writes one admin decision makes, in the transaction the caller
/// owns and commits.
async fn settle(
    transaction: &mut Transaction<'_, Postgres>,
    queued: &PendingQueue,
    decision: &Decision<'_>,
) -> Result<(), IdentityError> {
    let chosen = ChosenPlayer {
        alias_id: &queued.alias_id,
        player_id: decision.resolved_player_id,
    };
    move_alias(transaction, chosen).await?;
    settle_player(transaction, decision.resolved_player_id).await?;
    record_decision(transaction, queued, decision).await
}

/// why: the canonical form is unique per player, so a decision can collide
/// with an alias the chosen player already carries; the redundant row is
/// dropped instead of failing the decision.
async fn move_alias(
    transaction: &mut Transaction<'_, Postgres>,
    chosen: ChosenPlayer<'_>,
) -> Result<(), IdentityError> {
    let moved = sqlx::query!(
        "UPDATE fantatt.identity_aliases SET player_id = $2, status = 'RESOLVED', resolved_at = now() WHERE alias_id = $1",
        chosen.alias_id,
        chosen.player_id
    )
    .execute(&mut **transaction)
    .await;
    match moved {
        Ok(_) => Ok(()),
        Err(error) if is_alias_conflict(&error) => drop_alias(transaction, chosen.alias_id).await,
        Err(error) => Err(IdentityError::from(error)),
    }
}

async fn drop_alias(
    transaction: &mut Transaction<'_, Postgres>,
    alias_id: &str,
) -> Result<(), IdentityError> {
    sqlx::query!(
        "DELETE FROM fantatt.identity_aliases WHERE alias_id = $1",
        alias_id
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn settle_player(
    transaction: &mut Transaction<'_, Postgres>,
    player_id: &str,
) -> Result<(), IdentityError> {
    sqlx::query!(
        "UPDATE fantatt.players SET provisional = false, updated_at = now() WHERE player_id = $1",
        player_id
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn record_decision(
    transaction: &mut Transaction<'_, Postgres>,
    queued: &PendingQueue,
    decision: &Decision<'_>,
) -> Result<(), IdentityError> {
    sqlx::query!(
        "UPDATE fantatt.identity_ambiguity_queue SET status = 'RESOLVED', resolved_player_id = $2, resolved_by = $3, resolution_note = $4, resolved_at = now() WHERE queue_id = $1",
        queued.queue_id,
        decision.resolved_player_id,
        decision.actor_id,
        decision.note
    )
    .execute(&mut **transaction)
    .await?;
    audit_decision(transaction, decision).await
}

/// The append-only log entry one decision leaves behind.
async fn audit_decision(
    transaction: &mut Transaction<'_, Postgres>,
    decision: &Decision<'_>,
) -> Result<(), IdentityError> {
    sqlx::query!(
        "INSERT INTO fantatt.audit_log (actor_id, action, entity_type, entity_id, payload) VALUES ($1, 'identity.ambiguity.resolved', 'player', $2, $3)",
        decision.actor_id,
        decision.resolved_player_id,
        serde_json::json!({ "note": decision.note })
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn is_listed(queued: &PendingQueue, player_id: &str) -> bool {
    queued
        .candidate_player_ids
        .iter()
        .any(|candidate| candidate == player_id)
}

fn ambiguous_candidates(outcome: &Resolution) -> Vec<Candidate> {
    match outcome {
        Resolution::Ambiguous { candidates } => candidates.clone(),
        _ => Vec::new(),
    }
}

fn alias_confidence(state: IdentityState) -> &'static str {
    match state {
        IdentityState::Resolved => "HIGH",
        IdentityState::Ambiguous => "AMBIGUOUS",
        IdentityState::Unresolved => "LOW",
    }
}

fn confidence_label(confidence: RatingConfidence) -> &'static str {
    match confidence {
        RatingConfidence::High => "HIGH",
        RatingConfidence::Medium => "MEDIUM",
        RatingConfidence::Low => "LOW",
    }
}

fn duplicate_rating(error: &sqlx::Error, rating: &Rating) -> Option<IdentityError> {
    let database = error.as_database_error()?;
    if database.constraint()? != "player_ratings_player_source_as_of_key" {
        return None;
    }
    Some(IdentityError::DuplicateRating {
        as_of: rating.as_of(),
        rating_source: rating.source().to_owned(),
    })
}

fn is_alias_conflict(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|database| database.constraint())
        .is_some_and(|constraint| constraint == "identity_aliases_player_normalized_key")
}
