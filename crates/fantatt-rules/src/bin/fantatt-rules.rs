//! `fantatt-rules verify`: load every rule bundle under `rules/` and report the
//! content-addressed version each one holds.

use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            println!("rule bundles verified: {report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<fantatt_rules::Verification, fantatt_rules::BundleError> {
    fantatt_rules::BundleSet::discover()?.verify()
}