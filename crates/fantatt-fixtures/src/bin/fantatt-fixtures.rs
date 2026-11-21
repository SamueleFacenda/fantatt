//! `fantatt-fixtures verify`: check the committed fixture tree (G1.1).

use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            println!("fixture set verified: {report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<fantatt_fixtures::Verification, fantatt_fixtures::FixtureError> {
    fantatt_fixtures::FixtureSet::discover()?.verify()
}
