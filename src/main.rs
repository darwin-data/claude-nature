mod footprint;
mod sites;
mod transcript;

use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::json;

use footprint::Factors;
use sites::Sites;
use transcript::{Filter, Summary, summarise, transcripts};

/// The nature footprint of your Claude Code sessions: energy, climate and water,
/// read against the water stress of the basin each data centre draws from.
#[derive(Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Totals over the transcripts still on disk (Claude Code keeps about 30 days).
    Report {
        /// Start date, inclusive (YYYY-MM-DD).
        #[arg(long)]
        since: Option<String>,
        /// Keep only the sessions whose working directory contains this text.
        #[arg(long)]
        project: Option<String>,
        /// Print JSON instead of text.
        #[arg(long)]
        json: bool,
    },
    /// One line for the Claude Code status line. Reads the status line JSON on stdin.
    Statusline,
    /// The candidate data centres and the water stress of their basins.
    Sites,
}

fn main() -> Result<()> {
    let factors = Factors::embedded();
    match Cli::parse().command {
        Command::Report {
            since,
            project,
            json,
        } => {
            let root = projects_dir()?;
            let files = transcripts(&root);
            let summary = summarise(
                &factors,
                &files,
                &Filter {
                    since: since.clone(),
                    project,
                },
            );
            if json {
                println!("{}", serde_json::to_string_pretty(&report_json(&summary))?);
            } else {
                print_report(&summary, since.as_deref());
            }
        }
        Command::Statusline => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            println!("{}", statusline(&factors, &input));
        }
        Command::Sites => print_sites(),
    }
    Ok(())
}

fn projects_dir() -> Result<PathBuf> {
    let config = match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(".claude"),
    };
    Ok(config.join("projects"))
}

#[derive(Deserialize)]
struct StatusInput {
    transcript_path: Option<String>,
}

/// The session's own transcript, plus its subagents' under `<session>/`.
fn statusline(factors: &Factors, input: &str) -> String {
    let Some(path) = serde_json::from_str::<StatusInput>(input)
        .ok()
        .and_then(|i| i.transcript_path)
    else {
        return String::new();
    };
    let main = PathBuf::from(&path);
    let mut files = vec![main.clone()];
    files.extend(transcripts(&main.with_extension("")));
    let s = summarise(factors, &files, &Filter::default());
    format!(
        "💧 {} · {}",
        water(s.footprint.water_l()),
        co2e(s.footprint.co2e_g)
    )
}

fn report_json(s: &Summary) -> serde_json::Value {
    json!({
        "sessions": s.sessions,
        "requests": s.requests,
        "tokens": {
            "input": s.tokens.input,
            "cache_write": s.tokens.cache_write,
            "cache_read": s.tokens.cache_read,
            "output": s.tokens.output,
        },
        "energy_wh": s.footprint.energy_wh,
        "co2e_g": s.footprint.co2e_g,
        "water_on_site_l": s.footprint.water_on_site_l,
        "water_off_site_l": s.footprint.water_off_site_l,
        "sites": Sites::embedded().sites,
    })
}

fn print_report(s: &Summary, since: Option<&str>) {
    let period = since.map_or("all transcripts on disk".to_owned(), |d| {
        format!("since {d}")
    });
    println!("claude-nature · {period} · {} sessions\n", s.sessions);
    let cache_share = if s.tokens.total() > 0 {
        100.0 * s.tokens.cache_read as f64 / s.tokens.total() as f64
    } else {
        0.0
    };
    println!(
        "  Tokens    {} ({cache_share:.0}% cache reads)",
        tokens(s.tokens.total())
    );
    println!("  Energy    {}", energy(s.footprint.energy_wh));
    println!("  Climate   {}", co2e(s.footprint.co2e_g));
    println!(
        "  Water     {}  ({} cooling on site, {} to make the electricity)\n",
        water(s.footprint.water_l()),
        water(s.footprint.water_on_site_l),
        water(s.footprint.water_off_site_l),
    );
    println!(
        "  A litre does not weigh the same in every basin. Cooling water is drawn at the site:"
    );
    print_site_table();
    println!(
        "\n  Anthropic does not publish how it splits requests between sites. Each row is a \"what if it all ran here\"."
    );
    println!("  Order of magnitude only. See METHODOLOGY.md.");
    if s.excluded_requests > 0 {
        println!(
            "  {} requests to non-Claude models are left out.",
            s.excluded_requests
        );
    }
}

fn print_sites() {
    let sites = Sites::embedded();
    print_site_table();
    println!();
    for site in &sites.sites {
        println!("  {}: {}", site.id, site.why);
    }
    println!("\n  Source: {}", sites.source);
}

fn print_site_table() {
    println!(
        "\n  {:<58} {:<15} {:>22}",
        "Site", "Basin stress", "Withdrawals / supply"
    );
    for site in Sites::embedded().sites {
        println!(
            "  {:<58} {:<15} {:>21.0}%",
            site.name,
            site.aqueduct.bws_label,
            site.aqueduct.bws_raw * 100.0
        );
    }
}

fn tokens(n: u64) -> String {
    let n = n as f64;
    match n {
        n if n >= 1e9 => format!("{:.1} B", n / 1e9),
        n if n >= 1e6 => format!("{:.1} M", n / 1e6),
        n if n >= 1e3 => format!("{:.1} K", n / 1e3),
        n => format!("{n:.0}"),
    }
}

fn energy(wh: f64) -> String {
    if wh >= 1000.0 {
        format!("{:.1} kWh", wh / 1000.0)
    } else {
        format!("{wh:.1} Wh")
    }
}

fn co2e(g: f64) -> String {
    if g >= 1000.0 {
        format!("{:.1} kgCO₂e", g / 1000.0)
    } else {
        format!("{g:.1} gCO₂e")
    }
}

fn water(l: f64) -> String {
    if l >= 1.0 {
        format!("{l:.1} L")
    } else {
        format!("{:.0} mL", l * 1000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statusline_reads_the_session_and_its_subagents() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/projects/-tmp-demo/sess-a.jsonl"
        );
        let line = statusline(
            &Factors::embedded(),
            &json!({ "transcript_path": path }).to_string(),
        );
        assert!(line.starts_with("💧 "), "{line}");
        assert!(line.contains("gCO₂e"), "{line}");
    }

    #[test]
    fn statusline_is_empty_without_a_transcript() {
        assert_eq!(statusline(&Factors::embedded(), "{}"), "");
    }

    #[test]
    fn units() {
        assert_eq!(water(0.25), "250 mL");
        assert_eq!(water(12.34), "12.3 L");
        assert_eq!(co2e(1500.0), "1.5 kgCO₂e");
        assert_eq!(tokens(25_300_000_000), "25.3 B");
    }
}
