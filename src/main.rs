mod cache;
mod footprint;
mod nature;
mod sites;
mod transcript;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::SystemTime;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use serde_json::json;

use footprint::{Factors, Footprint};
use nature::Nature;
use sites::Sites;
use transcript::{Filter, Scan, Summary, summarise, summarise_records, transcripts};

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
    /// Lines for the Claude Code status line: this session, then today. Reads the status
    /// line JSON on stdin.
    Statusline {
        /// Run this status line command first, with the same stdin, and print its output
        /// above. This is how `install-statusline` keeps an existing status line.
        #[arg(long)]
        wrap: Option<String>,
        /// Leave out the line for today.
        #[arg(long)]
        no_today: bool,
    },
    /// Adds the status line to Claude Code settings. An existing status line stays on top.
    InstallStatusline,
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
        Command::Statusline { wrap, no_today } => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            if let Some(command) = wrap {
                let above = run_wrapped(&command, &input);
                if !above.is_empty() {
                    println!("{above}");
                }
            }
            let color = std::env::var_os("NO_COLOR").is_none();
            let lines = statusline(&factors, &input, color, !no_today);
            if !lines.is_empty() {
                println!("{lines}");
            }
        }
        Command::InstallStatusline => install_statusline()?,
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

/// Runs a status line command with the same stdin, and returns its output without the
/// final newline. A command that fails prints nothing: the nature lines still show.
fn run_wrapped(command: &str, input: &str) -> String {
    let Ok(mut child) = Process::new("sh")
        .args(["-c", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return String::new();
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    child
        .wait_with_output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_owned())
        .unwrap_or_default()
}

/// The session's own transcript plus its subagents' under `<session>/`, then, if `today`,
/// every transcript of the same projects directory written since local midnight.
fn statusline(factors: &Factors, input: &str, color: bool, today: bool) -> String {
    let Some(path) = serde_json::from_str::<StatusInput>(input)
        .ok()
        .and_then(|i| i.transcript_path)
    else {
        return String::new();
    };
    let main = PathBuf::from(&path);
    let mut files = vec![main.clone()];
    files.extend(transcripts(&main.with_extension("")));
    let scans: Vec<Scan> = files.iter().map(|f| cache::scan_cached(f)).collect();
    let records = || scans.iter().flat_map(|s| &s.records);
    let session = summarise_records(factors, records(), &Filter::default());
    let turn = scans[0].last_prompt.as_ref().map(|since| {
        let filter = Filter {
            since: Some(since.clone()),
            ..Filter::default()
        };
        summarise_records(factors, records(), &filter).footprint
    });
    let paint = Paint(color);
    let mut extras = Vec::new();
    if let Some(turn) = turn {
        extras.push(format!("↳ last turn +{}", co2e(turn.total_co2e_g())));
    }
    if let Some(share) = session.tokens.cache_share() {
        extras.push(format!("♻️ {:.0}% cache", share * 100.0));
    }
    let mut out = metrics(&session.footprint, paint);
    if !extras.is_empty() {
        out.push_str("  ");
        out.push_str(&paint.grey(extras.join(" · ")));
    }
    if today
        && let Some(projects) = main.parent().and_then(Path::parent)
        && let Some(day) = today_summary(factors, projects)
        && day.requests > 0
    {
        out.push('\n');
        out.push_str(&paint.grey("📅 today ".to_owned()));
        out.push_str(&metrics(&day.footprint, paint));
        let sessions = if day.sessions == 1 {
            "session"
        } else {
            "sessions"
        };
        out.push_str(&paint.grey(format!("  {} {sessions}", day.sessions)));
    }
    out
}

/// Every request since local midnight, in the transcripts written since then.
fn today_summary(factors: &Factors, projects: &Path) -> Option<Summary> {
    let midnight = jiff::Zoned::now().start_of_day().ok()?.timestamp();
    let since = SystemTime::from(midnight);
    let files: Vec<PathBuf> = transcripts(projects)
        .into_iter()
        .filter(|f| {
            std::fs::metadata(f)
                .and_then(|m| m.modified())
                .is_ok_and(|t| t >= since)
        })
        .collect();
    cache::prune();
    let scans: Vec<Scan> = files.iter().map(|f| cache::scan_cached(f)).collect();
    let filter = Filter {
        since: Some(midnight.to_string()),
        ..Filter::default()
    };
    Some(summarise_records(
        factors,
        scans.iter().flat_map(|s| &s.records),
        &filter,
    ))
}

#[derive(Clone, Copy)]
struct Paint(bool);

impl Paint {
    fn code(self, code: &str, text: String) -> String {
        if self.0 {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text
        }
    }

    fn grey(self, text: String) -> String {
        self.code("90", text)
    }
}

/// Energy, water, CO₂e and nature, each in its own colour, with grey equivalents.
fn metrics(fp: &Footprint, paint: Paint) -> String {
    let nature = Nature::embedded();
    let (lo, hi) = min_max(
        Sites::embedded()
            .sites
            .iter()
            .map(|site| nature.at_site(site, fp).equivalent_m2_yr),
    );
    let like = |text: String| paint.grey(format!("≈ {text}"));
    [
        paint.code("93", format!("⚡ {}", energy(fp.energy_wh))),
        paint.code("96", format!("💧 {}", water(fp.water_l()))),
        format!(
            "{} {}",
            paint.code("37", format!("🏭 {}", co2e(fp.total_co2e_g()))),
            like(format!("🚗 {}", car_km(fp.total_co2e_g())))
        ),
        format!(
            "{} {}",
            paint.code("92", format!("🌱 {} PDF·m²·yr", range(lo, hi))),
            like(area((lo + hi) / 2.0))
        ),
    ]
    .join("  ")
}

/// Points Claude Code at `claude-nature statusline`. An existing status line command is
/// kept and passed to `--wrap`, so it still shows on the first line. The old file is
/// copied to `settings.json.bak-claude-nature` first.
fn install_statusline() -> Result<()> {
    let path = projects_dir()?
        .parent()
        .context("no Claude Code config directory")?
        .join("settings.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_owned());
    let mut settings: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("{} is not JSON", path.display()))?;
    let current = settings
        .pointer("/statusLine/command")
        .and_then(|c| c.as_str())
        .map(str::to_owned);
    if current
        .as_deref()
        .is_some_and(|c| c.contains("claude-nature"))
    {
        println!("The status line already calls claude-nature: nothing to do.");
        return Ok(());
    }
    let command = match &current {
        Some(old) => format!("claude-nature statusline --wrap {}", shell_quote(old)),
        None => "claude-nature statusline".to_owned(),
    };
    let object = settings
        .as_object_mut()
        .context("settings.json is not a JSON object")?;
    object.insert(
        "statusLine".to_owned(),
        json!({ "type": "command", "command": command, "padding": 0 }),
    );
    if path.exists() {
        std::fs::copy(&path, path.with_extension("json.bak-claude-nature"))?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&settings)? + "\n")?;
    println!("Status line set in {}:\n  {command}", path.display());
    if current.is_some() {
        println!("Your previous status line stays on the first line.");
    }
    Ok(())
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}
/// EPA, "Greenhouse Gas Emissions from a Typical Passenger Vehicle": about 400 g CO₂ per
/// mile, so 400 / 1.609 g per km.
/// <https://www.epa.gov/greenvehicles/greenhouse-gas-emissions-typical-passenger-vehicle>
const CAR_G_CO2_PER_KM: f64 = 400.0 / 1.609_344;

