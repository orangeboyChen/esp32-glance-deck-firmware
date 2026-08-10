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
                power: None,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        display::{
            DisplayPage, DisplayRelease, DISPLAY_HEIGHT, DISPLAY_IMAGE_BYTES, DISPLAY_IMAGE_FORMAT,
            DISPLAY_WIDTH,
        },
        pages::CachedPage,
    };

    #[derive(Default)]
    struct MemoryCache {
        pages: HashMap<String, Vec<u8>>,
        fail_read: bool,
    }

    impl DisplayCache for MemoryCache {
        type Error = &'static str;

        fn current_release(&self) -> Result<Option<crate::display::DisplayRelease>, Self::Error> {
            Ok(None)
        }

        fn previous_release(&self) -> Result<Option<crate::display::DisplayRelease>, Self::Error> {
            Ok(None)
        }

        fn contains_page(&self, image_sha256: &str) -> Result<bool, Self::Error> {
            Ok(self.pages.contains_key(image_sha256))
        }

        fn read_page(&self, image_sha256: &str) -> Result<Option<Vec<u8>>, Self::Error> {
            if self.fail_read {
                return Err("cache_read_failed");
            }
            Ok(self.pages.get(image_sha256).cloned())
        }

        fn commit_release(
            &mut self,
            _release: &crate::display::DisplayRelease,
            _pages: &[(crate::display::DisplayPage, Vec<u8>)],
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingRenderer {
        rendered_page: Option<String>,
        fail: bool,
    }

    impl PageRenderer for RecordingRenderer {
        type Error = &'static str;

        fn render_cached_page(
            &mut self,
            page: &CachedPage,
            frame: &[u8],
        ) -> Result<(), Self::Error> {
            if self.fail {
                return Err("render_failed");
            }
            assert_eq!(frame.len(), DISPLAY_IMAGE_BYTES);
            self.rendered_page = Some(page.page_id.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingMqtt {
        publications: Vec<(String, Vec<u8>, bool)>,
        fail: bool,
    }

    impl Mqtt_client for RecordingMqtt {
        type Error = &'static str;

        fn publish(
            &mut self,
            topic: &str,
            payload: &[u8],
            retained: bool,
        ) -> Result<(), Self::Error> {
            if self.fail {
                return Err("mqtt_failed");
            }
            self.publications
                .push((topic.to_owned(), payload.to_vec(), retained));
            Ok(())
        }

        fn subscribe(&mut self, _topic: &str) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    fn controller() -> PageController {
        let page = DisplayPage {
            page_id: "usage".to_owned(),
            image_format: DISPLAY_IMAGE_FORMAT.to_owned(),
            image_width: DISPLAY_WIDTH,
            image_height: DISPLAY_HEIGHT,
            image_url: "https://example.test/usage.bin".to_owned(),
            image_sha256: "a".repeat(64),
            image_bytes: DISPLAY_IMAGE_BYTES,
        };
        let release = DisplayRelease {
            release_id: "release-1".to_owned(),
            document_version: 1,
            active_page_id: page.page_id.clone(),
            pages: vec![page],
        };
        PageController::new(PageNavigator::from_release(&release).unwrap())
    }

    fn cached_page() -> MemoryCache {
        MemoryCache {
            pages: HashMap::from([("a".repeat(64), vec![0; DISPLAY_IMAGE_BYTES])]),
            fail_read: false,
        }
    }

    #[test]
    fn renders_cached_page_before_publishing_confirmed_state() {
        let mut controller = controller();
        let cache = cached_page();
        let mut renderer = RecordingRenderer::default();
        let mut mqtt = RecordingMqtt::default();
        let topics = DeviceTopics::new("office-deck");

        assert!(controller
            .next_page(&cache, &mut renderer, Some(&mut mqtt), &topics, -48)
            .is_ok());
        assert_eq!(renderer.rendered_page.as_deref(), Some("usage"));
        assert_eq!(mqtt.publications.len(), 1);
        let (topic, payload, retained) = &mqtt.publications[0];
        assert_eq!(topic, &topics.state());
        assert!(*retained);
        let state: DeviceState = serde_json::from_slice(payload).unwrap();
        assert_eq!(state.page_id, "usage");
        assert_eq!(state.wifi_rssi, -48);
    }

    #[test]
    fn never_publishes_when_local_frame_is_unavailable_or_rendering_fails() {
        let topics = DeviceTopics::new("office-deck");
        let mut mqtt = RecordingMqtt::default();
        let mut renderer = RecordingRenderer::default();
        assert!(matches!(
            controller().next_page(
                &MemoryCache::default(),
                &mut renderer,
                Some(&mut mqtt),
                &topics,
                -48
            ),
            Err(PageControllerError::MissingCachedPage)
        ));
        assert!(mqtt.publications.is_empty());

        renderer.fail = true;
        assert!(matches!(
            controller().next_page(&cached_page(), &mut renderer, Some(&mut mqtt), &topics, -48),
            Err(PageControllerError::Renderer("render_failed"))
        ));
        assert!(mqtt.publications.is_empty());
    }

    #[test]
    fn reports_cache_and_mqtt_failures_without_faking_confirmation() {
        let topics = DeviceTopics::new("office-deck");
        let mut renderer = RecordingRenderer::default();
        assert!(matches!(
            controller().next_page(
                &MemoryCache {
                    fail_read: true,
                    ..MemoryCache::default()
                },
                &mut renderer,
                None::<&mut RecordingMqtt>,
                &topics,
                -48,
            ),
            Err(PageControllerError::Cache("cache_read_failed"))
        ));

        let mut mqtt = RecordingMqtt {
            fail: true,
            ..RecordingMqtt::default()
        };
        assert!(matches!(
            controller().next_page(&cached_page(), &mut renderer, Some(&mut mqtt), &topics, -48),
            Err(PageControllerError::Mqtt("mqtt_failed"))
        ));
        assert!(mqtt.publications.is_empty());
    }
}
