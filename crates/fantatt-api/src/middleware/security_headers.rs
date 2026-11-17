//! Security headers on every response (docs/10-security.md §3).

use axum::extract::Request;
use axum::http::{HeaderValue, header};
use axum::middleware::Next;
use axum::response::Response;
use rand::RngCore;

/// The documented policy, byte-exact except for the nonce placeholders that
/// each response fills from its own freshly generated nonce.
const CSP_TEMPLATE: &str = "default-src 'none'; script-src 'nonce-{nonce}'; style-src 'nonce-{nonce}'; img-src 'self' data: blob:; connect-src 'self'; font-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'; object-src 'none'; require-trusted-types-for 'script'";
const STRICT_TRANSPORT_SECURITY: &str = "max-age=63072000; includeSubDomains";

/// The per-response CSP nonce, available to handlers through request
/// extensions so inline scripts and styles can carry it.
#[derive(Clone, Debug)]
pub struct ResponseNonce(pub String);

pub async fn attach_security_headers(mut request: Request, next: Next) -> Response {
    let nonce = generate_nonce();
    request
        .extensions_mut()
        .insert(ResponseNonce(nonce.clone()));
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        header_value(&csp_directives(&nonce)),
    );
    response.headers_mut().insert(
        header::STRICT_TRANSPORT_SECURITY,
        header_value(STRICT_TRANSPORT_SECURITY),
    );
    response
        .headers_mut()
        .insert(header::X_CONTENT_TYPE_OPTIONS, header_value("nosniff"));
    response
}

fn csp_directives(nonce: &str) -> String {
    CSP_TEMPLATE.replace("{nonce}", nonce)
}

fn generate_nonce() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    hex(&bytes)
}

/// why: no hex crate in the workspace; these short digests (nonces, request
/// ids, HMAC tags) format at the call site.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn header_value(text: &str) -> HeaderValue {
    // why: every input is a constant or freshly generated hexadecimal, both of
    // which are valid header values; a failure here is a programming error.
    HeaderValue::from_str(text).expect("generated header value is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_nonce_placeholders_carry_the_same_generated_value() {
        let directives = csp_directives("abc123");
        assert_eq!(directives.matches("nonce-abc123").count(), 2);
        assert!(!directives.contains("{nonce}"));
        assert!(directives.starts_with("default-src 'none';"));
        assert!(directives.ends_with("require-trusted-types-for 'script'"));
    }

    #[test]
    fn a_nonce_is_thirty_two_hex_characters() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 32);
        assert!(nonce.chars().all(|character| character.is_ascii_hexdigit()));
    }
}
