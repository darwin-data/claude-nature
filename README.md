# claude-nature

The nature footprint of your Claude Code sessions. The tool turns your token use into
energy, then into the pressures that drive nature loss at each data centre: climate,
water, pollution and land. It adds them up in one unit, species·yr, with ReCiPe 2016.

```
claude-nature · all transcripts on disk · 644 sessions

  Tokens    20.8 B (97% cache reads)
  Energy    922.3 kWh
  Water     4.9 m³  (166.0 L cooling on site, 4.7 m³ to make the electricity)
  Nature    7.8e-7 to 1.3e-6 species·yr, as if 53 to 86 m² of land lost all its species for a year

  If every request ran at one site:

  Site                Grid        CO₂e species·yr  Climate    Water  Pollution     Land   Basin stress
  Ashburn VA          SRVC  249.5 kgCO₂e     7.8e-7      90%       3%         4%       3%   Low, 6% withdrawn
  New Carlisle IN     RFCW  383.2 kgCO₂e     1.2e-6      90%       2%         5%       2%   High, 71% withdrawn
  Memphis TN          SRTV  377.9 kgCO₂e     1.2e-6      90%       2%         5%       2%   Low, 0% withdrawn
  Council Bluffs IA*  MROW  387.6 kgCO₂e     1.3e-6      86%       2%        10%       2%   Low, 5% withdrawn
  The Dalles OR*      NWPP  265.8 kgCO₂e     8.6e-7      86%       3%         6%       4%   Low, 1% withdrawn

  species·yr: species lost, integrated over time (ReCiPe 2016, ecosystem quality).
  Basin stress: WRI Aqueduct 4.0 withdrawals over renewable supply, where cooling water is drawn.
  * Assumed site. Anthropic does not publish how it splits requests between sites.
  Order of magnitude only. See METHODOLOGY.md.
  79 requests to non-Claude models are left out.
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
| Climate change | CO₂e of the grid that supplies each site | US EPA eGRID2023 |
| Overexploitation | Water for cooling and for power plants | Jegham et al. 2025 |
| Pollution | NOx and SO₂ of the grid: acidification and ozone | US EPA eGRID2023 |
| Ecosystem use | Land of the generation mix, extraction included | Lovering et al. 2022 |
| Invasive alien species | Not counted: no path for electricity | |

ReCiPe 2016 converts each pressure into species·yr, and the report divides the total by the
mean species density to give an area: the m² that would lose all their species for a
year. The answer surprised us: at every site, climate is 86-90% of the total. For AI use,
the nature footprint follows the carbon footprint. The local water signal stays in its own
column, because no species number captures it yet.

The five pressures are the drivers of nature loss that IPBES names. Darwin
([darwindata.io](https://darwindata.io)) measures them for company supply chains, and
this tool applies the same reading to one developer's AI use.

## Install

You need Rust (`rustup`, then `cargo`).

```bash
cargo install --git https://github.com/darwin-data/claude-nature
claude-nature report
```

### Status line

Add this to `~/.claude/settings.json`. If you already have a status line, call
`claude-nature statusline` from your own script and append its output.

```json
{
  "statusLine": { "type": "command", "command": "claude-nature statusline" }
}
```

It prints the current session, subagents included: `💧 3.8 L · 204.4 gCO₂e`.

### Slash command

Install the plugin to get `/nature-report` in Claude Code:

```
/plugin marketplace add darwin-data/claude-nature
/plugin install claude-nature@claude-nature
```

## Commands

| Command | Output |
| --- | --- |
| `claude-nature report [--since YYYY-MM-DD] [--project TEXT] [--json]` | Totals over the transcripts on disk |
| `claude-nature statusline` | One line for the status line (reads its JSON on stdin) |
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
