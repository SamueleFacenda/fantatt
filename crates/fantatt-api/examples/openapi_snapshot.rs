//! Regenerates the committed OpenAPI snapshot.
//!
//! Run it after a handler or response-shape change and check the diff: an
//! undocumented change fails the snapshot test, and this example is the
//! sanctioned way to update it deliberately.

use fantatt_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    println!("{}", ApiDoc::openapi().to_pretty_json().expect("serialize"));
}
