use std::collections::BTreeMap;

use serde::Serialize;

use bridge_dds::core::deal::{Direction, Strain};
use bridge_dds::core::position::SnapshotPosition;
use bridge_dds::core::{Board, ParResult, TricksMatrix};

pub fn print_text_full_deal(table: &TricksMatrix, par: &ParResult) {
    use bridge_dds::core::deal::Side;

    println!("Deal matrix: tricks for declarer");

    print!("  ");
    for strain in Strain::all() {
        let s = match strain {
            Strain::NoTrump => "NT".to_string(),
            other => other.as_char().to_string(),
        };
        print!("{:>3}", s);
    }
    println!();

    for decl in Direction::all() {
        print!("{:>2}", decl.as_char().to_string());
        for strain in Strain::all() {
            print!("{:>3}", table.get(strain, decl));
        }
        println!();
    }

    print!("{:>2}", "NS");
    for strain in Strain::all() {
        let t = table.best_for_side(Side::NS, strain);
        print!("{:>3}", t);
    }
    println!();

    print!("{:>2}", "EW");
    for strain in Strain::all() {
        let t = table.best_for_side(Side::EW, strain);
        print!("{:>3}", t);
    }
    println!();

    let contracts = par.contracts.join(", ");
    let sign = if par.score > 0 { "+" } else { "" };
    println!("Par: {}; {}{}", contracts, sign, par.score);
}

pub fn print_json_full_deal(board: &Board, table: &TricksMatrix, par: &ParResult) {
    #[derive(Serialize)]
    struct Output {
        tricks: bridge_dds::core::tricks::TricksJson,
        par: ParEntry,
    }
    #[derive(Serialize)]
    struct ParEntry {
        score: i32,
        contracts: Vec<String>,
    }
    let output = Output {
        tricks: table.to_json(),
        par: ParEntry {
            score: par.score,
            contracts: par.contracts.clone(),
        },
    };
    let _ = board;
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

pub fn print_text_position_matrix(pm: &bridge_dds::dds::PositionMatrix) {
    println!("Position matrix: tricks for side to act");
    print!("  ");
    for strain in Strain::all() {
        let s = match strain {
            Strain::NoTrump => "NT".to_string(),
            other => other.as_char().to_string(),
        };
        print!("{:>3}", s);
    }
    println!();

    for (idx, &dir) in Direction::all().iter().enumerate() {
        print!("{:>2}", dir.as_char().to_string());
        for strain in Strain::all() {
            print!("{:>3}", pm.data[strain.dds_index()][idx]);
        }
        println!();
    }
}

pub fn print_json_position_matrix(pm: &bridge_dds::dds::PositionMatrix) {
    #[derive(Serialize)]
    struct MatrixOutput {
        row_semantics: &'static str,
        value_semantics: &'static str,
        values: serde_json::Value,
    }
    use serde_json::{json, Map};
    let mut values = Map::new();
    for (idx, dir) in Direction::all().iter().enumerate() {
        let mut strain_map = Map::new();
        for strain in Strain::all() {
            let key = if matches!(strain, Strain::NoTrump) {
                "NT".to_string()
            } else {
                strain.as_char().to_string()
            };
            strain_map.insert(key, json!(pm.data[strain.dds_index()][idx]));
        }
        values.insert(dir.as_char().to_string(), json!(strain_map));
    }
    let output = MatrixOutput {
        row_semantics: "next_to_act",
        value_semantics: "tricks_for_side_to_act",
        values: json!(values),
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

pub fn print_text_continuation(
    pos: &SnapshotPosition,
    trump: Strain,
    results: &[bridge_dds::dds::CardResult],
) {
    let trump_label = match trump {
        Strain::NoTrump => "NT".to_string(),
        other => other.as_char().to_string(),
    };

    let first = pos.current_trick().leader();
    let next_to_act = pos.current_trick().next_to_act();

    println!("Trump: {}", trump_label);
    println!("First: {}", first.as_char());
    if pos.current_trick().is_empty() {
        println!("Current tricks: (empty)");
    } else {
        print!("Current tricks: ");
        for i in 0..pos.current_trick().len() {
            let card = pos.current_trick().cards()[i];
            let player = pos.current_trick().player_at(i).unwrap();
            print!("{}{} ", player.as_char(), card.to_pbn());
        }
        println!();
    }
    println!("Next to act: {}", next_to_act.as_char());
    println!();

    let mut by_score: BTreeMap<u8, Vec<String>> = BTreeMap::new();
    for r in results {
        by_score
            .entry(r.tricks_for_side_to_act)
            .or_default()
            .push(r.card.to_pbn());
    }

    let side_label = match next_to_act {
        Direction::North | Direction::South => "NS",
        Direction::East | Direction::West => "EW",
    };
    println!(
        "{} plays for {} side tricks:",
        next_to_act.as_char(),
        side_label
    );
    for (score, cards) in by_score.iter().rev() {
        println!("{}: {}", score, cards.join(" "));
    }
}

pub fn print_json_continuation(
    pos: &SnapshotPosition,
    trump: Strain,
    results: &[bridge_dds::dds::CardResult],
) {
    #[derive(Serialize)]
    struct Continuation {
        trump: String,
        next_to_act: String,
        current_trick: Vec<String>,
        suggested: Vec<CardResultJson>,
    }
    #[derive(Serialize)]
    struct CardResultJson {
        card: String,
        tricks_for_side_to_act: u8,
        optimal: bool,
    }
    let output = Continuation {
        trump: match trump {
            Strain::NoTrump => "NT".to_string(),
            other => other.as_char().to_string(),
        },
        next_to_act: pos.current_trick().next_to_act().as_char().to_string(),
        current_trick: pos
            .current_trick()
            .cards()
            .iter()
            .map(|c| c.to_pbn())
            .collect(),
        suggested: results
            .iter()
            .map(|r| CardResultJson {
                card: r.card.to_pbn(),
                tricks_for_side_to_act: r.tricks_for_side_to_act,
                optimal: r.is_optimal,
            })
            .collect(),
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
