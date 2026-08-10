from __future__ import annotations

from typing import Any

import voluptuous as vol

from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant, ServiceCall
from homeassistant.helpers import config_validation as cv
from homeassistant.helpers.aiohttp_client import async_get_clientsession
from homeassistant.components.http import HomeAssistantView
from aiohttp import web

from .api import GlanceDeckApiClient
from .const import (
    ATTR_DEVICE_ID,
    ATTR_PAGE_ID,
    CONF_API_TOKEN,
    CONF_BASE_URL,
    DOMAIN,
    PLATFORMS,
    SERVICE_REFRESH_DEVICE,
    SERVICE_SHOW_PAGE,
    SERVICE_START_OTA,
)
from .coordinator import GlanceDeckCoordinator

type GlanceDeckConfigEntry = ConfigEntry[GlanceDeckCoordinator]


async def async_setup_entry(hass: HomeAssistant, entry: GlanceDeckConfigEntry) -> bool:
    """Set up the integration from a config entry."""
    api = GlanceDeckApiClient(
        async_get_clientsession(hass), entry.data[CONF_BASE_URL], entry.data[CONF_API_TOKEN]
    )
    coordinator = GlanceDeckCoordinator(hass, entry, api)
    await coordinator.async_config_entry_first_refresh()
    entry.runtime_data = coordinator

    if not hass.data.setdefault(DOMAIN, {}).get("preview_view_registered"):
        hass.http.register_view(GlanceDeckPreviewView(hass))
        hass.data[DOMAIN]["preview_view_registered"] = True

    await hass.config_entries.async_forward_entry_setups(entry, PLATFORMS)
    _async_register_services(hass)
    return True


async def async_unload_entry(hass: HomeAssistant, entry: GlanceDeckConfigEntry) -> bool:
    """Unload the integration."""
    return await hass.config_entries.async_unload_platforms(entry, PLATFORMS)


def _async_register_services(hass: HomeAssistant) -> None:
    if hass.services.has_service(DOMAIN, SERVICE_SHOW_PAGE):
        return

    async def async_command(call: ServiceCall, action: str, payload: dict[str, Any] | None = None) -> None:
        device_id = call.data[ATTR_DEVICE_ID]
        for entry in hass.config_entries.async_entries(DOMAIN):
            coordinator: GlanceDeckCoordinator = entry.runtime_data
            if coordinator.device(device_id) is not None:
                await coordinator.api.async_command(device_id, action, payload)
                await coordinator.async_request_refresh()
                return
        raise ValueError(f"Unknown Glance Deck device: {device_id}")

    async def async_show_page(call: ServiceCall) -> None:
        await async_command(call, "show_page", {ATTR_PAGE_ID: call.data[ATTR_PAGE_ID]})

    async def async_refresh_device(call: ServiceCall) -> None:
        await async_command(call, "refresh_release")

    async def async_start_ota(call: ServiceCall) -> None:
        device_id = call.data[ATTR_DEVICE_ID]
        for entry in hass.config_entries.async_entries(DOMAIN):
            coordinator: GlanceDeckCoordinator = entry.runtime_data
            if coordinator.device(device_id) is not None:
                await coordinator.api.async_start_ota(device_id)
                await coordinator.async_request_refresh()
                return
        raise ValueError(f"Unknown Glance Deck device: {device_id}")

    hass.services.async_register(
        DOMAIN,
        SERVICE_SHOW_PAGE,
        async_show_page,
        schema=cv.make_entity_service_schema({
            vol.Required(ATTR_DEVICE_ID): cv.string,
            vol.Required(ATTR_PAGE_ID): cv.string,
        }),
    )
    hass.services.async_register(
        DOMAIN,
        SERVICE_REFRESH_DEVICE,
        async_refresh_device,
        schema=cv.make_entity_service_schema({vol.Required(ATTR_DEVICE_ID): cv.string}),
    )
    hass.services.async_register(
        DOMAIN,
        SERVICE_START_OTA,
        async_start_ota,
        schema=cv.make_entity_service_schema({vol.Required(ATTR_DEVICE_ID): cv.string}),
    )


class GlanceDeckPreviewView(HomeAssistantView):
    """Serve backend previews to authenticated Lovelace cards without exposing tokens."""

    url = "/api/glance_deck/preview/{entry_id}/{device_id}"
    name = "api:glance_deck:preview"
    requires_auth = True

    def __init__(self, hass: HomeAssistant) -> None:
        self.hass = hass

    async def get(self, request: web.Request, entry_id: str, device_id: str) -> web.Response:
        entry = self.hass.config_entries.async_get_entry(entry_id)
        if entry is None or entry.domain != DOMAIN or entry.runtime_data is None:
            return web.Response(status=404)
        coordinator: GlanceDeckCoordinator = entry.runtime_data
        if coordinator.device(device_id) is None:
            return web.Response(status=404)
        image, content_type = await coordinator.api.async_get_preview(device_id)
        return web.Response(body=image, content_type=content_type)
