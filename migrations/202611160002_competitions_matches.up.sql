-- FITET competition surfaces: teams, competitions, matchdays, ties and the
-- per-player appearances the scoring engine consumes (G0.5).

CREATE TABLE fantatt.teams (
    team_id        text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('team', team_id)),
    competition_id text NOT NULL,
    name           text NOT NULL,
    season_year    integer NOT NULL,
    created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX teams_competition_season_name_key
    ON fantatt.teams (competition_id, season_year, name);

CREATE TABLE fantatt.competitions (
    competition_id text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('cmp', competition_id)),
    kind           text NOT NULL CHECK (kind IN ('CHAMPIONSHIP', 'TOURNAMENT')),
    region         text NOT NULL,
    season_year    integer NOT NULL,
    division_tier  text NOT NULL
                   CHECK (division_tier IN ('B2', 'C1', 'C2', 'D1', 'D2')),
    name           text NOT NULL,
    gender_band    text NOT NULL CHECK (gender_band IN ('MALE', 'FEMALE', 'MIXED')),
    source_uri     text,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX competitions_region_season_tier_name_key
    ON fantatt.competitions (region, season_year, division_tier, name);

ALTER TABLE fantatt.teams
    ADD CONSTRAINT teams_competition_id_fkey
    FOREIGN KEY (competition_id) REFERENCES fantatt.competitions (competition_id)
    ON DELETE CASCADE;

CREATE TABLE fantatt.matchdays (
    matchday_id   text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('md', matchday_id)),
    competition_id text NOT NULL REFERENCES fantatt.competitions (competition_id) ON DELETE CASCADE,
    number        integer NOT NULL CHECK (number >= 1),
    scheduled_date date,
    status        text NOT NULL DEFAULT 'SCHEDULED'
                  CHECK (status IN ('SCHEDULED', 'PARTIAL', 'COMPLETE', 'CANCELLED')),
    source_uri    text,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX matchdays_competition_number_key
    ON fantatt.matchdays (competition_id, number);

CREATE TABLE fantatt.ties (
    tie_id       text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('tie', tie_id)),
    matchday_id  text NOT NULL REFERENCES fantatt.matchdays (matchday_id) ON DELETE CASCADE,
    home_team_id text REFERENCES fantatt.teams (team_id),
    away_team_id text REFERENCES fantatt.teams (team_id),
    home_score   integer CHECK (home_score >= 0),
    away_score   integer CHECK (away_score >= 0),
    winner_team_id text REFERENCES fantatt.teams (team_id),
    format       text NOT NULL CHECK (format IN
                   ('SWAYTHLING', 'MINI_SWAYTHLING', 'NEW_SWAYTHLING', 'CUSTOM')),
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE fantatt.appearances (
    appearance_id   text PRIMARY KEY CHECK (fantatt.assert_prefixed_id('app', appearance_id)),
    tie_id          text NOT NULL REFERENCES fantatt.ties (tie_id) ON DELETE CASCADE,
    matchday_id     text NOT NULL REFERENCES fantatt.matchdays (matchday_id) ON DELETE CASCADE,
    player_id       text NOT NULL REFERENCES fantatt.players (player_id) ON DELETE CASCADE,
    kind            text NOT NULL CHECK (kind IN ('SINGLES', 'DOUBLES')),
    partner_id      text REFERENCES fantatt.players (player_id),
    opponent_id     text REFERENCES fantatt.players (player_id),
    opponent_rating integer,
    played          boolean NOT NULL DEFAULT true,
    outcome         text CHECK (outcome IN ('WIN', 'LOSS')),
    sets_won        integer CHECK (sets_won >= 0),
    sets_lost       integer CHECK (sets_lost >= 0),
    scoreline       jsonb,
    is_decider      boolean NOT NULL DEFAULT false,
    is_comeback     boolean NOT NULL DEFAULT false,
    is_away         boolean NOT NULL DEFAULT false,
    is_team_win     boolean NOT NULL DEFAULT false,
    is_doublet      boolean NOT NULL DEFAULT false,
    created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX appearances_player_matchday_key
    ON fantatt.appearances (player_id, matchday_id);