fn car_km(co2e_g: f64) -> String {
    let km = co2e_g / CAR_G_CO2_PER_KM;
    if km >= 10.0 {
        format!("{km:.0} km")
    } else {
        format!("{km:.1} km")
    }
}

/// Reference areas in m², smallest first. A4 sheet: ISO 216, 210 × 297 mm. Parking space:
/// 2.5 × 5 m. Tennis court: ITF doubles court, 23.77 × 10.97 m. Football pitch: FIFA
/// Laws of the Game, 105 × 68 m.
const AREAS: [(f64, &str, &str); 5] = [
    (0.210 * 0.297, "📄", "A4"),
    (2.5 * 5.0, "🅿️", "parking spaces"),
    (23.77 * 10.97, "🎾", "tennis courts"),
    (105.0 * 68.0, "⚽", "football pitches"),
    (10_000.0, "🌳", "ha"),
];

/// The largest reference area that `m2` fills at least once. An order of magnitude, so one
/// value: the middle of the site range.
fn area(m2: f64) -> String {
    let (size, icon, label) = AREAS
        .iter()
        .rev()
        .find(|(size, _, _)| m2 >= *size)
        .unwrap_or(&AREAS[0]);
    let n = m2 / size;
    format!("{icon} {} {label}", range(n, n))
}

