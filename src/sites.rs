use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Candidate data centres for Claude inference, from `data/sites.json`.
#[derive(Debug, Deserialize)]
pub struct Sites {
    pub source: String,
    pub sites: Vec<Site>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Site {
    pub id: String,
    pub name: String,
    /// Table label. A trailing `*` marks an assumed site.
    pub short: String,
    pub lat: f64,
    pub lon: f64,
    pub why: String,
    pub aqueduct: Aqueduct,
    pub pfister: Pfister,
    pub grid: Grid,
}

/// Pfister et al. 2009 watershed under the site. `cf_eq` is the terrestrial ecosystem
/// factor (PDF·m²·yr/m³) that ReCiPe 2016 builds its water factor on; `multiplier` is
/// `cf_eq` over the US value, 0.3514. `wsi` is shown, not used: in Pfister 2009 it only
/// drives human health.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pfister {
    pub cf_eq_m2_yr_per_m3: f64,
    pub wsi: f64,
    pub multiplier: f64,
}

/// US EPA eGRID2023 subregion that supplies the site. Rates are output emission
/// rates in kg/MWh. `mix` is the share of net generation per source.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Grid {
    pub egrid_subregion: String,
    pub co2e_kg_per_mwh: f64,
    pub nox_kg_per_mwh: f64,
    pub so2_kg_per_mwh: f64,
    pub mix: BTreeMap<String, f64>,
}

/// WRI Aqueduct 4.0 baseline water stress of the basin under the site.
/// `bws_raw` is gross withdrawals over available renewable supply.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Aqueduct {
    pub pfaf_id: u64,
    pub bws_raw: f64,
    pub bws_score: f64,
    pub bws_cat: i8,
    pub bws_label: String,
}

impl Sites {
    pub fn embedded() -> Self {
        serde_json::from_str(include_str!("../data/sites.json"))
            .expect("data/sites.json is checked by the test suite")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_site_has_a_basin() {
        let sites = Sites::embedded();
        assert!(!sites.sites.is_empty());
        for site in &sites.sites {
            assert!(site.aqueduct.pfaf_id > 0, "{}", site.id);
            assert!((0..=4).contains(&site.aqueduct.bws_cat), "{}", site.id);
        }
    }
}
