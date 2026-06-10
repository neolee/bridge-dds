use std::io::Read;

use bridge_dds::core::deal::{Direction, Strain};
use bridge_dds::core::position::{CurrentTrick, PlayPosition, SnapshotPosition};
use bridge_dds::core::{self, Error};
use bridge_dds::dds::DdsSolver;
use clap::{Parser, Subcommand, ValueEnum};

mod output;
use output::*;

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
    use bridge_dds::core::deal::Hands;

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

    let hands = Hands::try_new(residual.hands)
        .map_err(|e| Error::InvalidPosition(format!("Hands: {}", e)))?;

    // Build CurrentTrick: derive leader from the first card's player.
    let ct = if residual.current_trick.is_empty() {
        CurrentTrick::empty(first)
    } else {
        let leader = residual.current_trick[0].0;
        let cards: Vec<_> = residual.current_trick.iter().map(|(_, c)| *c).collect();
        CurrentTrick::try_new(leader, cards)?
    };

    let snap = SnapshotPosition::try_new(hands, ct)?;

    // Resolve trump: CLI overrides PBN tag.
    let trump_str = trump_arg.or(residual.trump.map(|s| s.as_char().to_string()));

    if matrix {
        let pm = DdsSolver::solve_position_matrix(&snap)?;
        match format {
            OutputFormat::Text => print_text_position_matrix(&pm),
            OutputFormat::Json => print_json_position_matrix(&pm),
        }
        return Ok(());
    }

    let trump_s = trump_str.ok_or_else(|| {
        Error::InvalidTrump(
            "--trump is required for continuation analysis (or add a [Trump] tag)".into(),
        )
    })?;
    let trump = Strain::from_char(trump_s.chars().next().unwrap_or(' '))
        .ok_or_else(|| Error::InvalidTrump(trump_s.clone()))?;

    let play = PlayPosition::try_from(snap)?;
    let results = DdsSolver::solve_position(&play, trump)?;
    let snap_out = SnapshotPosition::try_from(&play)?;

    match format {
        OutputFormat::Text => print_text_continuation(&snap_out, trump, &results),
        OutputFormat::Json => print_json_continuation(&snap_out, trump, &results),
    }

    Ok(())
}

fn cmd_play_trace(
    format: OutputFormat,
    trump_arg: Option<String>,
    declarer_arg: Option<String>,
    input: &str,
) -> Result<(), Error> {
    use bridge_dds::core::deal::Hands;

    let board = core::pbn::parse_record(input)?;

    let trump_s = trump_arg
        .ok_or_else(|| Error::InvalidTrump("--trump is required for Play trace analysis".into()))?;
    let trump = Strain::from_char(trump_s.chars().next().unwrap_or(' '))
        .ok_or_else(|| Error::InvalidTrump(trump_s.clone()))?;

    let play_value = extract_tag_value(input, "Play")?;
    let (tag_leader, cards) = core::play::parse_play_tag(&play_value)?;

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

    let hands = Hands::try_new(board.deal.hands)
        .map_err(|e| Error::InvalidPosition(format!("Hands: {}", e)))?;
    let snap = SnapshotPosition::try_new(hands, CurrentTrick::empty(opening_leader))?;
    let mut track = PlayPosition::try_from(snap)?;

    for card in &cards {
        track
            .play_card(*card, trump)
            .map_err(|e| Error::InvalidPlayTrace(format!("card {}: {}", card.to_pbn(), e)))?;
    }

    let results = DdsSolver::solve_position(&track, trump)?;
    let snap_out = SnapshotPosition::try_from(&track)?;

    match format {
        OutputFormat::Text => print_text_continuation(&snap_out, trump, &results),
        OutputFormat::Json => print_json_continuation(&snap_out, trump, &results),
    }

    Ok(())
}

fn extract_tag_value(input: &str, tag: &str) -> Result<String, Error> {
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(&format!("[{} ", tag)) || line.starts_with(&format!("[{}\t", tag)) {
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
