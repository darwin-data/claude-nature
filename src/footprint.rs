use std::collections::BTreeMap;
use std::ops::AddAssign;

use serde::Deserialize;

/// Energy and water parameters, from `data/factors.json`.
#[derive(Debug, Deserialize)]
pub struct Factors {
    pub grid_g_co2e_per_wh: f64,
    pub water_on_site_l_per_kwh: f64,
    pub water_off_site_l_per_kwh: f64,
    pub cache_read_factor: f64,
    pub embodied_g_co2e_per_kwh: f64,
    pub families: BTreeMap<String, FamilyFactors>,
}

#[derive(Debug, Deserialize)]
pub struct FamilyFactors {
    pub input_g_per_mtok: f64,
    pub output_g_per_mtok: f64,
}

impl Factors {
    pub fn embedded() -> Self {
        serde_json::from_str(include_str!("../data/factors.json"))
            .expect("data/factors.json is checked by the test suite")
    }

    /// Resolves a `message.model` string to a family key of `data/factors.json`.
    /// A model that names no family (a local model behind `ANTHROPIC_BASE_URL`,
    /// `<synthetic>`) returns `None` and is left out of every total.
    pub fn family_of(&self, model: &str) -> Option<&str> {
        let model = model.to_ascii_lowercase();
        if !model.contains("claude") {
            return None;
        }
        self.families
            .keys()
            .find(|family| model.contains(family.as_str()))
            .map(String::as_str)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, serde::Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    pub output: u64,
}

impl Tokens {
    pub fn total(&self) -> u64 {
        self.input + self.cache_write + self.cache_read + self.output
    }

    /// Share of the prompt side read from the prompt cache, the cheapest way to send tokens.
    pub fn cache_share(&self) -> Option<f64> {
        let prompt = self.input + self.cache_write + self.cache_read;
        (prompt > 0).then(|| self.cache_read as f64 / prompt as f64)
    }
}

impl AddAssign for Tokens {
    fn add_assign(&mut self, other: Self) {
        self.input += other.input;
        self.cache_write += other.cache_write;
        self.cache_read += other.cache_read;
        self.output += other.output;
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Footprint {
    pub energy_wh: f64,
    /// Operational: the electricity, at the grid intensity of the energy factors.
    pub co2e_g: f64,
    /// Manufacturing of the hardware, per kWh used. It does not depend on the site.
    pub embodied_co2e_g: f64,
    pub water_on_site_l: f64,
    pub water_off_site_l: f64,
}

impl Footprint {
    pub fn water_l(&self) -> f64 {
        self.water_on_site_l + self.water_off_site_l
    }

    pub fn total_co2e_g(&self) -> f64 {
        self.co2e_g + self.embodied_co2e_g
    }
}

impl AddAssign for Footprint {
    fn add_assign(&mut self, other: Self) {
        self.energy_wh += other.energy_wh;
        self.co2e_g += other.co2e_g;
        self.embodied_co2e_g += other.embodied_co2e_g;
        self.water_on_site_l += other.water_on_site_l;
        self.water_off_site_l += other.water_off_site_l;
    }
}

/// The claude-carbon formula, carried from CO2e back to the energy it starts from,
/// then to water. See METHODOLOGY.md.
pub fn footprint(factors: &Factors, family: &str, tokens: Tokens) -> Footprint {
    let Some(f) = factors.families.get(family) else {
        return Footprint::default();
    };
    let co2e_g = ((tokens.input + tokens.cache_write) as f64 * f.input_g_per_mtok
        + tokens.cache_read as f64 * f.input_g_per_mtok * factors.cache_read_factor
        + tokens.output as f64 * f.output_g_per_mtok)
        / 1_000_000.0;
    let energy_wh = co2e_g / factors.grid_g_co2e_per_wh;
    let kwh = energy_wh / 1000.0;
    Footprint {
        energy_wh,
        co2e_g,
        embodied_co2e_g: kwh * factors.embodied_g_co2e_per_kwh,
        water_on_site_l: kwh * factors.water_on_site_l_per_kwh,
        water_off_site_l: kwh * factors.water_off_site_l_per_kwh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_resolution() {
        let f = Factors::embedded();
        assert_eq!(f.family_of("claude-opus-5-5"), Some("opus"));
        assert_eq!(f.family_of("claude-sonnet-4-6"), Some("sonnet"));
        assert_eq!(f.family_of("claude-haiku-5-5"), Some("haiku"));
        assert_eq!(f.family_of("claude-fable-5-1"), Some("fable"));
        assert_eq!(f.family_of("<synthetic>"), None);
        assert_eq!(f.family_of("qwen3-coder"), None);
    }

    #[test]
    fn one_million_sonnet_output_tokens() {
        let f = Factors::embedded();
        let fp = footprint(
            &f,
            "sonnet",
            Tokens {
                output: 1_000_000,
                ..Tokens::default()
            },
        );
        assert!((fp.co2e_g - 826.0).abs() < 1e-9);
        assert!((fp.embodied_co2e_g - 826.0 / 287.0 * 40.0).abs() < 1e-9);
        assert!((fp.energy_wh - 826.0 / 0.287).abs() < 1e-6);
        let kwh = 826.0 / 0.287 / 1000.0;
        assert!((fp.water_l() - kwh * (0.18 + 5.11)).abs() < 1e-9);
    }

    #[test]
    fn cache_reads_count_at_a_fraction_of_input() {
        let f = Factors::embedded();
        let fp = footprint(
            &f,
            "opus",
            Tokens {
                cache_read: 1_000_000,
                ..Tokens::default()
            },
        );
        assert!((fp.co2e_g - 78.0 * 0.08).abs() < 1e-9);
    }
}
