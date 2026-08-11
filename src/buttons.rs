use anyhow::{bail, Result};
use esp_idf_svc::sys::{
    gpio_config, gpio_config_t, gpio_get_level, gpio_int_type_t_GPIO_INTR_DISABLE,
    gpio_mode_t_GPIO_MODE_INPUT, gpio_num_t_GPIO_NUM_18, gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
    gpio_pullup_t_GPIO_PULLUP_ENABLE,
};

// Waveshare ESP32-S3-RLCD-4.2 board key: GPIO18, active-low.
pub const KEY_GPIO: i32 = 18;
const DEBOUNCE_SAMPLES: u8 = 3;
const LONG_PRESS_SAMPLES: u16 = 75;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyEvent {
    ShortPress,
    LongPress,
}

pub struct KeyButton {
    stable_pressed: bool,
    candidate_pressed: bool,
    candidate_samples: u8,
    held_samples: u16,
    long_press_reported: bool,
}

impl KeyButton {
    pub fn new() -> Result<Self> {
        let configuration = gpio_config_t {
            pin_bit_mask: 1_u64 << KEY_GPIO,
            mode: gpio_mode_t_GPIO_MODE_INPUT,
            pull_up_en: gpio_pullup_t_GPIO_PULLUP_ENABLE,
            pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: gpio_int_type_t_GPIO_INTR_DISABLE,
            ..Default::default()
        };
        let result = unsafe { gpio_config(&configuration) };
        if result != 0 {
            bail!("configure KEY GPIO{KEY_GPIO} failed: {result}");
        }
        Ok(Self {
            stable_pressed: false,
            candidate_pressed: false,
            candidate_samples: 0,
            held_samples: 0,
            long_press_reported: false,
        })
    }

    /// Poll every 20 ms from the application task. The key is electrically active-low.
    pub fn poll(&mut self) -> Option<KeyEvent> {
        let pressed = unsafe { gpio_get_level(gpio_num_t_GPIO_NUM_18) } == 0;
        if pressed != self.candidate_pressed {
            self.candidate_pressed = pressed;
            self.candidate_samples = 1;
            return None;
        }
        if self.candidate_samples < DEBOUNCE_SAMPLES {
            self.candidate_samples += 1;
            return None;
        }
        if self.stable_pressed != pressed {
            self.stable_pressed = pressed;
            if pressed {
                self.held_samples = 0;
                self.long_press_reported = false;
                return None;
            }
            return (!self.long_press_reported).then_some(KeyEvent::ShortPress);
        }
        if self.stable_pressed && !self.long_press_reported {
            self.held_samples = self.held_samples.saturating_add(1);
            if self.held_samples >= LONG_PRESS_SAMPLES {
                self.long_press_reported = true;
                return Some(KeyEvent::LongPress);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_official_key_pin() {
        assert_eq!(KEY_GPIO, 18);
    }
}