/// Two significant digits at most; one value when both bounds round the same.
fn range(lo: f64, hi: f64) -> String {
    let fmt = |x: f64| match x {
        x if x >= 10.0 => format!("{x:.0}"),
        x if x >= 1.0 => format!("{x:.1}"),
        x if x >= 0.01 => format!("{x:.2}"),
        x if x > 0.0 => format!("{x:.0e}"),
        _ => "0".to_owned(),
    };
    let (a, b) = (fmt(lo), fmt(hi));
    if a == b { a } else { format!("{a}–{b}") }
}

fn report_json(s: &Summary) -> serde_json::Value {
    let nature = Nature::embedded();
    let sites: Vec<_> = Sites::embedded()
        .sites
        .into_iter()
        .map(|site| json!({ "site": &site, "footprint": nature.at_site(&site, &s.footprint) }))
        .collect();
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
        "co2e_operational_g": s.footprint.co2e_g,
        "co2e_embodied_g": s.footprint.embodied_co2e_g,
        "water_on_site_l": s.footprint.water_on_site_l,
        "water_off_site_l": s.footprint.water_off_site_l,
        "sites": sites,
    })
}

fn print_report(s: &Summary, since: Option<&str>) {
    let nature = Nature::embedded();
    let sites = Sites::embedded().sites;
    let at: Vec<_> = sites
        .iter()
        .map(|site| nature.at_site(site, &s.footprint))
        .collect();
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
    println!(
        "  CO₂e      {}  ({} electricity at 287 g/kWh, {} hardware manufacturing)",
        co2e(s.footprint.total_co2e_g()),
        co2e(s.footprint.co2e_g),
        co2e(s.footprint.embodied_co2e_g),
    );
    println!(
        "  Water     {}  ({} cooling on site, {} to make the electricity)",
        water(s.footprint.water_l()),
        water(s.footprint.water_on_site_l),
        water(s.footprint.water_off_site_l),
    );
    let (lo, hi) = min_max(at.iter().map(|f| f.species_yr.total()));
    let (lo_m2, hi_m2) = min_max(at.iter().map(|f| f.equivalent_m2_yr));
    println!(
        "  Nature    {lo:.1e} to {hi:.1e} species·yr, as if {lo_m2:.0} to {hi_m2:.0} m² of land lost all its species for a year\n"
    );
    println!("  If every request ran at one site:\n");
    println!(
        "  {:<19} {:<5} {:>10} {:>10} {:>8} {:>8} {:>10} {:>8}   Basin stress",
        "Site", "Grid", "CO₂e", "species·yr", "Climate", "Water", "Pollution", "Land"
    );
    for (site, f) in sites.iter().zip(&at) {
        let total = f.species_yr.total();
        let share = |x: f64| {
            if total > 0.0 {
                format!("{:.0}%", 100.0 * x / total)
            } else {
                "-".into()
            }
        };
        println!(
            "  {:<19} {:<5} {:>10} {:>10.1e} {:>8} {:>8} {:>10} {:>8}   {}, {:.0}% withdrawn",
            site.short,
            site.grid.egrid_subregion,
            co2e(f.co2e_kg * 1000.0),
            total,
            share(f.species_yr.climate_change),
            share(f.species_yr.overexploitation),
            share(f.species_yr.pollution),
            share(f.species_yr.ecosystem_use),
            stress_level(&site.aqueduct.bws_label),
            site.aqueduct.bws_raw * 100.0,
        );
    }
    println!();
    println!("  species·yr: species lost, integrated over time (ReCiPe 2016, ecosystem quality).");
    println!(
        "  Basin stress: WRI Aqueduct 4.0 withdrawals over renewable supply, where cooling water is drawn."
    );
    println!("  * Assumed site. Anthropic does not publish how it splits requests between sites.");
    println!("  Order of magnitude only. See METHODOLOGY.md.");
    if s.excluded_requests > 0 {
        println!(
            "  {} requests to non-Claude models are left out.",
            s.excluded_requests
        );
    }
}

