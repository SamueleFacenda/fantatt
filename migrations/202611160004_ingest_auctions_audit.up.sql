-- Ingest ledger and jobs, auction tables (Phase 5 placeholders), tournament
-- predictions, notifications and the append-only audit log (G0.5).

CREATE TABLE fantatt.ingest_ledger (
    ledger_id      text PRIMARY KEY CHECK (fantatt.assert_ulid(ledger_id)),
    source         text NOT NULL,
    source_id      text NOT NULL,
    content_hash   text NOT NULL,
    etag           text,
    last_modified  timestamptz,
    state          text NOT NULL
                   CHECK (state IN ('FETCHED', 'SKIPPED', 'INCOMPLETE')),
    payload_location text,
    fetched_at     timestamptz NOT NULL DEFAULT now(),
    created_at     timestamptz NOT NULL DEFAULT now()
);

-- The diff stage keys on (source, source_id, content_hash): re-fetching an
-- unchanged page inserts nothing (docs/01-architecture.md §4.1).
CREATE UNIQUE INDEX ingest_ledger_source_key
    ON fantatt.ingest_ledger (source, source_id, content_hash);

CREATE TABLE fantatt.ingest_jobs (
    job_id      text PRIMARY KEY CHECK (fantatt.assert_ulid(job_id)),
    job_type    text NOT NULL,
    source      text NOT NULL,
    source_id   text,
    state       text NOT NULL DEFAULT 'PENDING'
                CHECK (state IN ('PENDING', 'RUNNING', 'SUCCEEDED', 'FAILED', 'SKIPPED')),
    scheduled_at timestamptz NOT NULL DEFAULT now(),
    next_run_at timestamptz,
    started_at  timestamptz,
    finished_at timestamptz,
    attempts    integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error  text,
    payload     jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

-- Dedupe: at most one active job per (job_type, source, source_id).
CREATE UNIQUE INDEX ingest_jobs_active_key
    ON fantatt.ingest_jobs (job_type, source, source_id)
    WHERE state IN ('PENDING', 'RUNNING');

CREATE TABLE fantatt.quarantine (
    quarantine_id text PRIMARY KEY CHECK (fantatt.assert_ulid(quarantine_id)),
    source        text NOT NULL,
    source_id     text NOT NULL,
    reason        text NOT NULL,
    report        jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at    timestamptz NOT NULL DEFAULT now(),
    resolved_at   timestamptz,
    resolved_by   text REFERENCES fantatt.users (user_id)
);

CREATE TABLE fantatt.lots (
    lot_id             text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('lot', lot_id)),
    league_id          text NOT NULL REFERENCES fantatt.leagues (league_id) ON DELETE CASCADE,
    player_id          text NOT NULL REFERENCES fantatt.players (player_id),
    status             text NOT NULL DEFAULT 'OPEN'
                       CHECK (status IN ('OPEN', 'SOLD', 'UNSOLD')),
    deadline           timestamptz,
    soft_close_window  interval,
    current_best_bid_id text,
    created_at         timestamptz NOT NULL DEFAULT now(),
    closed_at          timestamptz
);

CREATE UNIQUE INDEX lots_league_player_key
    ON fantatt.lots (league_id, player_id) WHERE status = 'OPEN';

CREATE TABLE fantatt.offers (
    offer_id        text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('off', offer_id)),
    lot_id          text NOT NULL REFERENCES fantatt.lots (lot_id) ON DELETE CASCADE,
    manager_id      text NOT NULL REFERENCES fantatt.users (user_id),
    parent_offer_id text REFERENCES fantatt.offers (offer_id),
    amount_currency char(3) NOT NULL DEFAULT 'EUR',
    amount_minor    bigint NOT NULL CHECK (amount_minor >= 0),
    state           text NOT NULL DEFAULT 'OPEN'
                    CHECK (state IN ('OPEN', 'ACCEPTED', 'REJECTED', 'COUNTERED', 'WITHDRAWN')),
    idempotency_key text NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    responded_at    timestamptz
);

CREATE UNIQUE INDEX offers_idempotency_key ON fantatt.offers (idempotency_key);

