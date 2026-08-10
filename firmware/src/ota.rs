use crate::mqtt::Ota_phase;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ota_policy {
    pub phase: Ota_phase,
    pub external_power: bool,
    pub battery_percent: Option<u8>,
}

impl Ota_policy {
    pub fn new(external_power: bool, battery_percent: Option<u8>) -> Self {
        Self {
            phase: Ota_phase::Downloading,
            external_power,
            battery_percent,
        }
    }

    pub fn can_start(&self) -> bool {
        self.external_power || self.battery_percent.is_some_and(|percent| percent >= 30)
    }

    pub fn transition(&mut self, next: Ota_phase) -> Result<(), &'static str> {
        let valid = matches!(
            (&self.phase, &next),
            (Ota_phase::Downloading, Ota_phase::Verifying)
                | (Ota_phase::Verifying, Ota_phase::Rebooting)
                | (Ota_phase::Rebooting, Ota_phase::Healthy)
                | (Ota_phase::Rebooting, Ota_phase::Rolled_back)
                | (_, Ota_phase::Failed)
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
        assert!(!Ota_policy::new(false, Some(29)).can_start());
        assert!(Ota_policy::new(false, Some(30)).can_start());
        assert!(Ota_policy::new(true, None).can_start());
    }

    #[test]
    fn only_allows_ordered_health_transitions() {
        let mut policy = Ota_policy::new(true, None);
        assert_eq!(policy.transition(Ota_phase::Verifying), Ok(()));
        assert_eq!(policy.transition(Ota_phase::Rebooting), Ok(()));
        assert_eq!(policy.transition(Ota_phase::Healthy), Ok(()));
        assert_eq!(
            policy.transition(Ota_phase::Rebooting),
            Err("ota_phase_transition_invalid")
        );
    }
}
