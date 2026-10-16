# claude-nature

The nature footprint of your Claude Code sessions: energy, climate and water. The tool
reads each litre against the water stress of the basin where a data centre draws it.

```
claude-nature · all transcripts on disk · 656 sessions

  Tokens    20.6 B (97% cache reads)
  Energy    916.5 kWh
  Climate   263.0 kgCO₂e
  Water     4848.3 L  (165.0 L cooling on site, 4683.3 L to make the electricity)

  A litre does not weigh the same in every basin. Cooling water is drawn at the site:

  Site                                                       Basin stress      Withdrawals / supply
  AWS us-east-1, Ashburn, Virginia                           Low (<10%)                          6%
  AWS Project Rainier, New Carlisle, Indiana                 High (40-80%)                      71%
  Colossus 1, Memphis, Tennessee                             Low (<10%)                          0%
  Google Cloud us-central1, Council Bluffs, Iowa (assumed)   Low (<10%)                          5%
  Google Cloud us-west1, The Dalles, Oregon (assumed)        Low (<10%)                          1%
```

## Why water, and why the basin

A carbon figure is the same wherever the tonne is emitted. A water figure is not. One
litre drawn from a basin that uses 6% of its renewable supply has little effect. The same
litre drawn from a basin that uses 71% of it competes with farms, towns and rivers. The
TNFD guidance for the Technology and Communications sector asks data centres to report
exactly this: the local water stress score, with WRI Aqueduct as the reference.

claude-nature keeps the energy model of
[claude-carbon](https://github.com/gwittebolle/claude-carbon) and adds the nature side.

| Pressure | Status |
| --- | --- |
| Climate change (CO₂e) | Yes, claude-carbon factors |
| Water use, in litres | Yes, on site and off site |
| Water use, against basin stress | Yes, WRI Aqueduct 4.0 at each candidate site |
| Pollution (NOx and particles from on-site gas turbines) | Next |
| Land use at the site | Next |
| Resource extraction (hardware) | Next |

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
| `claude-nature sites` | The candidate sites, why each one is there, and its basin |

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

MIT licence.
