-- Reverse of 202611160002_competitions_matches.up.sql.

DROP TABLE IF EXISTS fantatt.appearances;
DROP TABLE IF EXISTS fantatt.ties;
DROP TABLE IF EXISTS fantatt.matchdays;
ALTER TABLE fantatt.teams DROP CONSTRAINT IF EXISTS teams_competition_id_fkey;
DROP TABLE IF EXISTS fantatt.teams;
DROP TABLE IF EXISTS fantatt.competitions;