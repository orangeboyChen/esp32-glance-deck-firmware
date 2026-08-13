use super::canvas::Canvas;

const ICON_SIZE: usize = 32;

#[derive(Clone, Copy)]
pub enum Icon {
    Check,
    CheckMark,
    Checking,
    Download,
    Error,
    Failed,
    Maintenance,
    Pairing,
    ShortPress,
    LongPress,
    Update,
    Wifi,
}

pub fn draw_icon(canvas: &mut Canvas, icon: Icon, left: usize, top: usize) {
    let bitmap = match icon {
        Icon::Check => include_bytes!("../../assets/local-icons/check.mono1"),
        Icon::CheckMark => include_bytes!("../../assets/local-icons/check-mark.mono1"),
        Icon::Checking => include_bytes!("../../assets/local-icons/checking.mono1"),
        Icon::Download => include_bytes!("../../assets/local-icons/download.mono1"),
        Icon::Error => include_bytes!("../../assets/local-icons/error.mono1"),
        Icon::Failed => include_bytes!("../../assets/local-icons/failed.mono1"),
        Icon::Maintenance => include_bytes!("../../assets/local-icons/maintenance.mono1"),
        Icon::Pairing => include_bytes!("../../assets/local-icons/pairing.mono1"),
        Icon::ShortPress => include_bytes!("../../assets/local-icons/short-press.mono1"),
        Icon::LongPress => include_bytes!("../../assets/local-icons/long-press.mono1"),
        Icon::Update => include_bytes!("../../assets/local-icons/update.mono1"),
        Icon::Wifi => include_bytes!("../../assets/local-icons/wifi.mono1"),
    };
    canvas.blit_mono1(left, top, ICON_SIZE, ICON_SIZE, bitmap);
}
