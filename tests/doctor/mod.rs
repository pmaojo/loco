use loco_rs::{controller::cli_console::DoctorSnapshotResponse, introspection::cli::CommandOutput};
use serde_json::json;

#[test]
fn doctor_snapshot_response_parses_structured_payloads() {
    let output = CommandOutput::new(
        0,
        r#"{"ok":false,"checks":[{"name":"db","status":"failing"}]}"#,
        "db connectivity failed",
    );

    let response = DoctorSnapshotResponse::from(output);

    assert_eq!(response.status, 0);
    assert_eq!(
        response.stdout,
        json!({"ok": false, "checks": [{"name": "db", "status": "failing"}]})
    );
    assert_eq!(response.stderr, "db connectivity failed");
}

#[test]
fn doctor_snapshot_response_wraps_plain_text_output() {
    let output = CommandOutput::new(1, "doctor timed out", "");

    let response = DoctorSnapshotResponse::from(output);

    assert_eq!(response.status, 1);
    assert_eq!(response.stdout, json!({"raw": "doctor timed out"}));
    assert!(response.stderr.is_empty());
}
