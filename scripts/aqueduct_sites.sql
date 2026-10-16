-- Regenerates the `aqueduct` block of data/sites.json.
-- 1. Download https://files.wri.org/aqueduct/aqueduct-4-0-water-risk-data.zip (261 MB) and unzip it.
-- 2. duckdb -c ".read scripts/aqueduct_sites.sql" with GDB set to the path of Aq40_Y2023D07M05.gdb:
--      sed "s|\$GDB|/path/to/Aq40_Y2023D07M05.gdb|" scripts/aqueduct_sites.sql | duckdb
INSTALL spatial;
LOAD spatial;

WITH s(site, lat, lon) AS (
  VALUES
    ('ashburn-va', 39.0438, -77.4874),
    ('new-carlisle-in', 41.7003, -86.5097),
    ('memphis-tn', 35.0606, -90.1515),
    ('council-bluffs-ia', 41.2210, -95.8608),
    ('the-dalles-or', 45.6321, -121.2015)
),
a AS (
  SELECT * FROM ST_Read('$GDB', layer = 'baseline_annual') WHERE name_0 = 'United States'
)
SELECT s.site, a.pfaf_id, round(a.bws_raw, 4) AS bws_raw, round(a.bws_score, 3) AS bws_score, a.bws_cat, a.bws_label
FROM s JOIN a ON ST_Intersects(a.Shape, ST_Point(s.lon, s.lat))
ORDER BY s.site;
