use crate::mqtt::{Device_power_state, Power_source};

pub trait Power_provider {
    fn sample(&mut self) -> Device_power_state;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Power_measurement {
    pub vbus_present: bool,
    pub battery_charging: bool,
    pub battery_supplying: bool,
    pub battery_percent: Option<u8>,
    pub battery_mv: Option<u16>,
}

pub fn classify_measurement(measurement: Power_measurement) -> Device_power_state {
    let source = match (measurement.vbus_present, measurement.battery_supplying) {
        (true, true) => Power_source::Usb_and_battery,
        (true, false) => Power_source::Usb,
        (false, _) => Power_source::Battery,
    };

    Device_power_state {
        source,
        charging: measurement
            .vbus_present
            .then_some(measurement.battery_charging),
        battery_percent: measurement.battery_percent,
        battery_mv: measurement.battery_mv,
    }
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
    fn usb_powers_load_while_cell_charges() {
        assert_eq!(
            classify_measurement(Power_measurement {
                vbus_present: true,
                battery_charging: true,
                battery_supplying: false,
                battery_percent: Some(72),
                battery_mv: Some(3_940),
            }),
            Device_power_state {
                source: Power_source::Usb,
                charging: Some(true),
                battery_percent: Some(72),
                battery_mv: Some(3_940),
            }
        );
    }

    #[test]
    fn battery_supplement_is_reported_only_when_measured() {
        assert_eq!(
            classify_measurement(Power_measurement {
                vbus_present: true,
                battery_charging: false,
                battery_supplying: true,
                battery_percent: Some(18),
                battery_mv: Some(3_650),
            })
            .source,
            Power_source::Usb_and_battery
        );
    }

    #[test]
    fn absent_vbus_uses_battery_without_claiming_charging() {
        let state = classify_measurement(Power_measurement {
            vbus_present: false,
            battery_charging: false,
            battery_supplying: true,
            battery_percent: Some(55),
            battery_mv: Some(3_820),
        });
        assert_eq!(state.source, Power_source::Battery);
        assert_eq!(state.charging, None);
    }

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
