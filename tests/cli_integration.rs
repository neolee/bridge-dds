use std::io::Write;
use std::process::{Command, Output, Stdio};

const MID_TRICK: &str = "\
[Position \"N:K9..Q8. T.T76.. 73.92.. AJ.J.6.\"]
[First \"N\"]
[Trump \"D\"]
[CurrentTrick \"E:ST S:S3 W:SA\"]
";

const DEAL: &str = "N:QJ6.K652.J85.T98 873.J97.AT764.Q4 K5.T83.KQ9.A7652 AT942.AQ4.32.KJ3";

fn run_bridge(input: &str, arguments: &[&str]) -> Output {
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
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn test_cli_mid_trick_score_side_and_exact_results() {
    let text = run_bridge(MID_TRICK, &["solve", "--trump", "D"]);
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

    let json = run_bridge(MID_TRICK, &["solve", "--trump", "D", "--format", "json"]);
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

#[test]
fn test_cli_prefixed_and_unprefixed_legacy_play_match() {
    let prefixed = format!("[Deal \"{}\"]\n[Play \"E:S3=S5=S2=SQ\"]\n", DEAL);
    let unprefixed = format!("[Deal \"{}\"]\n[Play \"S3=S5=S2=SQ\"]\n", DEAL);

    let prefixed = run_bridge(&prefixed, &["solve", "--trump", "S"]);
    assert!(
        prefixed.status.success(),
        "{}",
        String::from_utf8_lossy(&prefixed.stderr)
    );
    let unprefixed = run_bridge(&unprefixed, &["solve", "--trump", "S", "--declarer", "N"]);
    assert!(
        unprefixed.status.success(),
        "{}",
        String::from_utf8_lossy(&unprefixed.stderr)
    );
    assert_eq!(prefixed.stdout, unprefixed.stdout);
}

#[test]
fn test_cli_standard_play_uses_fixed_columns_and_changing_leaders() {
    let input = format!(
        "[Deal \"{}\"]\n[Play \"E\"]\nS3 S5 S2 SQ\nH7 H3 HA H2\n- - C3 C8\n",
        DEAL
    );
    let output = run_bridge(&input, &["solve", "--trump", "S"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("First: W"));
    assert!(text.contains("Current tricks: WC3 NC8"));
    assert!(text.contains("Next to act: E"));
}

#[test]
fn test_cli_contract_and_declarer_are_play_fallbacks() {
    let input = format!(
        "[Deal \"{}\"]\n[Contract \"4S\"]\n[Declarer \"N\"]\n[Play \"S3\"]\n",
        DEAL
    );
    let output = run_bridge(&input, &["solve"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Trump: S"));
    assert!(text.contains("Current tricks: ES3"));
    assert!(text.contains("Next to act: S"));

    let override_input = format!(
        "[Deal \"{}\"]\n[Contract \"4H\"]\n[Declarer \"E\"]\n[Play \"S3\"]\n",
        DEAL
    );
    let output = run_bridge(
        &override_input,
        &["solve", "--trump", "S", "--declarer", "N"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Trump: S"));
    assert!(text.contains("Current tricks: ES3"));
}

#[test]
fn test_cli_rejects_declarer_and_play_leader_conflict() {
    let input = format!("[Deal \"{}\"]\n[Declarer \"E\"]\n[Play \"E:S3\"]\n", DEAL);
    let output = run_bridge(&input, &["solve", "--trump", "S"]);
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "error: conflicting input: Play opening leader E does not follow declarer E"
    );
}

#[test]
fn test_cli_rejects_current_trick_order_during_parsing() {
    let input = "\
[Position \"N:AKQJ... .AKQJ.. ..AKQJ. ...AKQJ\"]
[First \"S\"]
[Trump \"NT\"]
[CurrentTrick \"E:HA N:SA\"]
";
    let output = run_bridge(input, &["solve", "--trump", "NT"]);
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "error: invalid position: CurrentTrick: expected S as player 2, got N"
    );
}
