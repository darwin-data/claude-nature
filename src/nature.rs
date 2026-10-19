use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::footprint::Footprint;
use crate::sites::Site;

/// Characterisation factors, from `data/nature.json`.
#[derive(Debug, Deserialize)]
pub struct Nature {
    pub recipe: Recipe,
    pub land_use_intensity: LandUseIntensity,
}

/// ReCiPe 2016 v1.1, Hierarchist, endpoint "ecosystem quality".
#[derive(Debug, Deserialize)]
pub struct Recipe {
    pub climate_terrestrial_species_yr_per_kg_co2e: f64,
    pub climate_freshwater_species_yr_per_kg_co2e: f64,
    pub water_terrestrial_species_yr_per_m3: f64,
    pub water_freshwater_species_yr_per_m3: f64,
    pub acidification_species_yr_per_kg_nox: f64,
    pub acidification_species_yr_per_kg_so2: f64,
    pub ozone_species_yr_per_kg_nox: f64,
    pub land_occupation_annual_crop_eq_per_m2_yr: f64,
    pub land_species_yr_per_m2_yr_annual_crop_eq: f64,
    pub terrestrial_species_per_m2: f64,
}

/// Lovering et al. 2022, Table 1, per generation source.
#[derive(Debug, Deserialize)]
pub struct LandUseIntensity {
    pub ha_per_twh_yr: BTreeMap<String, f64>,
}

/// Loss of species integrated over time, split by IPBES pressure.
/// Invasive alien species has no characterisation path for electricity and stays out.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize)]
pub struct Species {
    pub climate_change: f64,
    pub overexploitation: f64,
    pub pollution: f64,
    pub ecosystem_use: f64,
}

impl Species {
    pub fn total(&self) -> f64 {
        self.climate_change + self.overexploitation + self.pollution + self.ecosystem_use
    }
}

/// What the usage weighs if every request ran at one site.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SiteFootprint {
    pub site_id: String,
    pub co2e_kg: f64,
    pub water_m3: f64,
    pub nox_kg: f64,
    pub so2_kg: f64,
    pub land_m2_yr: f64,
    pub species_yr: Species,
    /// `species_yr` over the mean terrestrial species density: the area that
    /// would lose all its species for one year.
    pub equivalent_m2_yr: f64,
}

impl Nature {
    pub fn embedded() -> Self {
        serde_json::from_str(include_str!("../data/nature.json"))
            .expect("data/nature.json is checked by the test suite")
    }

    /// Land occupied per MWh of the site's grid, in m²·yr.
    /// 1 ha/TWh/y is 10 000 m² for 1 000 000 MWh, so 0.01 m²·yr per MWh.
    pub fn land_m2_yr_per_mwh(&self, site: &Site) -> f64 {
        site.grid
            .mix
            .iter()
            .map(|(source, share)| {
                share
                    * self
                        .land_use_intensity
                        .ha_per_twh_yr
                        .get(source)
                        .copied()
                        .unwrap_or(0.0)
                    * 0.01
            })
            .sum()
    }

    pub fn at_site(&self, site: &Site, usage: &Footprint) -> SiteFootprint {
        let r = &self.recipe;
        let mwh = usage.energy_wh / 1_000_000.0;
        let co2e_kg = mwh * site.grid.co2e_kg_per_mwh + usage.embodied_co2e_g / 1000.0;
        let nox_kg = mwh * site.grid.nox_kg_per_mwh;
        let so2_kg = mwh * site.grid.so2_kg_per_mwh;
        let water_m3 = usage.water_l() / 1000.0;
        let land_m2_yr = mwh * self.land_m2_yr_per_mwh(site);
        let species_yr = Species {
            climate_change: co2e_kg
                * (r.climate_terrestrial_species_yr_per_kg_co2e
                    + r.climate_freshwater_species_yr_per_kg_co2e),
            overexploitation: water_m3
                * (r.water_terrestrial_species_yr_per_m3 + r.water_freshwater_species_yr_per_m3),
            pollution: nox_kg
                * (r.acidification_species_yr_per_kg_nox + r.ozone_species_yr_per_kg_nox)
                + so2_kg * r.acidification_species_yr_per_kg_so2,
            ecosystem_use: land_m2_yr
                * r.land_occupation_annual_crop_eq_per_m2_yr
                * r.land_species_yr_per_m2_yr_annual_crop_eq,
        };
        SiteFootprint {
            site_id: site.id.clone(),
            co2e_kg,
            water_m3,
            nox_kg,
            so2_kg,
            land_m2_yr,
            equivalent_m2_yr: species_yr.total() / r.terrestrial_species_per_m2,
            species_yr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sites::Sites;

    fn one_mwh() -> Footprint {
        Footprint {
            energy_wh: 1_000_000.0,
            co2e_g: 287_000.0,
            embodied_co2e_g: 40_000.0,
            water_on_site_l: 180.0,
            water_off_site_l: 5110.0,
        }
    }

    #[test]
    fn every_grid_source_has_a_land_use_intensity() {
        let nature = Nature::embedded();
        for site in Sites::embedded().sites {
            let share: f64 = site.grid.mix.values().sum();
            assert!(
                (share - 1.0).abs() < 0.01,
                "{} mix sums to {share}",
                site.id
            );
            for source in site.grid.mix.keys() {
                assert!(
                    nature.land_use_intensity.ha_per_twh_yr.contains_key(source),
                    "{source} has no land-use intensity"
                );
            }
        }
    }

    #[test]
    fn one_mwh_in_virginia() {
        let nature = Nature::embedded();
        let sites = Sites::embedded();
        let site = sites.sites.iter().find(|s| s.id == "ashburn-va").unwrap();
        let fp = nature.at_site(site, &one_mwh());
        assert!((fp.co2e_kg - (270.49 + 40.0)).abs() < 1e-9);
        assert!((fp.water_m3 - 5.29).abs() < 1e-9);
        assert!((fp.species_yr.climate_change - 310.49 * (2.8e-9 + 7.7e-14)).abs() < 1e-18);
        assert!((fp.species_yr.overexploitation - 5.29 * (5.2e-9 + 2.9e-12)).abs() < 1e-18);
        assert!(
            (fp.species_yr.pollution - (0.1279 * (7.71e-8 + 4.65e-8) + 0.0689 * 2.37e-7)).abs()
                < 1e-18
        );
        let land = 0.01
            * (0.105 * 1000.0
                + 0.001 * 410.0
                + 0.393 * 410.0
                + 0.399 * 7.1
                + 0.014 * 650.0
                + 0.021 * 130.0
                + 0.005 * 130.0
                + 0.06 * 2000.0
                + 0.002 * 410.0
                + 0.001 * 410.0);
        assert!((fp.land_m2_yr - land).abs() < 1e-9);
        assert!((fp.equivalent_m2_yr - fp.species_yr.total() / 1.48e-8).abs() < 1e-9);
    }
}
