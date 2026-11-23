//! Portal pages are legacy frameset markup: single-quoted attributes, mixed
//! `.asp`/`.php` spellings, and charset declarations that disagree with the
//! bytes (the recorded fixtures declare windows-1252 while the bytes are
//! UTF-8). Discovery reads attributes only — `href` on anchors, `src` on
//! frames — which are ASCII on every recorded page, so pages parse through a
//! lossy UTF-8 decode; text-faithful decoding arrives with the parsers
//! (G1.4).

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

// why: a windows-1252 declaration over UTF-8 bytes makes a strict decode fail,
// and discovery needs the ASCII targets, not the prose
fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
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
}
