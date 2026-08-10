use crate::mqtt::{Device_power_state, Power_source};

pub trait Power_provider {
    fn sample(&mut self) -> Device_power_state;
}

pub struct Unavailable_power_provider;

impl Power_provider for Unavailable_power_provider {
    fn sample(&mut self) -> Device_power_state {
        Device_power_state {
            source: Power_source::Unavailable,
            charging: None,
            battery_percent: None,
            battery_mv: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_carrier_never_fabricates_power_measurements() {
        let mut provider = Unavailable_power_provider;
        assert_eq!(
            provider.sample(),
            Device_power_state {
                source: Power_source::Unavailable,
                charging: None,
                battery_percent: None,
                battery_mv: None,
            }
        );
    }
}
