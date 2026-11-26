//! The identity reconciliation admin surface (docs/07-api-spec.md §12).

use axum::Json;
use axum::extract::{FromRequest, FromRequestParts, Path, Request, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use utoipa::ToSchema;

use fantatt_jobs::identity::{Decision, IdentityError, IdentityStore, QueuedAmbiguity};

use crate::authz::PRINCIPAL_ID_KEY;
use crate::error::{Problem, ProblemType};
use crate::state::AppState;

const QUEUE_LIMIT: i64 = 50;
const DECISION_LIMIT: usize = 64 * 1024;

/// The ambiguities awaiting a decision.
#[derive(Debug, Serialize, ToSchema)]
pub struct AmbiguityQueue {
    pub items: Vec<AmbiguityItem>,
}

/// One queued ambiguity with the candidate players it waits on.
#[derive(Debug, Serialize, ToSchema)]
pub struct AmbiguityItem {
    pub queue_id: String,
    pub alias_id: String,
    pub raw_alias: String,
    pub normalized_alias: String,
    pub status: String,
    pub candidates: Vec<CandidatePlayer>,
}

/// One candidate of a queued ambiguity.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CandidatePlayer {
    pub player_id: String,
    pub full_name: String,
}

/// The admin decision on one queued ambiguity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResolutionRequest {
    pub resolved_player_id: String,
    pub note: Option<String>,
}

/// The merge the admin decision settled.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResolutionConfirmation {
    pub queue_id: String,
    pub resolved_player_id: String,
}

/// The ambiguities awaiting a decision, newest first, each with the candidate
/// players it waits on.
#[utoipa::path(get, path = "/api/v1/admin/players/reconciliation", responses(
    (status = 200, description = "The pending ambiguities, newest first", body = AmbiguityQueue),
    (status = 401, description = "The session expired", body = Problem),
    (status = 403, description = "The session is no platform superadmin", body = Problem),
    (status = 500, description = "The queue is unreadable", body = Problem),
))]
pub async fn list_ambiguities(
    State(state): State<AppState>,
) -> Result<Json<AmbiguityQueue>, Problem> {
    let store = IdentityStore::new(state.pool().clone());
    let queued = store
        .pending_ambiguities(QUEUE_LIMIT)
        .await
        .map_err(rejected)?;
    let items = queued.iter().map(AmbiguityItem::from_queued).collect();
    Ok(Json(AmbiguityQueue { items }))
}

/// Settles one queued ambiguity the way the admin decided: the alias moves to
/// the chosen player, who stops being provisional, and the append-only log
/// records who decided.
#[utoipa::path(post, path = "/api/v1/admin/players/reconciliation/{queue_id}/resolve",
    params(("queue_id" = String, Path, description = "The queued ambiguity to settle")),
    request_body = ResolutionRequest,
    responses(
    (status = 200, description = "The merge the decision settled", body = ResolutionConfirmation),
    (status = 401, description = "The session expired", body = Problem),
    (status = 403, description = "The session is no platform superadmin", body = Problem),
    (status = 422, description = "The ambiguity awaits no decision, or the chosen player is no candidate", body = Problem),
))]
pub async fn resolve_ambiguity(
    actor: AdminActor,
    State(state): State<AppState>,
    decision: PendingDecision,
) -> Response {
    let choice = Decision {
        resolved_player_id: &decision.choice.resolved_player_id,
        actor_id: &actor.0,
        note: decision.choice.note.as_deref(),
    };
    let store = IdentityStore::new(state.pool().clone());
    match store.resolve_ambiguity(&decision.queue_id, choice).await {
        Ok(()) => Json(ResolutionConfirmation {
            queue_id: decision.queue_id,
            resolved_player_id: decision.choice.resolved_player_id,
        })
        .into_response(),
        Err(error) => failure(error).into_response(),
    }
}

/// The admin one decision is attributed to, read from the session the authorize
/// middleware already proved.
pub struct AdminActor(String);

impl<S: Send + Sync> FromRequestParts<S> for AdminActor {
    type Rejection = Problem;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| unauthenticated())?;
        acting_admin(&session)
            .await
            .map(AdminActor)
            .ok_or_else(unauthenticated)
    }
}

/// The queued ambiguity the path names and the decision the body carries.
pub struct PendingDecision {
    queue_id: String,
    choice: ResolutionRequest,
}

impl<S: Send + Sync> FromRequest<S> for PendingDecision {
    type Rejection = Problem;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = request.into_parts();
        let Path(queue_id) = Path::from_request_parts(&mut parts, state)
            .await
            .map_err(|_| unprocessable("E_RequestInvalid"))?;
        let bytes = axum::body::to_bytes(body, DECISION_LIMIT)
            .await
            .map_err(|_| unprocessable("E_RequestInvalid"))?;
        let choice: ResolutionRequest =
            serde_json::from_slice(&bytes).map_err(|_| unprocessable("E_RequestInvalid"))?;
        Ok(Self { queue_id, choice })
    }
}

impl AmbiguityItem {
    fn from_queued(queued: &QueuedAmbiguity) -> Self {
        Self {
            queue_id: queued.queue_id.clone(),
            alias_id: queued.alias_id.clone(),
            raw_alias: queued.raw_alias.clone(),
            normalized_alias: queued.normalized_alias.clone(),
            status: queued.status.clone(),
            candidates: candidates(&queued.candidates),
        }
    }
}

fn candidates(evidence: &serde_json::Value) -> Vec<CandidatePlayer> {
    serde_json::from_value(evidence.clone()).unwrap_or_default()
}

/// why: the authorize middleware already proved the principal is a superadmin,
/// but the decision names its actor, so the handler reads the id back from the
/// session.
async fn acting_admin(session: &Session) -> Option<String> {
    session.get(PRINCIPAL_ID_KEY).await.unwrap_or(None)
}

fn failure(error: IdentityError) -> Problem {
    match error {
        IdentityError::CandidateNotListed => unprocessable("E_CandidateNotListed"),
        IdentityError::QueueNotPending => unprocessable("E_QueueNotPending"),
        IdentityError::Rating(_) => unprocessable("E_RatingInvalid"),
        other => rejected(other),
    }
}

fn rejected(error: IdentityError) -> Problem {
    tracing::error!(error = ?error, "the identity store refused the query");
    Problem::from_type(ProblemType::Internal)
}

/// why: a decision the store refuses is the caller's own doing, so it answers
/// unprocessable rather than a server failure.
fn unprocessable(code: &str) -> Problem {
    Problem::from_type(ProblemType::InvalidRequest)
        .with_code(code)
        .with_status(StatusCode::UNPROCESSABLE_ENTITY)
}

fn unauthenticated() -> Problem {
    Problem::from_type(ProblemType::SessionExpired)
}
