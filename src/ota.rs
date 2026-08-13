use crate::mqtt::OtaPhase;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtaPolicy {
    pub phase: OtaPhase,
    pub external_power: bool,
    pub battery_percent: Option<u8>,
}

impl OtaPolicy {
    pub fn new(external_power: bool, battery_percent: Option<u8>) -> Self {
        Self {
            phase: OtaPhase::Downloading,
            external_power,
            battery_percent,
        }
    }

    pub fn can_start(&self) -> bool {
        self.external_power || self.battery_percent.is_some_and(|percent| percent >= 30)
    }

    pub fn transition(&mut self, next: OtaPhase) -> Result<(), &'static str> {
        let valid = matches!(
            (&self.phase, &next),
            (OtaPhase::Downloading, OtaPhase::Verifying)
                | (OtaPhase::Verifying, OtaPhase::Rebooting)
                | (OtaPhase::Rebooting, OtaPhase::Healthy)
                | (OtaPhase::Rebooting, OtaPhase::RolledBack)
                | (_, OtaPhase::Failed)
        );
        if !valid {
            return Err("ota_phase_transition_invalid");
        }
        self.phase = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defers_battery_update_below_threshold() {
        assert!(!OtaPolicy::new(false, Some(29)).can_start());
        assert!(OtaPolicy::new(false, Some(30)).can_start());
        assert!(OtaPolicy::new(true, None).can_start());
    }

    #[test]
    fn only_allows_ordered_health_transitions() {
        let mut policy = OtaPolicy::new(true, None);
        assert_eq!(policy.transition(OtaPhase::Verifying), Ok(()));
        assert_eq!(policy.transition(OtaPhase::Rebooting), Ok(()));
        assert_eq!(policy.transition(OtaPhase::Healthy), Ok(()));
        assert_eq!(
            policy.transition(OtaPhase::Rebooting),
            Err("ota_phase_transition_invalid")
        );
    }
}
