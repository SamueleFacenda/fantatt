//! Knowledge-graph runtime and reasoning engine integration.
//!
//! Wraps Oxigraph as a derived projection of the Postgres ledger, validates
//! ingest documents with rudof SHACL shapes, and evaluates N3 rule bundles with
//! eyeron to emit offline-verifiable proof certificates.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
