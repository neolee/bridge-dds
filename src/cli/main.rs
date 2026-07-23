use std::io::Read;

use bridge_dds::core::deal::{Direction, Strain};
use bridge_dds::core::pbn::{ParsedPlay, ParsedRecord};
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
    let record = core::pbn::parse_record(&input)?;

    if record.position.is_some() {
        return cmd_residual(format, trump_arg, first_arg, matrix, record);
    }

    if record.play.is_some() {
        return cmd_play_trace(format, trump_arg, declarer_arg, record);
    }

    let board = core::deal::Board {
        deal: record.deal.ok_or(Error::MissingPbnTag("Deal"))?,
        dealer: record.dealer.ok_or(Error::MissingPbnTag("Dealer"))?,
        vulnerable: record
            .vulnerable
            .ok_or(Error::MissingPbnTag("Vulnerable"))?,
    };
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
    record: ParsedRecord,
) -> Result<(), Error> {
    let hands = record.position.ok_or(Error::MissingPbnTag("Position"))?;

    // Resolve first: CLI overrides PBN tag. Must come from at least one source.
    let first = if let Some(ref f) = first_arg {
        let ch = f.trim().chars().next().unwrap_or(' ');
        Direction::from_char(ch).ok_or_else(|| Error::InvalidFirst(f.clone()))?
    } else if let Some(f) = record.first {
        f
    } else {
        return Err(Error::MissingPbnTag("First"));
    };

    let parsed_current_trick = record
        .current_trick
        .map(|current| current.cards)
        .unwrap_or_default();
    let ct = if parsed_current_trick.is_empty() {
        CurrentTrick::empty(first)
    } else {
        let leader = parsed_current_trick[0].player;
        let expected_first = leader.advance(parsed_current_trick.len());
        if first != expected_first {
            return Err(Error::ConflictingInput(format!(
                "First {} does not match CurrentTrick next player {}",
                first.as_char(),
                expected_first.as_char()
            )));
        }
        for directed in &parsed_current_trick {
            if !hands.get(directed.player).contains(directed.card) {
                return Err(Error::InvalidPosition(format!(
                    "CurrentTrick: {:?} does not hold {}",
                    directed.player,
                    directed.card.to_pbn()
                )));
            }
        }
        let cards: Vec<_> = parsed_current_trick
            .iter()
            .map(|directed| directed.card)
            .collect();
        CurrentTrick::try_new(leader, cards)?
    };

    let snap = SnapshotPosition::try_new(hands, ct)?;

    // Resolve trump: CLI overrides PBN tag.
    let trump_str = trump_arg.or(record.trump.map(|s| s.as_char().to_string()));

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
    record: ParsedRecord,
) -> Result<(), Error> {
    let deal = record.deal.ok_or(Error::MissingPbnTag("Deal"))?;
    let play = record.play.ok_or(Error::MissingPbnTag("Play"))?;

    let trump = if let Some(trump_s) = trump_arg {
        Strain::from_char(trump_s.chars().next().unwrap_or(' '))
            .ok_or_else(|| Error::InvalidTrump(trump_s.clone()))?
    } else if let Some(contract) = record.contract {
        contract.strain
    } else {
        return Err(Error::InvalidTrump(
            "--trump is required for Play trace analysis".into(),
        ));
    };

    let declarer = if let Some(declarer_s) = declarer_arg {
        Some(
            Direction::from_char(declarer_s.chars().next().unwrap_or(' '))
                .ok_or_else(|| Error::Dds(format!("invalid declarer: {}", declarer_s)))?,
        )
    } else {
        record.declarer
    };

    let opening_leader = match &play {
        ParsedPlay::Standard { first_column, .. } => *first_column,
        ParsedPlay::Legacy {
            opening_leader: Some(leader),
            ..
        } => *leader,
        ParsedPlay::Legacy {
            opening_leader: None,
            ..
        } => declarer.map(Direction::next).ok_or_else(|| {
            Error::Dds("--declarer is required when Play tag has no direction prefix".into())
        })?,
    };
    if let Some(declarer) = declarer {
        let expected = declarer.next();
        if opening_leader != expected {
            return Err(Error::ConflictingInput(format!(
                "Play opening leader {} does not follow declarer {}",
                opening_leader.as_char(),
                declarer.as_char()
            )));
        }
    }

    let normalized = core::play::normalize_play(&play, &deal, trump, opening_leader)?;
    let results = DdsSolver::solve_position(normalized.final_position(), trump)?;
    let snap_out = SnapshotPosition::try_from(normalized.final_position())?;

    match format {
        OutputFormat::Text => print_text_continuation(&snap_out, trump, &results),
        OutputFormat::Json => print_json_continuation(&snap_out, trump, &results),
    }

    Ok(())
}
