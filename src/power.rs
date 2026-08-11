use crate::mqtt::{Device_power_state, Power_source};

pub trait Power_provider {
    fn sample(&mut self) -> Device_power_state;
}

pub trait Power_measurement_reader {
    type Error;

    fn read_measurement(&mut self) -> Result<Power_measurement, Self::Error>;
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

pub struct Measured_power_provider<R> {
    reader: R,
}

impl<R> Measured_power_provider<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R> Power_provider for Measured_power_provider<R>
where
    R: Power_measurement_reader,
{
    fn sample(&mut self) -> Device_power_state {
        self.reader
            .read_measurement()
            .map(classify_measurement)
            .unwrap_or(Device_power_state {
                source: Power_source::Unavailable,
                charging: None,
                battery_percent: None,
                battery_mv: None,
            })
    }
}

pub fn max17048_voltage_mv(raw: u16) -> u16 {
    ((raw as u32 * 5) / 64).min(u16::MAX as u32) as u16
}

pub fn max17048_percent(raw: u16) -> u8 {
    ((raw >> 8).min(100)) as u8
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

    struct Test_reader(Result<Power_measurement, ()>);

    impl Power_measurement_reader for Test_reader {
        type Error = ();

        fn read_measurement(&mut self) -> Result<Power_measurement, Self::Error> {
            self.0.clone()
        }
    }

    #[test]
    fn measured_provider_fails_closed_and_converts_gauge_values() {
        let mut provider = Measured_power_provider::new(Test_reader(Err(())));
        assert_eq!(provider.sample().source, Power_source::Unavailable);
        assert_eq!(max17048_voltage_mv(0xCCCD), 4_096);
        assert_eq!(max17048_percent(72 << 8), 72);
    }
}
