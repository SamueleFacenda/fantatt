-- Reverse of 202611160001_identity_accounts.up.sql.

DROP TABLE IF EXISTS fantatt.identity_ambiguity_queue;
DROP TABLE IF EXISTS fantatt.identity_aliases;
DROP TABLE IF EXISTS fantatt.player_ratings;
DROP TABLE IF EXISTS fantatt.players;
DROP TABLE IF EXISTS fantatt.preferences;
DROP TABLE IF EXISTS fantatt.credentials;
DROP TABLE IF EXISTS fantatt.users;

DROP FUNCTION IF EXISTS fantatt.assert_prefixed_id(text, text);
DROP FUNCTION IF EXISTS fantatt.assert_ulid(text);

DROP EXTENSION IF EXISTS pgcrypto;

DROP TABLE IF EXISTS tower_sessions.session;
DROP SCHEMA IF EXISTS tower_sessions;
DROP SCHEMA IF EXISTS fantatt CASCADE;