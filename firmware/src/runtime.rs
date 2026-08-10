use crate::mqtt::{Command_status, DeviceState, Device_command, Device_command_action};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Local_screen {
    Release { page_id: String },
    Maintenance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Device_runtime {
    enabled_pages: Vec<String>,
    page_index: usize,
    pub screen: Local_screen,
}

impl Device_runtime {
    pub fn new(enabled_pages: Vec<String>) -> Self {
        let first_page = enabled_pages
            .first()
            .cloned()
            .unwrap_or_else(|| "system".to_owned());
        Self {
            enabled_pages,
            page_index: 0,
            screen: Local_screen::Release {
                page_id: first_page,
            },
        }
    }

    pub fn short_key_press(&mut self) {
        if self.enabled_pages.is_empty() {
            return;
        }
        self.page_index = (self.page_index + 1) % self.enabled_pages.len();
        self.screen = Local_screen::Release {
            page_id: self.enabled_pages[self.page_index].clone(),
        };
    }

    pub fn long_key_press(&mut self) {
        self.screen = Local_screen::Maintenance;
    }

    pub fn apply_command(&mut self, command: &Device_command) -> Result<(), &'static str> {
        match command.action {
            Device_command_action::Show_page => {
                let page_id = command.payload.page_id.as_ref().ok_or("page_id_required")?;
                self.page_index = self
                    .enabled_pages
                    .iter()
                    .position(|page| page == page_id)
                    .ok_or("page_not_enabled")?;
                self.screen = Local_screen::Release {
                    page_id: page_id.clone(),
                };
            }
            Device_command_action::Next_page => self.short_key_press(),
            Device_command_action::Enter_maintenance => self.long_key_press(),
            Device_command_action::Previous_page
            | Device_command_action::Set_rotation
            | Device_command_action::Refresh_release => {}
        }
        Ok(())
    }

    pub fn state(
        &self,
        wifi_rssi: i16,
        release_id: Option<String>,
        command_id: Option<String>,
        result: Result<(), &'static str>,
    ) -> DeviceState {
        let page_id = match &self.screen {
            Local_screen::Release { page_id } => page_id.clone(),
            Local_screen::Maintenance => "system".to_owned(),
        };
        let (command_status, error_message) = match result {
            Ok(()) => (Some(Command_status::Confirmed), None),
            Err(error) => (Some(Command_status::Failed), Some(error.to_owned())),
        };
        DeviceState {
            version: 1,
            page_id,
            wifi_rssi,
            display_release_id: release_id,
            display_updated_at: None,
            command_id,
            command_status,
            error_message,
            firmware_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_press_cycles_pages_and_long_press_opens_maintenance() {
        let mut runtime = Device_runtime::new(vec!["usage".to_owned(), "alerts".to_owned()]);
        runtime.short_key_press();
        assert_eq!(
            runtime.screen,
            Local_screen::Release {
                page_id: "alerts".to_owned()
            }
        );
        runtime.long_key_press();
        assert_eq!(runtime.screen, Local_screen::Maintenance);
    }

    #[test]
    fn applies_commands_and_reports_confirmed_or_failed_state() {
        let mut runtime = Device_runtime::new(vec!["usage".to_owned(), "alerts".to_owned()]);
        let show_alerts = Device_command {
            command_id: "one".to_owned(),
            action: Device_command_action::Show_page,
            payload: Command_payload {
                page_id: Some("alerts".to_owned()),
                rotation_seconds: None,
            },
        };
        assert_eq!(runtime.apply_command(&show_alerts), Ok(()));
        let state = runtime.state(
            -55,
            Some("release".to_owned()),
            Some("one".to_owned()),
            Ok(()),
        );
        assert_eq!(state.page_id, "alerts");
        assert_eq!(state.command_status, Some(Command_status::Confirmed));
        let invalid = Device_command {
            payload: Command_payload::default(),
            ..show_alerts
        };
        assert_eq!(runtime.apply_command(&invalid), Err("page_id_required"));
        let failed = runtime.state(-55, None, Some("two".to_owned()), Err("page_id_required"));
        assert_eq!(failed.command_status, Some(Command_status::Failed));
        assert_eq!(failed.error_message.as_deref(), Some("page_id_required"));
    }

    #[test]
    fn supports_next_and_maintenance_commands_and_rejects_missing_pages() {
        let mut runtime = Device_runtime::new(vec!["usage".to_owned()]);
        let next = Device_command {
            command_id: "next".to_owned(),
            action: Device_command_action::Next_page,
            payload: Command_payload::default(),
        };
        assert_eq!(runtime.apply_command(&next), Ok(()));
        let maintenance = Device_command {
            command_id: "maint".to_owned(),
            action: Device_command_action::Enter_maintenance,
            payload: Command_payload::default(),
        };
        assert_eq!(runtime.apply_command(&maintenance), Ok(()));
        assert_eq!(runtime.screen, Local_screen::Maintenance);
        let missing = Device_command {
            command_id: "missing".to_owned(),
            action: Device_command_action::Show_page,
            payload: Command_payload {
                page_id: Some("missing".to_owned()),
                rotation_seconds: None,
            },
        };
        assert_eq!(runtime.apply_command(&missing), Err("page_not_enabled"));
        for action in [
            Device_command_action::Previous_page,
            Device_command_action::Set_rotation,
            Device_command_action::Refresh_release,
        ] {
            assert_eq!(
                runtime.apply_command(&Device_command {
                    command_id: "ignored".to_owned(),
                    action,
                    payload: Command_payload::default()
                }),
                Ok(())
            );
        }
    }
}