CREATE TABLE fantatt.bids (
    bid_id          text PRIMARY KEY CHECK (fantatt.assert_ulid(bid_id)),
    lot_id          text NOT NULL REFERENCES fantatt.lots (lot_id) ON DELETE CASCADE,
    league_id       text NOT NULL REFERENCES fantatt.leagues (league_id) ON DELETE CASCADE,
    manager_id      text NOT NULL REFERENCES fantatt.users (user_id),
    amount_currency char(3) NOT NULL DEFAULT 'EUR',
    amount_minor    bigint NOT NULL CHECK (amount_minor >= 0),
    state           text NOT NULL DEFAULT 'PLACED'
                    CHECK (state IN ('PLACED', 'WON', 'OUTBID', 'REFUNDED')),
    idempotency_key text NOT NULL,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX bids_idempotency_key ON fantatt.bids (idempotency_key);

CREATE INDEX bids_league_manager_key
    ON fantatt.bids (league_id, manager_id) WHERE state <> 'REFUNDED';

-- INV-12: total auction spend per manager never exceeds the league budget.
CREATE FUNCTION fantatt.enforce_budget() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    committed bigint;
    ceiling   bigint;
BEGIN
    IF NEW.state = 'REFUNDED' THEN
        RETURN NEW;
    END IF;

    SELECT coalesce(sum(amount_minor), 0) INTO committed
      FROM fantatt.bids
     WHERE league_id = NEW.league_id
       AND manager_id = NEW.manager_id
       AND state <> 'REFUNDED';

    SELECT budget_minor INTO ceiling
      FROM fantatt.leagues
     WHERE league_id = NEW.league_id;

    IF committed + NEW.amount_minor > ceiling THEN
        RAISE EXCEPTION 'INV-12: manager % would exceed the league budget (% > %)',
            NEW.manager_id, committed + NEW.amount_minor, ceiling
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER bids_enforce_budget
    BEFORE INSERT ON fantatt.bids
    FOR EACH ROW EXECUTE FUNCTION fantatt.enforce_budget();

CREATE TABLE fantatt.tournaments (
    tournament_id  text PRIMARY KEY CHECK (fantatt.assert_ulid(tournament_id)),
    competition_id text REFERENCES fantatt.competitions (competition_id) ON DELETE SET NULL,
    name           text NOT NULL,
    kind           text NOT NULL CHECK (kind IN ('CHAMPIONSHIP', 'TOURNAMENT')),
    region         text NOT NULL,
    season_year    integer NOT NULL,
    bracket        jsonb,
    complete       boolean NOT NULL DEFAULT false,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE fantatt.predictions (
    prediction_id  text PRIMARY KEY CHECK (fantatt.assert_ulid(prediction_id)),
    tournament_id  text NOT NULL REFERENCES fantatt.tournaments (tournament_id) ON DELETE CASCADE,
    user_id        text NOT NULL REFERENCES fantatt.users (user_id) ON DELETE CASCADE,
    picks          jsonb NOT NULL DEFAULT '{}'::jsonb,
    deadline       timestamptz NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX predictions_tournament_user_key
    ON fantatt.predictions (tournament_id, user_id);

CREATE TABLE fantatt.notifications (
    notification_id text PRIMARY KEY CHECK (fantatt.assert_ulid(notification_id)),
    user_id         text NOT NULL REFERENCES fantatt.users (user_id) ON DELETE CASCADE,
    category        text NOT NULL,
    channel         text NOT NULL CHECK (channel IN ('IN_APP', 'EMAIL', 'PUSH')),
    title           text NOT NULL,
    body            text NOT NULL,
    payload         jsonb NOT NULL DEFAULT '{}'::jsonb,
    read_at         timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX notifications_user_created_key
    ON fantatt.notifications (user_id, created_at DESC);

CREATE TABLE fantatt.audit_log (
    entry_id    bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    actor_id    text,
    league_id   text,
    action      text NOT NULL,
    entity_type text,
    entity_id   text,
    payload     jsonb NOT NULL DEFAULT '{}'::jsonb,
    prev_hash   text NOT NULL,
    entry_hash  text NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX audit_log_actor_key ON fantatt.audit_log (actor_id, created_at DESC);
CREATE INDEX audit_log_league_key ON fantatt.audit_log (league_id, created_at DESC);
CREATE INDEX audit_log_action_key ON fantatt.audit_log (action, created_at DESC);

-- Append-only: no UPDATE, DELETE or TRUNCATE, ever (docs/10-security.md §11).
CREATE FUNCTION fantatt.reject_audit_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only: % is not permitted', TG_OP
        USING HINT = 'Append a correcting entry instead of mutating history.';
END;
$$;

-- Hash chain: each entry commits to the previous entry hash.
CREATE FUNCTION fantatt.chain_audit_entry() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    previous text;
BEGIN
    SELECT entry_hash INTO previous
      FROM fantatt.audit_log
     ORDER BY entry_id DESC
     LIMIT 1;

    IF previous IS NULL THEN
        previous := 'GENESIS';
    END IF;

    NEW.prev_hash := previous;
    NEW.entry_hash := encode(
        fantatt.digest(
            convert_to(
                previous || '|' ||
                coalesce(NEW.actor_id, '') || '|' ||
                coalesce(NEW.league_id, '') || '|' ||
                NEW.action || '|' ||
                coalesce(NEW.entity_type, '') || '|' ||
                coalesce(NEW.entity_id, '') || '|' ||
                NEW.payload::text,
                'UTF8'
            ),
            'sha256'::text
        ),
        'hex'
    );

    RETURN NEW;
END;
$$;

CREATE TRIGGER audit_log_chain_entry
    BEFORE INSERT ON fantatt.audit_log
    FOR EACH ROW EXECUTE FUNCTION fantatt.chain_audit_entry();

CREATE TRIGGER audit_log_append_only
    BEFORE UPDATE OR DELETE OR TRUNCATE ON fantatt.audit_log
    FOR EACH STATEMENT EXECUTE FUNCTION fantatt.reject_audit_mutation();

REVOKE UPDATE, DELETE, TRUNCATE ON fantatt.audit_log FROM PUBLIC;