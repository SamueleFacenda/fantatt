use crate::error::GraphError;
use oxigraph::model::{Literal, NamedNode};
use time::format_description::OwnedFormatItem;
use time::{Date, OffsetDateTime};

pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const TT: &str = "https://fantatt.org/ontology/tt#";
pub const PROV: &str = "http://www.w3.org/ns/prov#";
pub const DATA: &str = "https://fantatt.org/data/";
pub const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

const DATE_FORMAT: &str = "[year]-[month]-[day]";
const DATE_TIME_FORMAT: &str = "[year]-[month]-[day]T[hour]:[minute]:[second]Z";

const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

/// Percent-encodes every character an IRI path segment cannot carry, so a
/// portal name or URL stays one addressable segment instead of breaking the
/// IRI it embeds it.
pub fn segment(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        let safe = byte.is_ascii_alphanumeric() || b"-._~".contains(&byte);
        if safe {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(HEX_DIGITS[(byte >> 4) as usize]);
            encoded.push(HEX_DIGITS[(byte & 0x0f) as usize]);
        }
    }
    encoded
}

pub fn rdf_term(term: &str) -> Result<NamedNode, GraphError> {
    node_in(RDF, term)
}

pub fn ontology(term: &str) -> Result<NamedNode, GraphError> {
    node_in(TT, term)
}

pub fn data(segment_path: &str) -> Result<NamedNode, GraphError> {
    node_in(DATA, segment_path)
}

pub fn prov(term: &str) -> Result<NamedNode, GraphError> {
    node_in(PROV, term)
}

fn node_in(namespace: &str, term: &str) -> Result<NamedNode, GraphError> {
    let iri = format!("{namespace}{term}");
    NamedNode::new(iri.clone()).map_err(|source| GraphError::Iri { iri, source })
}

pub fn string_literal(value: &str) -> Literal {
    Literal::new_simple_literal(value)
}

pub fn typed_literal(value: &str, datatype: NamedNode) -> Literal {
    Literal::new_typed_literal(value, datatype)
}

pub fn integer_literal(value: i64) -> Result<Literal, GraphError> {
    Ok(typed_literal(&value.to_string(), xsd("integer")?))
}

pub fn boolean_literal(value: bool) -> Result<Literal, GraphError> {
    let text = if value { "true" } else { "false" };
    Ok(typed_literal(text, xsd("boolean")?))
}

pub fn date_literal(value: Date) -> Result<Literal, GraphError> {
    let format = parse_format(DATE_FORMAT, "date")?;
    let text = value
        .format(&format)
        .map_err(|source| GraphError::Literal {
            kind: "date",
            value: value.to_string(),
            source,
        })?;
    Ok(typed_literal(&text, xsd("date")?))
}

pub fn date_time_literal(value: OffsetDateTime) -> Result<Literal, GraphError> {
    let format = parse_format(DATE_TIME_FORMAT, "dateTime")?;
    let text = value
        .format(&format)
        .map_err(|source| GraphError::Literal {
            kind: "dateTime",
            value: value.to_string(),
            source,
        })?;
    Ok(typed_literal(&text, xsd("dateTime")?))
}

fn parse_format(format: &str, kind: &'static str) -> Result<OwnedFormatItem, GraphError> {
    time::format_description::parse_owned::<2>(format)
        .map_err(|source| GraphError::FormatDescription { kind, source })
}

pub fn xsd(term: &str) -> Result<NamedNode, GraphError> {
    let iri = format!("{XSD}{term}");
    NamedNode::new(iri.clone()).map_err(|source| GraphError::Iri { iri, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn day() -> Date {
        Date::from_calendar_date(2026, Month::November, 8).expect("valid date")
    }

    #[test]
    fn a_segment_keeps_safe_characters_and_encodes_the_rest() {
        assert_eq!(
            segment("plr_01ARZ3XK7M0C4V6B8N2QWE9RTA"),
            "plr_01ARZ3XK7M0C4V6B8N2QWE9RTA"
        );
        assert_eq!(segment("Sassari (SS)"), "Sassari%20%28SS%29");
        assert_eq!(segment("a/b?c#d"), "a%2Fb%3Fc%23d");
    }

    #[test]
    fn a_date_renders_as_an_xsd_date() {
        let literal = date_literal(day()).expect("date literal");
        assert_eq!(
            literal.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#date"
        );
        assert_eq!(literal.value(), "2026-11-08");
    }

    #[test]
    fn an_instant_renders_as_an_xsd_date_time() {
        let instant = day().with_hms(20, 41, 3).expect("instant").assume_utc();
        let literal = date_time_literal(instant).expect("date time literal");
        assert_eq!(
            literal.datatype().as_str(),
            "http://www.w3.org/2001/XMLSchema#dateTime"
        );
        assert_eq!(literal.value(), "2026-11-08T20:41:03Z");
    }

    #[test]
    fn typed_terms_carry_their_ontology_namespace() {
        assert_eq!(
            ontology("Matchday").expect("term").as_str(),
            "https://fantatt.org/ontology/tt#Matchday"
        );
        assert_eq!(
            data("matchday/md_01").expect("term").as_str(),
            "https://fantatt.org/data/matchday/md_01"
        );
        assert_eq!(
            prov("Activity").expect("term").as_str(),
            "http://www.w3.org/ns/prov#Activity"
        );
    }
}
