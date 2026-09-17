use crate::mqtt::{CommandStatus, DeviceCommand, DeviceCommandAction, DeviceState};

/// How long a maintenance long-press sequence stays pending. Beyond this the presses are treated as
/// unrelated, so one stray long press cannot hijack a later short press.
pub const MAINTENANCE_SEQUENCE_TIMEOUT_MS: u64 = 3_000;

/// Counts the long presses that walk the device into the Wi-Fi setup portal.
///
/// The counter is a *sequence*: three presses in a row mean "reprovision". Time matters, because a
/// counter with no expiry stays pending forever once incremented, and the next short press is then
/// silently reinterpreted as "confirm the pending step" instead of changing the page.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MaintenanceSequence {
    presses: u8,
    started_at_ms: Option<u64>,
}

impl MaintenanceSequence {
    pub const REQUIRED_PRESSES: u8 = 3;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn presses(&self) -> u8 {
        self.presses
    }

    /// Returns the screen to show for the press that was just registered.
    pub fn register_long_press(&mut self, now_ms: u64) -> u8 {
        if self
            .started_at_ms
            .is_some_and(|started| now_ms.saturating_sub(started) > MAINTENANCE_SEQUENCE_TIMEOUT_MS)
        {
            self.presses = 0;
        }
        self.presses = self.presses.saturating_add(1);
        self.started_at_ms = Some(now_ms);
        self.presses
    }

    /// Consumes any pending sequence. `Some(presses)` means the short press was absorbed by the
    /// sequence rather than cycling the page; `None` means the caller should change the page.
    pub fn register_short_press(&mut self, now_ms: u64) -> Option<u8> {
        if self
            .started_at_ms
            .is_some_and(|started| now_ms.saturating_sub(started) > MAINTENANCE_SEQUENCE_TIMEOUT_MS)
        {
            self.clear();
            return None;
        }
        if self.presses == 0 {
            return None;
        }
        let presses = self.presses;
        self.clear();
        Some(presses)
    }

