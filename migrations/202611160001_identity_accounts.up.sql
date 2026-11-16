-- Identity, accounts and the session store (G0.5).
--
-- Conventions used by every migration:
--   * identifiers follow the ULID wire convention of docs/07-api-spec.md and
--     are validated at the database boundary by fantatt.assert_prefixed_id /
--     fantatt.assert_ulid, using the prefixes declared in
--     crates/fantatt-core/src/ids.rs;
--   * timestamps are timestamptz and always UTC on the wire (RFC 3339);
--   * money is an ISO 4217 currency plus integer minor units, matching
--     fantatt-core::money::Money.

CREATE SCHEMA IF NOT EXISTS fantatt;

-- tower-sessions-sqlx-store creates schema tower_sessions and table session
-- with exactly this DDL through its own migrator; the mirror here keeps the
-- whole relational model inside one reversible migration set.
CREATE SCHEMA IF NOT EXISTS tower_sessions;

CREATE TABLE IF NOT EXISTS tower_sessions.session (
    id          text PRIMARY KEY NOT NULL,
    data        bytea NOT NULL,
    expiry_date timestamptz NOT NULL
);

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA fantatt;

CREATE FUNCTION fantatt.assert_ulid(candidate text)
RETURNS boolean LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT candidate ~ '^[0-9A-HJKMNP-TV-Z]{26}$'
$$;

CREATE FUNCTION fantatt.assert_prefixed_id(expected_prefix text, candidate text)
RETURNS boolean LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT candidate ~ ('^' || expected_prefix || '_[0-9A-HJKMNP-TV-Z]{26}$')
$$;

CREATE TABLE fantatt.users (
    user_id           text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('usr', user_id)),
    email             text NOT NULL,
    display_name      text NOT NULL,
    avatar_url        text,
    email_verified_at timestamptz,
    locale            text NOT NULL DEFAULT 'it',
    state             text NOT NULL DEFAULT 'ACTIVE'
                     CHECK (state IN ('ACTIVE', 'SUSPENDED', 'DELETED')),
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    deleted_at        timestamptz
);

CREATE UNIQUE INDEX users_email_key ON fantatt.users (lower(email));

CREATE TABLE fantatt.credentials (
    credential_id text PRIMARY KEY CHECK (fantatt.assert_ulid(credential_id)),
    user_id       text NOT NULL REFERENCES fantatt.users (user_id) ON DELETE CASCADE,
    kind          text NOT NULL CHECK (kind IN ('GOOGLE', 'PASSKEY')),
    identifier    text NOT NULL,
    public_key    jsonb,
    sign_count    bigint NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    last_used_at  timestamptz
);

CREATE UNIQUE INDEX credentials_kind_identifier_key
    ON fantatt.credentials (kind, identifier);

CREATE TABLE fantatt.preferences (
    user_id     text PRIMARY KEY REFERENCES fantatt.users (user_id) ON DELETE CASCADE,
    channels    jsonb NOT NULL DEFAULT '{}'::jsonb,
    categories  jsonb NOT NULL DEFAULT '{}'::jsonb,
    locale      text NOT NULL DEFAULT 'it',
    timezone    text NOT NULL DEFAULT 'Europe/Rome',
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE fantatt.players (
    player_id      text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('plr', player_id)),
    fitet_person_id text,
    full_name      text NOT NULL,
    gender         text NOT NULL CHECK (gender IN ('MALE', 'FEMALE')),
    provisional    boolean NOT NULL DEFAULT false,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX players_fitet_person_id_key
    ON fantatt.players (fitet_person_id) WHERE fitet_person_id IS NOT NULL;

-- Bitemporal ratings: asserted_at is when the federation published the value,
-- valid_from is the date the value applies from (docs/02-domain-model.md §4).
CREATE TABLE fantatt.player_ratings (
    rating_id    text PRIMARY KEY CHECK (fantatt.assert_ulid(rating_id)),
    player_id    text NOT NULL REFERENCES fantatt.players (player_id) ON DELETE CASCADE,
    value        integer NOT NULL,
    as_of        date NOT NULL,
    source       text NOT NULL,
    confidence   text NOT NULL CHECK (confidence IN ('HIGH', 'MEDIUM', 'LOW')),
    asserted_at  timestamptz NOT NULL DEFAULT now(),
    valid_from   date NOT NULL
);

-- INV-1: a player has exactly one rating value per asOf date per classifica.
CREATE UNIQUE INDEX player_ratings_player_source_as_of_key
    ON fantatt.player_ratings (player_id, source, as_of);

CREATE TABLE fantatt.identity_aliases (
    alias_id          text PRIMARY KEY CHECK (fantatt.assert_ulid(alias_id)),
    player_id         text NOT NULL REFERENCES fantatt.players (player_id) ON DELETE CASCADE,
    raw_alias         text NOT NULL,
    normalized_alias  text NOT NULL,
    evidence          jsonb NOT NULL DEFAULT '{}'::jsonb,
    confidence        text NOT NULL CHECK (confidence IN ('HIGH', 'MEDIUM', 'LOW', 'AMBIGUOUS')),
    status            text NOT NULL CHECK (status IN ('RESOLVED', 'AMBIGUOUS', 'UNRESOLVED')),
    created_at        timestamptz NOT NULL DEFAULT now(),
    resolved_at       timestamptz
);

CREATE UNIQUE INDEX identity_aliases_player_normalized_key
    ON fantatt.identity_aliases (player_id, normalized_alias);

CREATE TABLE fantatt.identity_ambiguity_queue (
    queue_id           text PRIMARY KEY CHECK (fantatt.assert_ulid(queue_id)),
    alias_id           text NOT NULL REFERENCES fantatt.identity_aliases (alias_id) ON DELETE CASCADE,
    candidate_player_ids text[] NOT NULL,
    status             text NOT NULL DEFAULT 'PENDING'
                       CHECK (status IN ('PENDING', 'RESOLVED', 'DISMISSED')),
    resolved_player_id text REFERENCES fantatt.players (player_id),
    resolved_by        text REFERENCES fantatt.users (user_id),
    resolution_note    text,
    created_at         timestamptz NOT NULL DEFAULT now(),
    resolved_at        timestamptz
);