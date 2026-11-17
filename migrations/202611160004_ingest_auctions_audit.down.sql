-- Reverse of 202611160004_ingest_auctions_audit.up.sql.

DROP TRIGGER IF EXISTS audit_log_append_only ON fantatt.audit_log;
DROP TRIGGER IF EXISTS audit_log_chain_entry ON fantatt.audit_log;
DROP FUNCTION IF EXISTS fantatt.reject_audit_mutation();
DROP FUNCTION IF EXISTS fantatt.chain_audit_entry();
DROP TABLE IF EXISTS fantatt.audit_log;

DROP TABLE IF EXISTS fantatt.notifications;
DROP TABLE IF EXISTS fantatt.predictions;
DROP TABLE IF EXISTS fantatt.tournaments;

DROP TRIGGER IF EXISTS bids_enforce_budget ON fantatt.bids;
DROP FUNCTION IF EXISTS fantatt.enforce_budget();
DROP TABLE IF EXISTS fantatt.bids;
DROP TABLE IF EXISTS fantatt.offers;
DROP TABLE IF EXISTS fantatt.lots;

DROP TABLE IF EXISTS fantatt.quarantine;
DROP TABLE IF EXISTS fantatt.ingest_jobs;
DROP TABLE IF EXISTS fantatt.ingest_ledger;