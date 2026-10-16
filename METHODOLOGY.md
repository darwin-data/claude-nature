# Methodology

## Chain

1. Read every assistant message in the Claude Code transcripts (`~/.claude/projects/**/*.jsonl`,
   subagent transcripts included).
2. Deduplicate by `(message.id, requestId)` and keep the last occurrence. Streaming writes
   one message several times with a growing `output_tokens`, and a resumed session replays
   earlier messages. claude-carbon and ccusage apply the same rule.
3. Find the model family from `message.model`. A model string without `claude`
   (a local model, `<synthetic>`) is left out of every total.
4. Compute CO₂e with the claude-carbon formula and factors.
5. Divide CO₂e by the grid intensity the factors come from. This gives the energy back.
6. Multiply energy by the water use factors.
7. Show the water stress of the basin under each candidate site.

## Energy and climate

All values come from `METHODOLOGY.md` of
[claude-carbon](https://github.com/gwittebolle/claude-carbon), read on 2026-10-16. That file
derives them from Jegham et al. 2025, "How Hungry is AI?", arXiv:2505.09598 v6.

```
co2e_g = ( (input + cache_write) * input_factor
         + cache_read * input_factor * cache_read_factor
         + output * output_factor ) / 1_000_000

energy_wh = co2e_g / 0.287
```

| Family | Input (gCO₂e/Mtok) | Output (gCO₂e/Mtok) |
| --- | ---: | ---: |
| Fable | 156 | 3304 |
| Opus | 78 | 1652 |
| Sonnet | 39 | 826 |
| Haiku | 10 | 207 |

`cache_read_factor` is 0.08. The grid intensity is 0.287 kgCO₂e/kWh (AWS, location-based),
and the factors include a PUE of 1.14. Only Sonnet is fitted on Jegham's data. The other
families are scaled from it. claude-carbon gives a plausible range of 0.7x to 3x around
its totals, and more for long contexts. The same range applies here.

The values live in `data/factors.json`. Keep them in step with claude-carbon.

## Water

| Factor | Value | What it covers |
| --- | ---: | --- |
| On site | 0.18 L/kWh | Water that cooling consumes at the data centre |
| Off site | 5.11 L/kWh | Water that power plants consume to make the electricity |

Both values come from Jegham et al. 2025, as claude-carbon lists them. claude-carbon
carries them but does not use them. One value covers every site. Real cooling water
changes a lot with the climate and the cooling design of each site: an air-cooled site in
Oregon in winter uses far less than an evaporative site in summer.

## Water stress

Source: WRI Aqueduct 4.0, baseline annual layer
(`Aqueduct40_waterrisk_download_Y2023M07D05`, CC BY 4.0). For each site, the tool takes
the basin polygon that holds the site point (`scripts/aqueduct_sites.sql`).

- `bws_raw` is gross water withdrawals divided by available renewable surface and
  ground water, for the basin and the year.
- `bws_cat` and `bws_label` are the Aqueduct categories: Low (<10%), Low-medium (10-20%),
  Medium-high (20-40%), High (40-80%), Extremely high (>80%).

What the table means: the on-site litres of the report are drawn at the site that serves
the request. In a High basin, those litres compete with a supply that is already 40-80%
withdrawn.

What the table does not mean:

- It does not tell you where your requests ran. Anthropic does not publish its split
  between sites. Each row is a separate case.
- It does not apply to the off-site litres. Power plants sit on the grid, often in another
  basin than the data centre.
- It is not a characterisation factor. The tool does not multiply litres by stress to give
  one number, because a weighted index (AWARE, for example) needs a choice of method that
  this version does not make.

## Candidate sites

| Site | Why it is a candidate |
| --- | --- |
| AWS us-east-1, Ashburn, Virginia | The grid intensity of the factors comes from this AWS region |
| AWS Project Rainier, New Carlisle, Indiana | Trainium2 cluster that Amazon built for Anthropic (Amazon, October 2025) |
| Colossus 1, Memphis, Tennessee | Site Anthropic took in May 2026, as claude-carbon reports it |
| Google Cloud us-central1, Council Bluffs, Iowa | Anthropic uses Google Cloud TPUs. The region is an assumption |
| Google Cloud us-west1, The Dalles, Oregon | Same: a Google Cloud TPU region, assumed |

Coordinates are at the town or the campus. A basin is tens to thousands of square
kilometres, so this precision is enough for a basin lookup. The values live in
`data/sites.json`.

## Not counted

- Training, and the manufacture of the hardware.
- Pollution and land use at each site (next versions).
- The chat and Cowork tabs of the Claude app. They run on Anthropic's servers and write
  no transcript on your machine.