    pub fn clear(&mut self) {
        self.presses = 0;
        self.started_at_ms = None;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalScreen {
    Release { page_id: String },
    Maintenance { phase: MaintenancePhase },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaintenancePhase {
    Overview,
    ConfirmReprovisioning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReprovisioningState {
    Inactive,
    PortalStarting,
    PortalActive { ssid: String, password: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeedbackKind {
    PageChanged,
    MaintenanceEntered,
    ReprovisionConfirmation,
    MaintenanceCancelled,
    Offline,
    Error { message: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceRuntime {
    enabled_pages: Vec<String>,
    page_index: usize,
    last_release_page: String,
    pub screen: LocalScreen,
    pub reprovisioning: ReprovisioningState,
    indicator_deadline_ms: Option<u64>,
    pub feedback: Option<FeedbackKind>,
}

impl DeviceRuntime {
    pub fn new(enabled_pages: Vec<String>) -> Self {
        let first_page = enabled_pages
            .first()
            .cloned()
            .unwrap_or_else(|| "system".to_owned());
        Self {
            enabled_pages,
            page_index: 0,
            last_release_page: first_page.clone(),
            screen: LocalScreen::Release {
                page_id: first_page,
            },
            reprovisioning: ReprovisioningState::Inactive,
            indicator_deadline_ms: None,
            feedback: None,
        }
    }

    pub fn short_key_press(&mut self) {
        if matches!(self.screen, LocalScreen::Maintenance { .. }) {
            self.cancel_maintenance();
            return;
        }
        if self.enabled_pages.is_empty() {
            return;
        }
        self.page_index = (self.page_index + 1) % self.enabled_pages.len();
        self.screen = LocalScreen::Release {
            page_id: self.enabled_pages[self.page_index].clone(),
        };
        self.last_release_page = self.enabled_pages[self.page_index].clone();
        self.feedback = Some(FeedbackKind::PageChanged);
    }

    pub fn long_key_press(&mut self) {
        match self.screen {
            LocalScreen::Release { ref page_id } => {
                self.last_release_page = page_id.clone();
                self.screen = LocalScreen::Maintenance {
                    phase: MaintenancePhase::Overview,
                };
                self.feedback = Some(FeedbackKind::MaintenanceEntered);
            }
            LocalScreen::Maintenance {
                phase: MaintenancePhase::Overview,
            } => {
                self.screen = LocalScreen::Maintenance {
                    phase: MaintenancePhase::ConfirmReprovisioning,
                };
                self.feedback = Some(FeedbackKind::ReprovisionConfirmation);
            }
            LocalScreen::Maintenance {
                phase: MaintenancePhase::ConfirmReprovisioning,
            } => {
                self.reprovisioning = ReprovisioningState::PortalStarting;
            }
        }
    }

    pub fn portal_started(&mut self, ssid: String, password: String) -> Result<(), &'static str> {
        if self.reprovisioning != ReprovisioningState::PortalStarting {
            return Err("reprovisioning_not_confirmed");
        }
        self.reprovisioning = ReprovisioningState::PortalActive { ssid, password };
        self.feedback = Some(FeedbackKind::ReprovisionConfirmation);
        Ok(())
    }

    pub fn finish_reprovisioning(&mut self) {
        self.reprovisioning = ReprovisioningState::Inactive;
        self.cancel_maintenance();
    }

    pub fn cancel_maintenance(&mut self) {
        self.reprovisioning = ReprovisioningState::Inactive;
        self.screen = LocalScreen::Release {
            page_id: self.last_release_page.clone(),
        };
        self.feedback = Some(FeedbackKind::MaintenanceCancelled);
    }

    pub fn show_page_indicator_until(&mut self, now_ms: u64) {
        self.indicator_deadline_ms = Some(now_ms + 2_000);
    }
    pub fn page_indicator_visible(&self, now_ms: u64) -> bool {
        self.indicator_deadline_ms
            .is_some_and(|deadline| now_ms < deadline)
    }
    pub fn page_indicator(&self) -> Option<(usize, usize)> {
        (!self.enabled_pages.is_empty()).then_some((self.page_index, self.enabled_pages.len()))
    }

    pub fn apply_command(&mut self, command: &DeviceCommand) -> Result<(), &'static str> {
        match command.action {
            DeviceCommandAction::ShowPage => {
                let page_id = command.payload.page_id.as_ref().ok_or("page_id_required")?;
                self.page_index = self
                    .enabled_pages
                    .iter()
                    .position(|page| page == page_id)
                    .ok_or("page_not_enabled")?;
                self.screen = LocalScreen::Release {
                    page_id: page_id.clone(),
                };
                self.last_release_page = page_id.clone();
            }
            DeviceCommandAction::NextPage => self.short_key_press(),
            DeviceCommandAction::EnterMaintenance => self.long_key_press(),
            DeviceCommandAction::PreviousPage
            | DeviceCommandAction::SetRotation
            | DeviceCommandAction::RefreshRelease => {}
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
            LocalScreen::Release { page_id } => page_id.clone(),
            LocalScreen::Maintenance { .. } => "system".to_owned(),
        };
        let (command_status, error_message) = match result {
            Ok(()) => (Some(CommandStatus::Confirmed), None),
            Err(error) => (Some(CommandStatus::Failed), Some(error.to_owned())),
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
            power: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mqtt::CommandPayload;

    #[test]
    fn key_press_cycles_pages_and_requires_three_long_presses_for_portal() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned(), "alerts".to_owned()]);
        runtime.short_key_press();
        assert_eq!(
            runtime.screen,
            LocalScreen::Release {
                page_id: "alerts".to_owned()
            }
        );
        runtime.long_key_press();
        assert_eq!(
            runtime.screen,
            LocalScreen::Maintenance {
                phase: MaintenancePhase::Overview
            }
        );
        assert_eq!(runtime.reprovisioning, ReprovisioningState::Inactive);
        runtime.long_key_press();
        assert_eq!(
            runtime.screen,
            LocalScreen::Maintenance {
                phase: MaintenancePhase::ConfirmReprovisioning
            }
        );
        runtime.long_key_press();
        assert_eq!(runtime.reprovisioning, ReprovisioningState::PortalStarting);
        assert_eq!(
            runtime.portal_started("GlanceDeck-Setup".to_owned(), "secret".to_owned()),
            Ok(())
        );
    }

    #[test]
    fn short_press_cancels_maintenance_and_restores_last_release() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned(), "home".to_owned()]);
        runtime.short_key_press();
        runtime.long_key_press();
        runtime.long_key_press();
        runtime.short_key_press();
        assert_eq!(
            runtime.screen,
            LocalScreen::Release {
                page_id: "home".to_owned()
            }
        );
        assert_eq!(runtime.reprovisioning, ReprovisioningState::Inactive);
        assert_eq!(runtime.feedback, Some(FeedbackKind::MaintenanceCancelled));
    }

    #[test]
    fn page_indicator_expires_after_two_seconds() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned(), "home".to_owned()]);
        runtime.short_key_press();
        runtime.show_page_indicator_until(1_000);
        assert_eq!(runtime.page_indicator(), Some((1, 2)));
        assert!(runtime.page_indicator_visible(2_999));
        assert!(!runtime.page_indicator_visible(3_000));
        assert_eq!(runtime.feedback, Some(FeedbackKind::PageChanged));
    }

    #[test]
    fn applies_commands_and_reports_confirmed_or_failed_state() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned(), "alerts".to_owned()]);
        let show_alerts = DeviceCommand {
            command_id: "one".to_owned(),
            action: DeviceCommandAction::ShowPage,
            payload: CommandPayload {
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
        assert_eq!(state.command_status, Some(CommandStatus::Confirmed));
        let invalid = DeviceCommand {
            payload: CommandPayload::default(),
            ..show_alerts
        };
        assert_eq!(runtime.apply_command(&invalid), Err("page_id_required"));
        let failed = runtime.state(-55, None, Some("two".to_owned()), Err("page_id_required"));
        assert_eq!(failed.command_status, Some(CommandStatus::Failed));
        assert_eq!(failed.error_message.as_deref(), Some("page_id_required"));
    }

    #[test]
    fn supports_next_and_maintenance_commands_and_rejects_missing_pages() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned()]);
        let next = DeviceCommand {
            command_id: "next".to_owned(),
            action: DeviceCommandAction::NextPage,
            payload: CommandPayload::default(),
        };
        assert_eq!(runtime.apply_command(&next), Ok(()));
        let maintenance = DeviceCommand {
            command_id: "maint".to_owned(),
            action: DeviceCommandAction::EnterMaintenance,
            payload: CommandPayload::default(),
        };
        assert_eq!(runtime.apply_command(&maintenance), Ok(()));
        assert_eq!(
            runtime.screen,
            LocalScreen::Maintenance {
                phase: MaintenancePhase::Overview
            }
        );
        let missing = DeviceCommand {
            command_id: "missing".to_owned(),
            action: DeviceCommandAction::ShowPage,
            payload: CommandPayload {
                page_id: Some("missing".to_owned()),
                rotation_seconds: None,
            },
        };
        assert_eq!(runtime.apply_command(&missing), Err("page_not_enabled"));
        for action in [
            DeviceCommandAction::PreviousPage,
            DeviceCommandAction::SetRotation,
            DeviceCommandAction::RefreshRelease,
        ] {
            assert_eq!(
                runtime.apply_command(&DeviceCommand {
                    command_id: "ignored".to_owned(),
                    action,
                    payload: CommandPayload::default()
                }),
                Ok(())
            );
        }
    }
}

