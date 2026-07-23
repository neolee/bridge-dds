use std::io::Write;
use std::process::{Command, Output, Stdio};

const MID_TRICK: &str = "\
[Position \"N:K9..Q8. T.T76.. 73.92.. AJ.J.6.\"]
[First \"N\"]
[Trump \"D\"]
[CurrentTrick \"E:ST S:S3 W:SA\"]
";

fn run_bridge(arguments: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bridge"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(MID_TRICK.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn test_cli_mid_trick_score_side_and_exact_results() {
    let text = run_bridge(&["solve", "--trump", "D"]);
    assert!(
        text.status.success(),
        "{}",
        String::from_utf8_lossy(&text.stderr)
    );
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("First: E"));
    assert!(text.contains("Current tricks: EST SS3 WSA"));
    assert!(text.contains("Next to act: N"));
    assert!(text.contains("N plays for NS side tricks:"));
    assert!(text.contains("3: S9"));
    assert!(text.contains("2: SK"));

    let json = run_bridge(&["solve", "--trump", "D", "--format", "json"]);
    assert!(
        json.status.success(),
        "{}",
        String::from_utf8_lossy(&json.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(json["score_side"], "NS");
    assert_eq!(json["next_to_act"], "N");
    assert_eq!(json["current_trick"], serde_json::json!(["ST", "S3", "SA"]));

    let suggested = json["suggested"].as_array().unwrap();
    assert!(suggested.iter().any(|result| {
        result["card"] == "S9" && result["tricks_for_score_side"] == 3 && result["optimal"] == true
    }));
    assert!(suggested.iter().any(|result| {
        result["card"] == "SK" && result["tricks_for_score_side"] == 2 && result["optimal"] == false
    }));
}
