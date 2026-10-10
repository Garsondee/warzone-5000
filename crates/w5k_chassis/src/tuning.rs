//! Shared numerical constants of the chassis, loaded from `content/physics/chassis/tuning.ron` (every number a `Param`).

use serde::Deserialize;
use w5k_contract::Param;

use crate::suspension::SuspensionTuning;
use crate::tyre::TyreTuning;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChassisTuning {
    pub friction_smoothing_m_s: Param,
    pub min_gas_volume_frac: Param,
    pub rolling_fade_speed_m_s: Param,
    pub slip_damping_time_s: Param,
}

impl ChassisTuning {
    pub fn from_ron(text: &str) -> Result<ChassisTuning, ron::error::SpannedError> {
        ron::from_str(text)
    }

    pub fn tyre(&self) -> TyreTuning {
        TyreTuning {
            rolling_fade_speed_m_s: self.rolling_fade_speed_m_s.v,
            slip_damping_time_s: self.slip_damping_time_s.v,
        }
    }

    pub fn suspension(&self) -> SuspensionTuning {
        SuspensionTuning {
            friction_smoothing_m_s: self.friction_smoothing_m_s.v,
            min_gas_volume_frac: self.min_gas_volume_frac.v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_tuning_file_loads_and_every_value_is_inside_its_band() {
        let t = ChassisTuning::from_ron(include_str!("../../../content/physics/chassis/tuning.ron"))
            .expect("tuning.ron parses");
        for p in [&t.friction_smoothing_m_s, &t.min_gas_volume_frac, &t.rolling_fade_speed_m_s, &t.slip_damping_time_s]
        {
            let (lo, hi) = p.band();
            assert!(p.v >= lo && p.v <= hi && !p.src.is_empty());
        }
    }
}
