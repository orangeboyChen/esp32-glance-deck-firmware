use crate::mqtt::{DevicePowerState, PowerSource};

pub trait PowerProvider {
    fn sample(&mut self) -> DevicePowerState;
}

pub trait PowerMeasurementReader {
    type Error;

    fn read_measurement(&mut self) -> Result<PowerMeasurement, Self::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PowerMeasurement {
    pub vbus_present: bool,
    pub battery_charging: bool,
    pub battery_supplying: bool,
    pub battery_percent: Option<u8>,
    pub battery_mv: Option<u16>,
}

pub fn classify_measurement(measurement: PowerMeasurement) -> DevicePowerState {
    let source = match (measurement.vbus_present, measurement.battery_supplying) {
        (true, true) => PowerSource::UsbAndBattery,
        (true, false) => PowerSource::Usb,
        (false, _) => PowerSource::Battery,
    };

    DevicePowerState {
        source,
        charging: measurement
            .vbus_present
            .then_some(measurement.battery_charging),
        battery_percent: measurement.battery_percent,
        battery_mv: measurement.battery_mv,
    }
}

pub struct UnavailablePowerProvider;

impl PowerProvider for UnavailablePowerProvider {
    fn sample(&mut self) -> DevicePowerState {
        DevicePowerState {
            source: PowerSource::Unavailable,
            charging: None,
            battery_percent: None,
            battery_mv: None,
        }
    }
}

pub struct MeasuredPowerProvider<R> {
    reader: R,
}

impl<R> MeasuredPowerProvider<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R> PowerProvider for MeasuredPowerProvider<R>
where
    R: PowerMeasurementReader,
{
    fn sample(&mut self) -> DevicePowerState {
        self.reader
            .read_measurement()
            .map(classify_measurement)
            .unwrap_or(DevicePowerState {
                source: PowerSource::Unavailable,
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
            classify_measurement(PowerMeasurement {
                vbus_present: true,
                battery_charging: true,
                battery_supplying: false,
                battery_percent: Some(72),
                battery_mv: Some(3_940),
            }),
            DevicePowerState {
                source: PowerSource::Usb,
                charging: Some(true),
                battery_percent: Some(72),
                battery_mv: Some(3_940),
            }
        );
    }

    #[test]
    fn battery_supplement_is_reported_only_when_measured() {
        assert_eq!(
            classify_measurement(PowerMeasurement {
                vbus_present: true,
                battery_charging: false,
                battery_supplying: true,
                battery_percent: Some(18),
                battery_mv: Some(3_650),
            })
            .source,
            PowerSource::UsbAndBattery
        );
    }

    #[test]
    fn absent_vbus_uses_battery_without_claiming_charging() {
        let state = classify_measurement(PowerMeasurement {
            vbus_present: false,
            battery_charging: false,
            battery_supplying: true,
            battery_percent: Some(55),
            battery_mv: Some(3_820),
        });
        assert_eq!(state.source, PowerSource::Battery);
        assert_eq!(state.charging, None);
    }

    #[test]
    fn unavailable_carrier_never_fabricates_power_measurements() {
        let mut provider = UnavailablePowerProvider;
        assert_eq!(
            provider.sample(),
            DevicePowerState {
                source: PowerSource::Unavailable,
                charging: None,
                battery_percent: None,
                battery_mv: None,
            }
        );
    }

    struct TestReader(Result<PowerMeasurement, ()>);

    impl PowerMeasurementReader for TestReader {
        type Error = ();

        fn read_measurement(&mut self) -> Result<PowerMeasurement, Self::Error> {
            self.0.clone()
        }
    }

    #[test]
    fn measured_provider_fails_closed_and_converts_gauge_values() {
        let mut provider = MeasuredPowerProvider::new(TestReader(Err(())));
        assert_eq!(provider.sample().source, PowerSource::Unavailable);
        assert_eq!(max17048_voltage_mv(0xCCCD), 4_096);
        assert_eq!(max17048_percent(72 << 8), 72);
    }
}
