use crate::{
    display::DisplayCache,
    mqtt::{DeviceState, DeviceTopics, Mqtt_client},
    pages::{PageNavigator, PageRenderer},
};

pub struct PageController {
    navigator: PageNavigator,
}

impl PageController {
    pub fn new(navigator: PageNavigator) -> Self {
        Self { navigator }
    }

    /// Apply a locally-originated page change: first flush a hash-verified cached
    /// frame to the RLCD renderer, then publish its confirmed state when MQTT is up.
    pub fn next_page<C, R, M>(
        &mut self,
        cache: &C,
        renderer: &mut R,
        mqtt: Option<&mut M>,
        topics: &DeviceTopics,
        wifi_rssi: i16,
    ) -> Result<(), PageControllerError<C::Error, R::Error, M::Error>>
    where
        C: DisplayCache,
        R: PageRenderer,
        M: Mqtt_client,
    {
        let page = self.navigator.next_page().clone();
        let image = cache
            .read_page(&page.image_sha256)
            .map_err(PageControllerError::Cache)?
            .ok_or(PageControllerError::MissingCachedPage)?;
        if image.len() != page.image_bytes {
            return Err(PageControllerError::MissingCachedPage);
        }
        renderer
            .render_cached_page(&page, &image)
            .map_err(PageControllerError::Renderer)?;

        if let Some(mqtt) = mqtt {
            let state = DeviceState {
                version: 1,
                page_id: page.page_id,
                wifi_rssi,
                display_release_id: None,
                display_updated_at: None,
                command_id: None,
                command_status: None,
                error_message: None,
                firmware_version: None,
            };
            let payload = serde_json::to_vec(&state).expect("device state must serialize");
            mqtt.publish(&topics.state(), &payload, true)
                .map_err(PageControllerError::Mqtt)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum PageControllerError<CacheError, RendererError, MqttError> {
    Cache(CacheError),
    Renderer(RendererError),
    Mqtt(MqttError),
    MissingCachedPage,
}
