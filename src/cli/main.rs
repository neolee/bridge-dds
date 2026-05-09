use std::io::Read;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

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
    /// Evaluate a single deal
    Solve {
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
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
        Command::Solve { format } => {
            if let Err(e) = cmd_solve(format) {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

fn cmd_solve(format: OutputFormat) -> Result<(), Error> {
    // Read one PBN record from stdin.
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    let board = core::pbn::parse_record(&input)?;

    DdsSolver::init();
    let table = DdsSolver::solve_table(&board.deal)?;
    let par = DdsSolver::compute_par(&table, board.dealer, board.vulnerable)?;

    match format {
        OutputFormat::Text => print_text(&board, &table, &par),
        OutputFormat::Json => print_json(&board, &table, &par),
    }

    Ok(())
}

fn print_text(_board: &Board, table: &TricksMatrix, par: &ParResult) {
    use bridge_dds::core::deal::{Direction, Side, Strain};

    // Header: label column (2 chars empty), then each strain in 3 chars.
    print!("  ");
    for strain in Strain::all() {
        let s = match strain {
            Strain::NoTrump => "NT".to_string(),
            other => other.as_char().to_string(),
        };
        print!("{:>3}", s);
    }
    println!();

    // Individual declarer rows.
    for decl in Direction::all() {
        print!("{:>2}", decl.as_char().to_string());
        for strain in Strain::all() {
            print!("{:>3}", table.get(strain, decl));
        }
        println!();
    }

    // Side summary rows.
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

    // Par line.
    let contracts = par.contracts.join(", ");
    let sign = if par.score > 0 { "+" } else { "" };
    println!("Par: {}; {}{}", contracts, sign, par.score);
}

fn print_json(board: &Board, table: &TricksMatrix, par: &ParResult) {
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

    let _ = board; // not emitted in JSON currently, but available.

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