/// "High (40-80%)" becomes "High": the share follows in the same cell.
fn stress_level(label: &str) -> &str {
    label.split(" (").next().unwrap_or(label)
}

fn min_max(values: impl Iterator<Item = f64>) -> (f64, f64) {
    values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    })
}

fn print_sites() {
    let sites = Sites::embedded();
    let nature = Nature::embedded();
    println!(
        "\n  {:<19} {:<5} {:>10} {:>9} {:>9} {:>11} {:>8}   Basin stress",
        "Site", "Grid", "CO₂e/MWh", "NOx/MWh", "SO₂/MWh", "Land/MWh", "Pfister"
    );
    for site in &sites.sites {
        println!(
            "  {:<19} {:<5} {:>7.0} kg {:>6.2} kg {:>6.2} kg {:>7.1} m²·yr {:>7.2}×   {}, {:.0}% withdrawn",
            site.short,
            site.grid.egrid_subregion,
            site.grid.co2e_kg_per_mwh,
            site.grid.nox_kg_per_mwh,
            site.grid.so2_kg_per_mwh,
            nature.land_m2_yr_per_mwh(site),
            site.pfister.multiplier,
            stress_level(&site.aqueduct.bws_label),
            site.aqueduct.bws_raw * 100.0,
        );
    }
    println!(
        "\n  Pfister: ecosystem water factor of the watershed over the US value (Pfister et al. 2009)."
    );
    println!();
    for site in &sites.sites {
        println!("  {}: {}. {}", site.short, site.name, site.why);
    }
    println!("\n  Source: {}", sites.source);
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
    } else if g >= 100.0 {
        format!("{g:.0} gCO₂e")
    } else {
        format!("{g:.1} gCO₂e")
    }
}

fn water(l: f64) -> String {
    if l >= 1000.0 {
        format!("{:.1} m³", l / 1000.0)
    } else if l >= 1.0 {
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
            false,
            false,
        );
        assert!(line.starts_with("⚡ "), "{line}");
        assert!(line.contains("gCO₂e ≈ 🚗 "), "{line}");
        assert!(!line.contains('\x1b'), "{line}");
        assert!(line.contains("↳ last turn +"), "{line}");
        assert!(line.contains("% cache"), "{line}");
        assert!(!line.contains('\n'), "{line}");
        assert!(line.contains("≈ 🚗 "), "{line}");
        assert!(line.contains(" PDF·m²·yr ≈ "), "{line}");
    }

    #[test]
    fn statusline_is_empty_without_a_transcript() {
        assert_eq!(statusline(&Factors::embedded(), "{}", true, true), "");
    }

    #[test]
    fn units() {
        assert_eq!(water(0.25), "250 mL");
        assert_eq!(water(12.34), "12.3 L");
        assert_eq!(water(4848.3), "4.8 m³");
        assert_eq!(co2e(1500.0), "1.5 kgCO₂e");
        assert_eq!(tokens(25_300_000_000), "25.3 B");
        assert_eq!(stress_level("High (40-80%)"), "High");
        assert_eq!(car_km(248.5), "1.0 km");
        assert_eq!(range(53.2, 86.4), "53–86");
        assert_eq!(range(0.0, 0.0), "0");
        assert_eq!(area(69.5), "🅿️ 5.6 parking spaces");
        assert_eq!(area(0.0), "📄 0 A4");
        assert_eq!(area(300.0), "🎾 1.2 tennis courts");
        assert_eq!(co2e(422.8), "423 gCO₂e");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }
}
