from __future__ import annotations

from datetime import timedelta
import logging
from typing import Any

from homeassistant.config_entries import ConfigEntry
from homeassistant.core import HomeAssistant
from homeassistant.helpers.update_coordinator import DataUpdateCoordinator, UpdateFailed

from .api import GlanceDeckApiClient, GlanceDeckApiError
from .const import COORDINATOR_UPDATE_SECONDS, DOMAIN


class GlanceDeckCoordinator(DataUpdateCoordinator[dict[str, dict[str, Any]]]):
    """Fetch device state from the control plane."""

    def __init__(self, hass: HomeAssistant, entry: ConfigEntry, api: GlanceDeckApiClient) -> None:
        super().__init__(
            hass,
            logger=logging.getLogger(__name__),
            name=f"{DOMAIN}_{entry.entry_id}",
            update_interval=timedelta(seconds=COORDINATOR_UPDATE_SECONDS),
        )
        self.api = api

    async def _async_update_data(self) -> dict[str, dict[str, Any]]:
        try:
            devices = await self.api.async_get_devices()
            get_alerts = getattr(self.api, "async_get_alerts", None)
            alerts = await get_alerts() if get_alerts is not None else []
            active_alerts: dict[str, list[dict[str, Any]]] = {}
            for alert in alerts:
                for device_id in alert.get("device_ids", []):
                    if isinstance(device_id, str):
                        active_alerts.setdefault(device_id, []).append(alert)
            result: dict[str, dict[str, Any]] = {}
            for device in devices:
                device_id = device.get("id")
                if not isinstance(device_id, str):
                    continue
                display = await self.api.async_get_display(device_id)
                pages = await self.api.async_get_device_pages(device_id)
                result[device_id] = {**device, "display": display, "page_configuration": pages}
                if active_alerts.get(device_id):
                    result[device_id]["active_alerts"] = active_alerts[device_id]
            return result
        except GlanceDeckApiError as error:
            raise UpdateFailed(str(error)) from error

    def device(self, device_id: str) -> dict[str, Any] | None:
        return self.data.get(device_id) if self.data else None
