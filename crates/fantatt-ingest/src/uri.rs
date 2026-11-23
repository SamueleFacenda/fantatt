//! URL resolution and normalization for portal discovery.
//!
//! The portal grammar is closed and recorded by G1.1: absolute hrefs,
//! root-relative paths, `..` segments, and query-only hrefs. The resolver
//! implements the RFC 3986 subset those shapes need. The normalized form is
//! the ledger content key, so two spellings of one target share a key:
//! lower-case scheme and host, no fragment, no default port, query pairs
//! sorted, and dot segments removed.

use std::fmt;

/// A normalized portal URL: the ledger content key of one fetch target.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uri {
    scheme: String,
    host: String,
    path: String,
    query: Vec<(String, String)>,
}

/// Why a URL could not be parsed or resolved.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UriError {
    #[error("{input:?} is not an absolute http(s) URL")]
    NotAbsolute { input: String },

    #[error("{input:?} carries no host")]
    MissingHost { input: String },
}

impl Uri {
    /// Parse an absolute http(s) URL into its normalized form.
    pub fn parse(input: &str) -> Result<Self, UriError> {
        let input = input.split('#').next().unwrap_or(input);
        let (scheme, rest) = split_scheme(input).ok_or_else(|| UriError::NotAbsolute {
            input: input.to_owned(),
        })?;
        let scheme = http_scheme(input, scheme)?;
        let (authority, tail) = split_authority(rest);
        let host = normalize_host(authority, &scheme).ok_or_else(|| UriError::MissingHost {
            input: input.to_owned(),
        })?;
        let (path, query) = split_path_query(tail);
        Ok(Self {
            scheme,
            host,
            path: normalize_path(path),
            query: normalize_query(query),
        })
    }

    /// Resolve one href against this URL (RFC 3986 §5.2 subset): absolute
    /// hrefs replace, protocol-relative hrefs keep the scheme, everything
    /// else resolves against this URL's path.
    pub fn resolve(&self, href: &str) -> Result<Self, UriError> {
        if split_scheme(href).is_some() {
            return Self::parse(href);
        }
        if let Some(rest) = href.strip_prefix("//") {
            return Self::parse(&format!("{}://{rest}", self.scheme));
        }
        let (href_path, query) = split_path_query(href.split('#').next().unwrap_or_default());
        let path = self.resolve_path(href_path);
        Ok(Self {
            scheme: self.scheme.clone(),
            host: self.host.clone(),
            path,
            query: normalize_query(query),
        })
    }

    fn resolve_path(&self, href_path: &str) -> String {
        if href_path.is_empty() {
            return self.path.clone();
        }
        if let Some(rest) = href_path.strip_prefix('/') {
            return normalize_path(rest);
        }
        let base = self.path.rsplit_once('/').map_or("", |(dir, _)| dir);
        normalize_path(&format!("{base}/{href_path}"))
    }

    /// The value of one query parameter.
    pub fn param(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    /// The normalized URL string.
    pub fn as_url(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}{}", self.scheme, self.host, self.path)?;
        if !self.query.is_empty() {
            let pairs: Vec<String> = self
                .query
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect();
            write!(f, "?{}", pairs.join("&"))?;
        }
        Ok(())
    }
}

/// A scheme separator: an RFC 3986 scheme, then `://`.
fn split_scheme(input: &str) -> Option<(&str, &str)> {
    let marker = input.find("://")?;
    let scheme = &input[..marker];
    let valid = !scheme.is_empty()
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then(|| (scheme, &input[marker + 3..]))
}

/// The lower-cased scheme, restricted to http and https.
fn http_scheme(input: &str, scheme: &str) -> Result<String, UriError> {
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(UriError::NotAbsolute {
            input: input.to_owned(),
        });
    }
    Ok(scheme)
}

fn split_authority(rest: &str) -> (&str, &str) {
    let end = rest.find(['/', '?']).unwrap_or(rest.len());
    (&rest[..end], &rest[end..])
}

fn split_path_query(tail: &str) -> (&str, Option<&str>) {
    match tail.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (tail, None),
    }
}

fn normalize_host(authority: &str, scheme: &str) -> Option<String> {
    let host_port = authority
        .split('@')
        .next_back()
        .filter(|value| !value.is_empty())?;
    let host = host_port.to_ascii_lowercase();
    let default = if scheme == "http" { ":80" } else { ":443" };
    Some(host.strip_suffix(default).unwrap_or(&host).to_owned())
}

// why: the portal serves pages, not directories, so a trailing slash and empty
// segments carry no meaning and a shared key must not keep them
fn normalize_path(path: &str) -> String {
    let mut output = String::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if let Some(index) = output.rfind('/') {
                    output.truncate(index);
                }
            }
            other => {
                output.push('/');
                output.push_str(other);
            }
        }
    }
    if output.is_empty() {
        "/".to_owned()
    } else {
        output
    }
}

fn normalize_query(query: Option<&str>) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = query
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(parse_pair)
        .collect();
    pairs.sort();
    pairs
}

fn parse_pair(pair: &str) -> (String, String) {
    match pair.split_once('=') {
        Some((key, value)) => (normalize_percent(key), normalize_percent(value)),
        None => (normalize_percent(pair), String::new()),
    }
}

