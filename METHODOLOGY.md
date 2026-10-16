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
7. For each candidate site, apply the grid of the site: CO₂e, NOx, SO₂ and land per MWh.
8. Convert each pressure to species·yr with ReCiPe 2016, and add them.
9. Show the water stress of the basin under each candidate site.

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

## Nature footprint

The nature footprint is one number per site, in species·yr: the number of species lost,
integrated over time. ReCiPe 2016 calls this endpoint "ecosystem quality". Darwin uses the
same endpoint on its platform. Each term maps to a pressure of the IPBES list.

```
species_yr = climate_change + overexploitation + pollution + ecosystem_use

climate_change   = co2e_kg  * (2.8e-9 + 7.7e-14)
overexploitation = water_m3 * (5.2e-9 + 2.9e-12)
pollution        = nox_kg   * (7.71e-8 + 4.65e-8) + so2_kg * 2.37e-7
ecosystem_use    = land_m2_yr * 0.73 * 8.88e-9
```

| Term | Factor | Source in RIVM Report 2016-0104a |
| --- | --- | --- |
| Climate, terrestrial and freshwater | 2.8e-9 and 7.7e-14 species·yr/kg CO₂ | Table 1.5, Hierarchist |
| Water consumed, terrestrial | 5.2e-9 species·yr/m³ | Table S7.3, United States, Hierarchist |
| Water consumed, freshwater | 2.9e-12 species·yr/m³ | Table S7.3, United States |
| Acidification, NOx | 7.71e-8 species·yr/kg | Table S3.2, USA |
| Acidification, SO₂ | 2.37e-7 species·yr/kg | Table S3.2, USA |
| Ozone formation, ecosystems, NOx | 4.65e-8 species·yr/kg | Table S2.3, United States |
| Land occupation, artificial areas | 0.73 annual crop eq | Table 12.1 |
| Land, midpoint to endpoint | 8.88e-9 species·yr per m²·yr annual crop eq | Table 12.2 |

The tool uses the country factors for the United States where the report gives them,
because every candidate site is in the United States. The world averages are higher for
water (1.35e-8) and SO₂ (2.12e-7) and lower for NOx ozone (1.29e-7 world against 4.65e-8 US).

To make the number readable, the report divides it by the mean terrestrial species
density, 1.48e-8 species/m² (RIVM report, section 7.4). The result is an area: the
number of m² that would lose all their species for one year.

### Per site

Source: US EPA eGRID2023 rev2 (June 2025), metric workbook, sheet SRL23
(`scripts/egrid_subregions.sql`). The tool takes the subregion that supplies the site.

| Site | Subregion | CO₂e kg/MWh | NOx kg/MWh | SO₂ kg/MWh |
| --- | --- | ---: | ---: | ---: |
| Ashburn, Virginia | SRVC | 270.49 | 0.1279 | 0.0689 |
| New Carlisle, Indiana | RFCW | 415.52 | 0.1914 | 0.1869 |
| Memphis, Tennessee | SRTV | 409.74 | 0.1742 | 0.1937 |
| Council Bluffs, Iowa | MROW | 420.28 | 0.3225 | 0.4064 |
| The Dalles, Oregon | NWPP | 288.16 | 0.2576 | 0.1216 |

The per-site CO₂e replaces the single 0.287 kg/kWh of claude-carbon. The status line keeps
0.287, because it shows no site.

### Land

Land per MWh is the generation mix of the subregion (eGRID) times the median land-use
intensity of each source (Lovering et al. 2022, PLOS ONE, Table 1, in ha/TWh/y). Coal and
gas include extraction. 1 ha/TWh/y is 0.01 m²·yr per MWh.

| Source | ha/TWh/y | Choice |
| --- | ---: | --- |
| Coal | 1000 | |
| Natural gas | 410 | Footprint. The spacing figure (1900) is left out |
| Oil, other fossil, other | 410 | No figure in the paper. Gas footprint used |
| Nuclear | 7.1 | |
| Hydro | 650 | Single-purpose dams |
| Biomass | 130 | Residue biomass. Dedicated biomass (58 000) is left out |
| Wind | 130 | Footprint. The spacing figure (12 000) is left out, because the land between turbines stays in use |
| Solar | 2000 | Ground-mounted PV |
| Geothermal | 45 | |

All land counts as occupation of "artificial areas" (0.73 annual crop eq). This is a
simplification: a reservoir or a solar farm is not an industrial site for species.
Transformation (the change from natural land) is not counted.

### What the result shows

On the author's 30 days, climate change is 86-90% of the nature footprint at every site,
pollution 4-10%, water 2-3% and land 2-4%. For AI use, the nature footprint follows the
carbon footprint. The local water signal does not appear in the species number, because
the ReCiPe water factor is a country average. That is why the basin stress stays a separate
column.

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

- Training, and the manufacture of the hardware. The mining for chips and servers is a
  resource extraction pressure, and it is absent.
- The land of the data centre itself.
- The on-site gas turbines at Memphis. The grid figures of the TVA subregion do not
  include them.
- Invasive alien species. No characterisation path exists for electricity.
- Human health (ReCiPe has a separate endpoint for it, in DALY).
- Water regionalised at the basin. The next step is the Pfister water stress index per
  basin, which is the method behind the ReCiPe country factor.
- The chat and Cowork tabs of the Claude app. They run on Anthropic's servers and write
  no transcript on your machine.