#[cfg(test)]
mod maintenance_sequence_tests {
    use super::*;

    #[test]
    fn three_presses_in_succession_request_the_portal() {
        let mut sequence = MaintenanceSequence::new();
        assert_eq!(sequence.register_long_press(0), 1);
        assert_eq!(sequence.register_long_press(500), 2);
        assert_eq!(sequence.register_long_press(900), 3);
        assert!(sequence.presses() >= MaintenanceSequence::REQUIRED_PRESSES);
    }

    #[test]
    fn a_stale_sequence_does_not_start_from_its_old_count() {
        let mut sequence = MaintenanceSequence::new();
        sequence.register_long_press(0);
        // An hour passes. The earlier press is unrelated to this one.
        assert_eq!(sequence.register_long_press(3_600_000), 1);
    }

    #[test]
    fn a_short_press_after_the_timeout_changes_the_page() {
        let mut sequence = MaintenanceSequence::new();
        // One accidental long press, then the user walks away.
        sequence.register_long_press(0);
        // A short press much later must cycle the page, not confirm the stale step. This is the
        // regression: without the timeout the press is absorbed and the page never changes.
        assert_eq!(sequence.register_short_press(60_000), None);
        assert_eq!(sequence.presses(), 0);
    }

    #[test]
    fn a_short_press_inside_the_timeout_absorbs_the_sequence() {
        let mut sequence = MaintenanceSequence::new();
        sequence.register_long_press(0);
        assert_eq!(sequence.register_short_press(500), Some(1));
        // The sequence is spent, so the next short press cycles the page again.
        assert_eq!(sequence.register_short_press(600), None);
    }

