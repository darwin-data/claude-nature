-- Regenerates the `grid` block of data/sites.json.
-- 1. Download https://www.epa.gov/system/files/documents/2025-06/egrid2023_data_metric_rev2.xlsx
-- 2. sed "s|\$XLSX|/path/to/egrid2023_data_metric_rev2.xlsx|" scripts/egrid_subregions.sql | duckdb
-- Rates are kg/MWh. Mix columns are shares of net generation.
INSTALL excel;
LOAD excel;

SELECT
  SUBRGN AS egrid_subregion,
  round(SRC2ERTA::DOUBLE, 2) AS co2e_kg_per_mwh,
  round(SRNOXRTA::DOUBLE, 4) AS nox_kg_per_mwh,
  round(SRSO2RTA::DOUBLE, 4) AS so2_kg_per_mwh,
  SRCLPR AS coal, SROLPR AS oil, SRGSPR AS gas, SRNCPR AS nuclear, SRHYPR AS hydro,
  SRBMPR AS biomass, SRWIPR AS wind, SRSOPR AS solar, SRGTPR AS geothermal,
  SROFPR AS other_fossil, SROPPR AS other
FROM read_xlsx('$XLSX', sheet = 'SRL23', range = 'A2:JZ30', header = true, all_varchar = true)
WHERE SUBRGN IN ('SRVC', 'RFCW', 'SRTV', 'MROW', 'NWPP')
ORDER BY SUBRGN;