// why: one target can be spelled %2f or %2F; the upper-case hex form is the
// normalized one so both spellings collapse to a single ledger key
fn normalize_percent(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            output.push(c);
            continue;
        }
        match escape_pair(&mut chars) {
            Some((first, second)) => {
                output.push('%');
                output.push(first.to_ascii_uppercase());
                output.push(second.to_ascii_uppercase());
            }
            None => output.push('%'),
        }
    }
    output
}

/// The two characters following a `%`, when both are hex digits.
fn escape_pair(chars: &mut std::str::Chars<'_>) -> Option<(char, char)> {
    let first = chars.next()?;
    let second = chars.next()?;
    (first.is_ascii_hexdigit() && second.is_ascii_hexdigit()).then_some((first, second))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(input: &str) -> Uri {
        Uri::parse(input).expect("url")
    }

    #[test]
    fn a_sorted_query_is_the_normalized_key() {
        let one = uri("https://portale.fitet.org/risultati/campionati/campionati.php?CONC=0&CAM=1");
        let other =
            uri("https://portale.fitet.org/risultati/campionati/campionati.php?CAM=1&CONC=0");
        assert_eq!(one, other);
        assert_eq!(
            one.as_url(),
            "https://portale.fitet.org/risultati/campionati/campionati.php?CAM=1&CONC=0"
        );
    }

    #[test]
    fn a_fragment_and_a_default_port_are_dropped() {
        let parsed = uri("https://Portale.FITET.org:443/menu.php#top");
        assert_eq!(parsed.as_url(), "https://portale.fitet.org/menu.php");
    }

    #[test]
    fn a_relative_href_resolves_against_the_base_directory() {
        let base = uri("https://portale.fitet.org/risultati/regioni/menu_reg.php?REG=2");
        let resolved = base
            .resolve("../campionati/campionati.php?CAM=240&CONC=1")
            .expect("resolve");
        assert_eq!(
            resolved.as_url(),
            "https://portale.fitet.org/risultati/campionati/campionati.php?CAM=240&CONC=1"
        );
        assert_eq!(resolved.param("CAM"), Some("240"));
    }

    #[test]
    fn a_root_relative_href_keeps_the_authority() {
        let base = uri("https://portale.fitet.org/risultati/campionati/Calendario.asp?CAM=1");
        let resolved = base.resolve("/menu.php").expect("resolve");
        assert_eq!(resolved.as_url(), "https://portale.fitet.org/menu.php");
    }

    #[test]
    fn a_query_only_href_keeps_the_path_and_replaces_the_query() {
        let base = uri("https://portale.fitet.org/risultati/campionati/menu.php");
        let resolved = base.resolve("?CID=2").expect("resolve");
        assert_eq!(
            resolved.as_url(),
            "https://portale.fitet.org/risultati/campionati/menu.php?CID=2"
        );
    }

    #[test]
    fn an_absolute_href_replaces_everything() {
        let base = uri("https://portale.fitet.org/menu.php");
        let resolved = base
            .resolve("https://other.example.org/x.php?a=1")
            .expect("resolve");
        assert_eq!(resolved.as_url(), "https://other.example.org/x.php?a=1");
    }

    #[test]
    fn a_protocol_relative_href_keeps_the_scheme() {
        let base = uri("https://portale.fitet.org/menu.php");
        let resolved = base
            .resolve("//portale.fitet.org/other.php")
            .expect("resolve");
        assert_eq!(resolved.as_url(), "https://portale.fitet.org/other.php");
    }

    #[test]
    fn dot_segments_collapse() {
        let base = uri("https://portale.fitet.org/risultati/regioni/menu_tor.php?REG=1");
        let resolved = base
            .resolve("../tornei/risultati_tornei.php?TOR=110&COMIT=1&ID=0")
            .expect("resolve");
        assert_eq!(
            resolved.as_url(),
            "https://portale.fitet.org/risultati/tornei/risultati_tornei.php?COMIT=1&ID=0&TOR=110"
        );
    }

    #[test]
    fn a_parent_beyond_the_root_stays_at_the_root() {
        let base = uri("https://portale.fitet.org/menu.php");
        let resolved = base.resolve("../../x.php").expect("resolve");
        assert_eq!(resolved.as_url(), "https://portale.fitet.org/x.php");
    }

    #[test]
    fn duplicate_parameters_survive_normalization() {
        let parsed = uri("https://portale.fitet.org/x.php?a=2&a=1");
        assert_eq!(parsed.param("a"), Some("1"));
        assert_eq!(parsed.as_url(), "https://portale.fitet.org/x.php?a=1&a=2");
    }

    #[test]
    fn percent_escapes_normalize_to_upper_case() {
        let parsed = uri("https://portale.fitet.org/x.php?term=a%2fb");
        assert_eq!(parsed.param("term"), Some("a%2Fb"));
    }

    #[test]
    fn a_relative_url_is_not_a_ledger_key() {
        assert_eq!(
            Uri::parse("menu.php"),
            Err(UriError::NotAbsolute {
                input: "menu.php".to_owned()
            })
        );
    }

    #[test]
    fn a_hostless_url_is_rejected() {
        assert_eq!(
            Uri::parse("https:///x.php"),
            Err(UriError::MissingHost {
                input: "https:///x.php".to_owned()
            })
        );
    }
}