    #[test]
    fn a_short_press_on_an_idle_device_cycles_the_page() {
        let mut sequence = MaintenanceSequence::new();
        assert_eq!(sequence.register_short_press(0), None);
    }

    #[test]
    fn the_press_count_never_overflows() {
        let mut sequence = MaintenanceSequence::new();
        for step in 0..300_u64 {
            sequence.register_long_press(step);
        }
        assert_eq!(sequence.presses(), u8::MAX);
    }

    /// A device with no enabled pages must not panic on a button press.
    #[test]
    fn ignores_a_short_press_when_no_pages_are_enabled() {
        let mut runtime = DeviceRuntime::new(Vec::new());
        runtime.short_key_press();
        assert_eq!(runtime.page_indicator(), None);
        assert!(runtime.feedback.is_none());
    }

    /// The portal can only be marked active once the operator confirmed it, so a stray callback
    /// cannot publish credentials the user never asked for.
    #[test]
    fn rejects_a_portal_that_was_never_confirmed() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned()]);
        assert_eq!(
            runtime.portal_started("ssid".to_owned(), "pass".to_owned()),
            Err("reprovisioning_not_confirmed")
        );
        runtime.long_key_press();
        runtime.long_key_press();
        runtime.long_key_press();
        assert_eq!(
            runtime.portal_started("GlanceDeck-Setup".to_owned(), "secret".to_owned()),
            Ok(())
        );
        assert_eq!(
            runtime.reprovisioning,
            ReprovisioningState::PortalActive {
                ssid: "GlanceDeck-Setup".to_owned(),
                password: "secret".to_owned()
            }
        );
    }

    /// Finishing reprovisioning returns to the release screen and drops the portal state.
    #[test]
    fn finish_reprovisioning_returns_to_the_release_page() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned()]);
        runtime.long_key_press();
        runtime.long_key_press();
        runtime.long_key_press();
        runtime
            .portal_started("GlanceDeck-Setup".to_owned(), "secret".to_owned())
            .unwrap();
        runtime.finish_reprovisioning();
        assert_eq!(runtime.reprovisioning, ReprovisioningState::Inactive);
        assert_eq!(
            runtime.screen,
            LocalScreen::Release {
                page_id: "usage".to_owned()
            }
        );
    }

    /// While maintenance is on screen the device still reports a page id the control plane
    /// recognises, rather than an empty or maintenance-specific value.
    #[test]
    fn reports_the_system_page_while_maintenance_is_on_screen() {
        let mut runtime = DeviceRuntime::new(vec!["usage".to_owned()]);
        runtime.long_key_press();
        let state = runtime.state(-60, Some("release".to_owned()), None, Ok(()));
        assert_eq!(state.page_id, "system");
        assert_eq!(state.command_status, Some(CommandStatus::Confirmed));
        assert!(state.error_message.is_none());
    }

    /// A failed command is reported with its reason so the console can surface it.
    #[test]
    fn reports_a_failed_command_with_its_reason() {
        let runtime = DeviceRuntime::new(vec!["usage".to_owned()]);
        let state = runtime.state(-60, None, Some("cmd-1".to_owned()), Err("page_not_enabled"));
        assert_eq!(state.command_id.as_deref(), Some("cmd-1"));
        assert_eq!(state.command_status, Some(CommandStatus::Failed));
        assert_eq!(state.error_message.as_deref(), Some("page_not_enabled"));
    }
}
