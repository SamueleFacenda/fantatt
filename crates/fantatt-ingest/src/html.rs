//! Portal pages are legacy frameset markup: single-quoted attributes, mixed
//! `.asp`/`.php` spellings, and a charset declaration that disagrees with the
//! bytes (the recorded fixtures declare windows-1252 while the bytes are
//! UTF-8). The decoder trusts valid UTF-8 first and falls back to the
//! declared charset only when the bytes cannot be UTF-8, so a page reads as
//! the text the portal meant to serve whatever spelling its declaration
//! carries.

use scraper::{Html, Selector};

/// Parse one portal page into a queryable document.
pub fn parse_page(bytes: &[u8]) -> Html {
    Html::parse_document(&decode(bytes))
}

/// Every value the named attribute carries on the selected elements, in
/// document order.
pub fn targets(document: &Html, selector: &Selector, attribute: &str) -> Vec<String> {
    document
        .select(selector)
        .filter_map(|element| element.value().attr(attribute))
        .map(str::to_string)
        .collect()
}

/// The text of every selected element, in document order.
pub fn texts(document: &Html, selector: &Selector) -> Vec<String> {
    document
        .select(selector)
        .map(|element| element.text().collect::<Vec<_>>().concat())
        .collect()
}

fn decode(bytes: &[u8]) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    match declared_charset(bytes) {
        Some(charset) if is_latin_family(&charset) => decode_windows_1252(bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

// why: a legacy page that declares windows-1252 means the browser superset,
// and iso-8859-1 is the alias browsers map onto the same table
fn is_latin_family(charset: &str) -> bool {
    matches!(
        charset.to_ascii_lowercase().as_str(),
        "windows-1252" | "cp1252" | "iso-8859-1" | "latin1"
    )
}

/// How many head bytes are scanned for the charset declaration.
const CHARSET_SCAN_BYTES: usize = 2048;

// why: the declaration is needed before a document exists, so the head is
// scanned as bytes rather than parsed
fn declared_charset(bytes: &[u8]) -> Option<String> {
    let head = &bytes[..bytes.len().min(CHARSET_SCAN_BYTES)];
    let text = String::from_utf8_lossy(head).to_ascii_lowercase();
    let start = text.find("charset=")? + "charset=".len();
    let rest = &text[start..];
    // why: a declaration value ends at a quote, a semicolon, the meta tag's
    // close, or whitespace, whichever comes first
    let end = rest
        .find(|c: char| matches!(c, '"' | '\'' | ';' | '>') || c.is_whitespace())
        .unwrap_or(rest.len());
    Some(rest[..end].to_owned())
}

/// The printable characters windows-1252 places in the C1 range, where
/// iso-8859-1 leaves control codes; the five unmapped slots decode to the
/// replacement character.
const WINDOWS_1252_C1: [char; 32] = [
    '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{FFFD}', '\u{017D}', '\u{FFFD}',
    '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
];

fn decode_windows_1252(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len());
    for byte in bytes {
        match byte {
            0x00..=0x7F | 0xA0..=0xFF => text.push(*byte as char),
            other => text.push(WINDOWS_1252_C1[usize::from(other - 0x80)]),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_extract_in_document_order() {
        let page = br#"<html><frameset rows="*">
            <frame src="menu_reg.php?REG=1">
            <frame src='menu_tor.php?REG=1'>
        </frameset></html>"#;
        let document = parse_page(page);
        let frames = Selector::parse("frame").expect("frame selector");
        assert_eq!(
            targets(&document, &frames, "src"),
            ["menu_reg.php?REG=1", "menu_tor.php?REG=1"]
        );
    }

    // why: html5ever's tendril is !Send without the atomic feature, so a
    // document crossing a task boundary keeps that feature switched on
    #[tokio::test]
    async fn a_document_crosses_a_task_boundary() {
        let page = br#"<html><body><a href="one">1</a><a href="two">2</a></body></html>"#;
        let document = parse_page(page);
        let found = tokio::task::spawn(async move {
            let anchors = Selector::parse("a").expect("anchor selector");
            targets(&document, &anchors, "href")
        })
        .await
        .expect("task joins");
        assert_eq!(found, ["one", "two"]);
    }

    // why: the recorded pages declare windows-1252 over valid UTF-8 bytes, so
    // the declaration must not override text that already decodes
    #[test]
    fn utf8_bytes_decode_though_the_page_declares_windows_1252() {
        let page = b"<meta charset=windows-1252><p>Nicol\xc3\xb2 Andre</p>";
        let document = parse_page(page);
        let paragraph = Selector::parse("p").expect("paragraph selector");
        assert_eq!(texts(&document, &paragraph), ["Nicolò Andre"]);
    }

    #[test]
    fn genuine_windows_1252_bytes_decode_past_the_declaration() {
        let page = b"<meta charset=windows-1252><p>Nicol\xf2 Andre\x80\x96\x92\x99</p>";
        let document = parse_page(page);
        let paragraph = Selector::parse("p").expect("paragraph selector");
        assert_eq!(texts(&document, &paragraph), ["Nicolò Andre€–’™"]);
    }

    #[test]
    fn an_unmapped_c1_byte_decodes_to_the_replacement_character() {
        let page = b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=windows-1252\"><p>\x81</p>";
        let document = parse_page(page);
        let paragraph = Selector::parse("p").expect("paragraph selector");
        assert_eq!(texts(&document, &paragraph), ["\u{FFFD}"]);
    }

    #[test]
    fn undecodable_bytes_fall_back_to_lossy_replacement() {
        let page = b"<p>Nicol\xf2 Andre</p>";
        let document = parse_page(page);
        let paragraph = Selector::parse("p").expect("paragraph selector");
        assert_eq!(texts(&document, &paragraph), ["Nicol\u{FFFD} Andre"]);
    }
}
