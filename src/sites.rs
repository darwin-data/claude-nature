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
    pub lat: f64,
    pub lon: f64,
    pub why: String,
    pub aqueduct: Aqueduct,
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
