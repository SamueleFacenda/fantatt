//! Scheduler and worker processes.
//!
//! Runs ingest, recompute, and notification jobs from a Postgres-backed queue
//! driven by LISTEN/NOTIFY events.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
