-- Reverse of 202611160003_leagues_squads.up.sql.

DROP TRIGGER IF EXISTS slots_enforce_exclusive_player ON fantatt.slots;
DROP FUNCTION IF EXISTS fantatt.enforce_exclusive_player();

DROP TABLE IF EXISTS fantatt.submissions;
DROP TABLE IF EXISTS fantatt.slots;
DROP TABLE IF EXISTS fantatt.squads;
DROP TABLE IF EXISTS fantatt.invitations;
DROP TABLE IF EXISTS fantatt.memberships;
DROP TABLE IF EXISTS fantatt.leagues;