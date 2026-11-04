//! John Doe 101: read a user's identity and licenses, project three fields,
//! and export a report — through the real `ogar-loco` interpreter, over the
//! synthetic fixture directory.
//!
//! ```sh
//! cargo run -p ogar-flow-m365 --features fixture --example john_doe_101
//! ```

use ogar_flow_m365::fixture::{FixtureAuthority, FixtureDirectory, login};
use ogar_flow_m365::{compile, run_flow};

const PIPELINE: &str = r#"Get-MgUser -UserId "john.doe@example.test" |
    Get-IamLicenseEvidence |
    Select-Object DisplayName, Department, Licenses |
    Export-Report -Format JSON"#;

fn main() {
    let flow = compile(PIPELINE).expect("the John Doe pipeline is in the admitted subset");
    let login = login("fixture-signed:reader").expect("the fixture reader logs in");
    let report = run_flow(
        &login,
        &flow,
        &FixtureDirectory,
        &FixtureAuthority::standard(),
    )
    .expect("the reader is scoped for this report");
    println!("{}", report.to_json());
}
