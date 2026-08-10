from __future__ import annotations

from typing import Final

DOMAIN: Final = "glance_deck"
PLATFORMS: Final = ["binary_sensor", "button", "select", "sensor", "update"]

CONF_BASE_URL: Final = "base_url"
CONF_API_TOKEN: Final = "api_token"

SERVICE_SHOW_PAGE: Final = "show_page"
SERVICE_REFRESH_DEVICE: Final = "refresh_device"
SERVICE_START_OTA: Final = "start_ota"

ATTR_DEVICE_ID: Final = "device_id"
ATTR_PAGE_ID: Final = "page_id"

API_TIMEOUT_SECONDS: Final = 15
COORDINATOR_UPDATE_SECONDS: Final = 30
