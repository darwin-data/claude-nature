# claude-nature

The nature footprint of your Claude Code sessions. The tool turns your token use into
energy, then into the pressures that drive nature loss at each data centre: climate,
water, pollution and land. It adds them up in one unit, species·yr, with ReCiPe 2016.

```
claude-nature · all transcripts on disk · 597 sessions

  Tokens    18.2 B (97% cache reads)
  Energy    819.3 kWh
  CO₂e      267.9 kgCO₂e  (235.2 kgCO₂e electricity at 287 g/kWh, 32.8 kgCO₂e hardware manufacturing)
  Water     4.3 m³  (147.5 L cooling on site, 4.2 m³ to make the electricity)
  Nature    7.8e-7 to 1.2e-6 species·yr, as if 53 to 82 m² of land lost all its species for a year

  If every request ran at one site:

  Site                Grid        CO₂e species·yr  Climate    Water  Pollution     Land   Basin stress
  Ashburn VA          SRVC  254.4 kgCO₂e     7.8e-7      91%       3%         3%       3%   Low, 6% withdrawn
  New Carlisle IN     RFCW  373.2 kgCO₂e     1.1e-6      91%       2%         5%       2%   High, 71% withdrawn
  Memphis TN          SRTV  368.5 kgCO₂e     1.1e-6      91%       2%         5%       2%   Low, 0% withdrawn
  Council Bluffs IA*  MROW  377.1 kgCO₂e     1.2e-6      87%       2%         9%       2%   Low, 5% withdrawn
  The Dalles OR*      NWPP  268.9 kgCO₂e     8.6e-7      88%       3%         6%       4%   Low, 1% withdrawn

  species·yr: species lost, integrated over time (ReCiPe 2016, ecosystem quality).
  Basin stress: WRI Aqueduct 4.0 withdrawals over renewable supply, where cooling water is drawn.
  * Assumed site. Anthropic does not publish how it splits requests between sites.
  Order of magnitude only. See METHODOLOGY.md.
  80 requests to non-Claude models are left out.
```

## Why water, and why the basin

A carbon figure is the same wherever the tonne is emitted. A water figure is not. One
litre drawn from a basin that uses 6% of its renewable supply has little effect. The same
litre drawn from a basin that uses 71% of it competes with farms, towns and rivers. The
TNFD guidance for the Technology and Communications sector asks data centres to report
exactly this: the local water stress score, with WRI Aqueduct as the reference.

claude-nature keeps the energy model of
[claude-carbon](https://github.com/gwittebolle/claude-carbon) and adds the nature side.

| IPBES pressure | What the tool counts | Source |
| --- | --- | --- |
| Climate change | CO₂e of the grid that supplies each site, plus hardware manufacturing | US EPA eGRID2023, Schneider et al. 2025 |
| Overexploitation | Water for cooling and for power plants, cooling at the watershed of the site | Jegham et al. 2025, Pfister et al. 2009 |
| Pollution | NOx and SO₂ of the grid: acidification and ozone | US EPA eGRID2023 |
| Ecosystem use | Land of the generation mix, extraction included | Lovering et al. 2022 |
| Invasive alien species | Not counted: no path for electricity | |

ReCiPe 2016 converts each pressure into species·yr, and the report divides the total by the
mean species density to give an area: the m² that would lose all their species for a
year. The answer surprised us: at every site, climate is 87-91% of the total. For AI use,
the nature footprint follows the carbon footprint. The local water signal stays in its own
column, because no species number captures it yet.

The five pressures are the drivers of nature loss that IPBES names. Darwin
([darwindata.io](https://darwindata.io)) measures them for company supply chains, and
this tool applies the same reading to one developer's AI use.

## Install

In Claude Code, the plugin does everything: it installs the binary and adds the status line.

```
/plugin marketplace add darwin-data/claude-nature
/plugin install nature-footprint@darwin-nature
/nature-footprint:setup
```

From a terminal, the same in two commands. No Rust needed: the script downloads the release
binary for macOS or Linux and checks its SHA-256.

```bash
curl -fsSL https://raw.githubusercontent.com/darwin-data/claude-nature/main/install.sh | sh
claude-nature install-statusline
```

With Rust: `cargo install --git https://github.com/darwin-data/claude-nature`.

### Status line

`claude-nature install-statusline` sets the status line in `~/.claude/settings.json` and
copies the old file to `settings.json.bak-claude-nature`. If you already have a status line,
it stays on the first line: claude-nature runs it through `--wrap`.

```
⚡ 2.3 kWh  💧 12.3 L  🏭 670 gCO₂e ≈ 🚗 2.7 km  🌱 0.13–0.22 PDF·m²·yr ≈ 📄 2.8 A4  ↳ last turn +199 gCO₂e · ♻️ 97% cache
📅 today ⚡ 10.5 kWh  💧 55.4 L  🏭 3.0 kgCO₂e ≈ 🚗 12 km  🌱 0.60–0.97 PDF·m²·yr ≈ 📄 13 A4  17 sessions
```

- The first line is the current session, subagents included. `last turn` counts what was
  sent since your last prompt. `cache` is the share of the prompt read from the prompt
  cache: the higher, the cheaper each request.
- The second line is every session since local midnight. `--no-today` removes it.
- The PDF·m²·yr range runs over the candidate sites, as in the report. The two `≈` give an
  order of magnitude: kilometres in a typical petrol car, and a familiar area for the
  middle of the range. METHODOLOGY.md gives the sources.
- Each metric has its own colour, and the equivalents are grey. Set `NO_COLOR` to get plain
  text.
- A day of transcripts is about 100 MB. claude-nature keeps a scan of each file in
  `~/.cache/claude-nature` and re-reads only the files that changed: about 65 ms per call.

### Slash command

The plugin adds `/nature-footprint:nature-report` (the full report) and
`/nature-footprint:setup` (install and status line).

## Commands

| Command | Output |
| --- | --- |
| `claude-nature report [--since YYYY-MM-DD] [--project TEXT] [--json]` | Totals over the transcripts on disk |
| `claude-nature statusline [--wrap COMMAND] [--no-today]` | The status line: session, then today (reads its JSON on stdin) |
| `claude-nature install-statusline` | Sets the status line in Claude Code settings, and keeps the existing one |
| `claude-nature sites` | The candidate sites: grid, emissions and land per MWh, basin, and why each one is there |

Claude Code deletes transcripts after about 30 days. The report covers what is still on
disk. Nothing leaves your machine.

## Limits

Order of magnitude only. Do not use these numbers for regulatory reporting. Anthropic
does not publish where each request runs, so the tool shows each site as a separate
case. See [METHODOLOGY.md](METHODOLOGY.md) for every factor and its source.

## Credits

- Energy and CO₂e factors: [claude-carbon](https://github.com/gwittebolle/claude-carbon)
  by Gaëtan Wittebolle (MIT), from Jegham et al. 2025.
- Water stress: [WRI Aqueduct 4.0](https://www.wri.org/aqueduct) (CC BY 4.0).
- Grid emissions and mix: [US EPA eGRID2023](https://www.epa.gov/egrid) (public domain).
- Land-use intensity: Lovering et al. 2022, PLOS ONE 17(7): e0270155.
- Characterisation: ReCiPe 2016 v1.1, RIVM Report 2016-0104a.

MIT licence.
