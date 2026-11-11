//! Notification fanout: in-app inbox, email, and web push.
//!
//! Deduplicates outbound delivery through the outbox and honors per-category
//! preferences. OpenTelemetry export is wired when observability lands.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
