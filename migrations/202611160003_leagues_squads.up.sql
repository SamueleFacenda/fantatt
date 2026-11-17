-- Leagues, memberships, invitations, squads and slots (G0.5).
--
-- Roles follow the capability model of docs/10-security.md §authz; the enum in
-- fantatt-core grows the same set in G0.7.

CREATE TABLE fantatt.leagues (
    league_id              text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('lg', league_id)),
    name                   text NOT NULL,
    slug                   text NOT NULL,
    owner_id               text NOT NULL REFERENCES fantatt.users (user_id),
    season_year            integer NOT NULL,
    ownership_mode         text NOT NULL DEFAULT 'SHARED'
                           CHECK (ownership_mode IN ('EXCLUSIVE', 'SHARED')),
    state                  text NOT NULL DEFAULT 'DRAFT'
                           CHECK (state IN ('DRAFT', 'OPEN', 'AUCTION', 'ROSTERING', 'ACTIVE', 'ARCHIVED')),
    budget_currency        char(3) NOT NULL DEFAULT 'EUR',
    budget_minor           bigint NOT NULL CHECK (budget_minor >= 0),
    squad_size             integer NOT NULL DEFAULT 15 CHECK (squad_size > 0),
    starters_per_tier      integer NOT NULL DEFAULT 2 CHECK (starters_per_tier >= 0),
    bench_per_tier         integer NOT NULL DEFAULT 1 CHECK (bench_per_tier >= 0),
    resell_day_of_month    integer CHECK (resell_day_of_month BETWEEN 1 AND 31),
    ruleset_version        text,
    settings               jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX leagues_slug_key ON fantatt.leagues (slug);

CREATE TABLE fantatt.memberships (
    membership_id   text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('mbr', membership_id)),
    league_id       text NOT NULL REFERENCES fantatt.leagues (league_id) ON DELETE CASCADE,
    user_id         text NOT NULL REFERENCES fantatt.users (user_id) ON DELETE CASCADE,
    role            text NOT NULL
                    CHECK (role IN ('SUPERADMIN', 'LEAGUE_ADMIN', 'COLLABORATOR', 'MANAGER', 'MEMBER')),
    state           text NOT NULL DEFAULT 'INVITED'
                    CHECK (state IN ('INVITED', 'ACTIVE', 'REMOVED')),
    remaining_currency char(3) NOT NULL DEFAULT 'EUR',
    remaining_minor bigint NOT NULL DEFAULT 0 CHECK (remaining_minor >= 0),
    joined_at       date NOT NULL DEFAULT (now() AT TIME ZONE 'UTC'),
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX memberships_league_user_key
    ON fantatt.memberships (league_id, user_id);

CREATE TABLE fantatt.invitations (
    invitation_id  text PRIMARY KEY CHECK (fantatt.assert_ulid(invitation_id)),
    league_id      text NOT NULL REFERENCES fantatt.leagues (league_id) ON DELETE CASCADE,
    email          text NOT NULL,
    role           text NOT NULL
                   CHECK (role IN ('LEAGUE_ADMIN', 'COLLABORATOR', 'MANAGER', 'MEMBER')),
    token_hash     text NOT NULL,
    invited_by     text NOT NULL REFERENCES fantatt.users (user_id),
    state          text NOT NULL DEFAULT 'PENDING'
                   CHECK (state IN ('PENDING', 'ACCEPTED', 'REVOKED', 'EXPIRED')),
    expires_at     timestamptz NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    responded_at   timestamptz
);

CREATE TABLE fantatt.squads (
    squad_id         text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('sq', squad_id)),
    league_id        text NOT NULL REFERENCES fantatt.leagues (league_id) ON DELETE CASCADE,
    manager_id       text NOT NULL REFERENCES fantatt.users (user_id) ON DELETE CASCADE,
    month_year       date NOT NULL,
    status           text NOT NULL DEFAULT 'DRAFT'
                     CHECK (status IN ('DRAFT', 'SUBMITTED', 'LOCKED', 'SCORED')),
    revision         bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    ruleset_version  text,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now()
);

-- One roster per manager per league per month, in both ownership modes.
CREATE UNIQUE INDEX squads_league_manager_month_key
    ON fantatt.squads (league_id, manager_id, month_year);

CREATE TABLE fantatt.slots (
    slot_id    text PRIMARY KEY CHECK (fantatt.assert_ulid(slot_id)),
    squad_id   text NOT NULL REFERENCES fantatt.squads (squad_id) ON DELETE CASCADE,
    tier       text NOT NULL CHECK (tier IN ('B2', 'C1', 'C2', 'D1', 'D2')),
    position   integer NOT NULL CHECK (position >= 0),
    is_bench   boolean NOT NULL DEFAULT false,
    player_id  text REFERENCES fantatt.players (player_id)
);

-- INV-2: a squad contains exactly 15 slots: per tier 2 starters + 1 bench.
CREATE UNIQUE INDEX slots_squad_tier_position_key
    ON fantatt.slots (squad_id, tier, position);

-- INV-6: no duplicate player within the same squad.
CREATE UNIQUE INDEX slots_squad_player_unique
    ON fantatt.slots (squad_id, player_id) WHERE player_id IS NOT NULL;

CREATE TABLE fantatt.submissions (
    submission_id    text PRIMARY KEY CHECK (fantatt.assert_ulid(submission_id)),
    squad_id         text NOT NULL REFERENCES fantatt.squads (squad_id) ON DELETE CASCADE,
    revision         bigint NOT NULL CHECK (revision >= 0),
    verdict          text NOT NULL CHECK (verdict IN ('ACCEPTED', 'REJECTED')),
    rule_trace       jsonb,
    proof_reference  text,
    created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX submissions_squad_revision_key
    ON fantatt.submissions (squad_id, revision);

-- INV-4: in exclusive leagues a player appears in at most one squad per
-- league per month. Checked at write time because the constraint depends on
-- the owning league's ownership_mode.
CREATE FUNCTION fantatt.enforce_exclusive_player() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    league_mode text;
    owning_month date;
    clash bigint;
BEGIN
    IF NEW.player_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT l.ownership_mode, s.month_year INTO league_mode, owning_month
      FROM fantatt.squads s
      JOIN fantatt.leagues l ON l.league_id = s.league_id
     WHERE s.squad_id = NEW.squad_id;

    IF league_mode IS NULL THEN
        RAISE EXCEPTION 'slot references unknown squad %', NEW.squad_id
            USING ERRCODE = 'foreign_key_violation';
    END IF;

    IF league_mode <> 'EXCLUSIVE' THEN
        RETURN NEW;
    END IF;

    SELECT count(*) INTO clash
      FROM fantatt.slots other
      JOIN fantatt.squads os ON os.squad_id = other.squad_id
     WHERE other.player_id = NEW.player_id
       AND other.squad_id <> NEW.squad_id
       AND os.league_id = (SELECT league_id FROM fantatt.squads WHERE squad_id = NEW.squad_id)
       AND os.month_year = owning_month;

    IF clash > 0 THEN
        RAISE EXCEPTION 'INV-4: player % already appears in another squad of this exclusive league for month %',
            NEW.player_id, owning_month
            USING ERRCODE = 'check_violation';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER slots_enforce_exclusive_player
    BEFORE INSERT OR UPDATE OF squad_id, player_id ON fantatt.slots
    FOR EACH ROW EXECUTE FUNCTION fantatt.enforce_exclusive_player();