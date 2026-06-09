use std::io::Read;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use bridge_dds::core::deal::{Direction, Strain};
use bridge_dds::core::position::Position;
use bridge_dds::core::{self, Board, Error, ParResult, TricksMatrix};
use bridge_dds::dds::DdsSolver;

#[derive(Parser)]
#[command(name = "bridge", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Evaluate a single deal or residual position
    Solve {
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,

        /// Trump suit: S, H, D, C, or NT
        #[arg(long)]
        trump: Option<String>,

        /// First player (next to act): N, E, S, or W
        #[arg(long)]
        first: Option<String>,

        /// Declarer (for Play trace import): N, E, S, or W
        #[arg(long)]
        declarer: Option<String>,

        /// Emit a position matrix instead of continuation analysis
        #[arg(long)]
        matrix: bool,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Solve {
            format,
            trump,
            first,
            declarer,
            matrix,
        } => {
            if let Err(e) = cmd_solve(format, trump, first, declarer, matrix) {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn cmd_solve(
    format: OutputFormat,
    trump_arg: Option<String>,
    first_arg: Option<String>,
    declarer_arg: Option<String>,
    matrix: bool,
) -> Result<(), Error> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    DdsSolver::init();

    // Try residual path first: if Position tag is present.
    if input.contains("[Position ") {
        return cmd_residual(format, trump_arg, first_arg, matrix, &input);
    }

    // Full-deal path. If a Play tag is present, route to play trace analysis.
    if input.contains("[Play ") {
        return cmd_play_trace(format, trump_arg, declarer_arg, &input);
    }

    let board = core::pbn::parse_record(&input)?;
    let table = DdsSolver::solve_table(&board.deal)?;
    let par = DdsSolver::compute_par(&table, board.dealer, board.vulnerable)?;

    match format {
        OutputFormat::Text => print_text_full_deal(&table, &par),
        OutputFormat::Json => print_json_full_deal(&board, &table, &par),
    }

    Ok(())
}

fn cmd_residual(
    format: OutputFormat,
    trump_arg: Option<String>,
    first_arg: Option<String>,
    matrix: bool,
    input: &str,
) -> Result<(), Error> {
    let residual = core::pbn::parse_residual_record(input)?;

    // Resolve first: CLI overrides PBN tag. Must come from at least one source.
    let first = if let Some(ref f) = first_arg {
        let ch = f.trim().chars().next().unwrap_or(' ');
        Direction::from_char(ch).ok_or_else(|| Error::InvalidFirst(f.clone()))?
    } else if let Some(f) = residual.first {
        f
    } else {
        return Err(Error::MissingPbnTag("First"));
    };

    let pos = Position {
        hands: residual.hands,
        next_to_act: first,
        current_trick: residual
            .current_trick
            .into_iter()
            .map(|(player, card)| bridge_dds::core::position::PlayedCard { player, card })
            .collect(),
    };

    // Resolve trump: CLI overrides PBN tag.
    let trump_str = trump_arg.or(residual.trump.map(|s| s.as_char().to_string()));

    if matrix {
        // Position matrix: all strains.
        let pm = DdsSolver::solve_position_matrix(&pos)?;
        match format {
            OutputFormat::Text => print_text_position_matrix(&pm),
            OutputFormat::Json => print_json_position_matrix(&pm),
        }
        return Ok(());
    }

    // Continuation analysis: need trump.
    let trump_s = trump_str.ok_or_else(|| {
        Error::InvalidTrump(
            "--trump is required for continuation analysis (or add a [Trump] tag)".into(),
        )
    })?;
    let trump = Strain::from_char(trump_s.chars().next().unwrap_or(' '))
        .ok_or_else(|| Error::InvalidTrump(trump_s.clone()))?;

    let results = DdsSolver::solve_position(&pos, trump)?;

    match format {
        OutputFormat::Text => print_text_continuation(&pos, trump, &results),
        OutputFormat::Json => print_json_continuation(&pos, trump, &results),
    }

    Ok(())
}

// --- Full-deal text output (Phase 1a) ---

fn cmd_play_trace(
    format: OutputFormat,
    trump_arg: Option<String>,
    declarer_arg: Option<String>,
    input: &str,
) -> Result<(), Error> {
    let board = core::pbn::parse_record(input)?;

    let trump_s = trump_arg
        .ok_or_else(|| Error::InvalidTrump("--trump is required for Play trace analysis".into()))?;
    let trump = Strain::from_char(trump_s.chars().next().unwrap_or(' '))
        .ok_or_else(|| Error::InvalidTrump(trump_s.clone()))?;

    let play_value = extract_tag_value(input, "Play")?;
    let (tag_leader, cards) = core::play::parse_play_tag(&play_value)?;

    // Opening leader: Play tag prefix takes priority, otherwise derive from declarer.
    let opening_leader = if let Some(tl) = tag_leader {
        tl
    } else {
        let declarer_s = declarer_arg.ok_or_else(|| {
            Error::Dds("--declarer is required when Play tag has no direction prefix".into())
        })?;
        let declarer = Direction::from_char(declarer_s.chars().next().unwrap_or(' '))
            .ok_or_else(|| Error::Dds(format!("invalid declarer: {}", declarer_s)))?;
        declarer.next()
    };

    if cards.is_empty() {
        let pos = Position {
            hands: board.deal.hands,
            next_to_act: opening_leader,
            current_trick: vec![],
        };
        let results = DdsSolver::solve_position(&pos, trump)?;
        match format {
            OutputFormat::Text => print_text_continuation(&pos, trump, &results),
            OutputFormat::Json => print_json_continuation(&pos, trump, &results),
        }
        return Ok(());
    }

    // Simulate play for turn tracking and validation.
    let mut track = Position {
        hands: board.deal.hands,
        next_to_act: opening_leader,
        current_trick: vec![],
    };
    for card in &cards {
        track = track
            .play_card(*card, trump)
            .map_err(|e| Error::InvalidPlayTrace(format!("card {}: {}", card.to_pbn(), e)))?;
    }

    // Build final Position: all original cards in hands, only the
    // final incomplete trick in current_trick (from the tracking state).
    let pos = Position {
        hands: board.deal.hands,
        next_to_act: track.next_to_act,
        current_trick: track.current_trick,
    };

    let results = DdsSolver::solve_position(&pos, trump)?;

    match format {
        OutputFormat::Text => print_text_continuation(&pos, trump, &results),
        OutputFormat::Json => print_json_continuation(&pos, trump, &results),
    }

    Ok(())
}

/// Extract the value of a PBN tag from raw input.
fn extract_tag_value(input: &str, tag: &str) -> Result<String, Error> {
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(&format!("[{} ", tag)) || line.starts_with(&format!("[{}\t", tag)) {
            // Extract value between quotes.
            let start = line
                .find('"')
                .ok_or_else(|| Error::PbnParse(format!("missing quote in {} tag", tag)))?;
            let end = line
                .rfind('"')
                .ok_or_else(|| Error::PbnParse(format!("missing closing quote in {} tag", tag)))?;
            return Ok(line[start + 1..end].to_string());
        }
    }
    Err(Error::PbnParse(format!("missing {} tag", tag)))
}

fn print_text_full_deal(table: &TricksMatrix, par: &ParResult) {
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

fn print_json_full_deal(board: &Board, table: &TricksMatrix, par: &ParResult) {
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

// --- Position matrix output ---

fn print_text_position_matrix(pm: &bridge_dds::dds::PositionMatrix) {
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

fn print_json_position_matrix(pm: &bridge_dds::dds::PositionMatrix) {
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

// --- Continuation analysis output ---

fn print_text_continuation(pos: &Position, trump: Strain, results: &[bridge_dds::dds::CardResult]) {
    use std::collections::BTreeMap;

    let trump_label = match trump {
        Strain::NoTrump => "NT".to_string(),
        other => other.as_char().to_string(),
    };

    let first = pos
        .current_trick
        .first()
        .map(|p| p.player)
        .unwrap_or(pos.next_to_act);

    println!("Trump: {}", trump_label);
    println!("First: {}", first.as_char());
    if pos.current_trick.is_empty() {
        println!("Current tricks: (empty)");
    } else {
        print!("Current tricks: ");
        for p in &pos.current_trick {
            print!("{}{} ", p.player.as_char(), p.card.to_pbn());
        }
        println!();
    }
    println!("Next to act: {}", pos.next_to_act.as_char());
    println!();

    // Group by score, descending.
    let mut by_score: BTreeMap<u8, Vec<String>> = BTreeMap::new();
    for r in results {
        by_score
            .entry(r.tricks_for_side_to_act)
            .or_default()
            .push(r.card.to_pbn());
    }

    let score_side = if let Some(first_played) = pos.current_trick.first() {
        first_played.player
    } else {
        pos.next_to_act
    };
    let side_label = match score_side {
        Direction::North | Direction::South => "NS",
        Direction::East | Direction::West => "EW",
    };
    println!(
        "{} plays for {} side tricks:",
        pos.next_to_act.as_char(),
        side_label
    );
    for (score, cards) in by_score.iter().rev() {
        println!("{}: {}", score, cards.join(" "));
    }
}

fn print_json_continuation(pos: &Position, trump: Strain, results: &[bridge_dds::dds::CardResult]) {
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
        next_to_act: pos.next_to_act.as_char().to_string(),
        current_trick: pos
            .current_trick
            .iter()
            .map(|p| format!("{}{}", p.player.as_char(), p.card.to_pbn()))
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
