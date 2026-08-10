from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import aiohttp

from .const import API_TIMEOUT_SECONDS


class GlanceDeckApiError(Exception):
    """Raised when the Glance Deck API cannot complete a request."""


class GlanceDeckApiUnauthorized(GlanceDeckApiError):
    """Raised when the Glance Deck API token is rejected."""


@dataclass(slots=True)
class GlanceDeckApiClient:
    session: aiohttp.ClientSession
    base_url: str
    api_token: str

    def __post_init__(self) -> None:
        self.base_url = self.base_url.rstrip("/")

    @property
    def _headers(self) -> dict[str, str]:
        return {"Authorization": f"Bearer {self.api_token}"}

    async def async_get_devices(self) -> list[dict[str, Any]]:
        response = await self._async_request("GET", "/api/v1/devices")
        devices = response.get("devices") if isinstance(response, dict) else None
        if not isinstance(devices, list):
            raise GlanceDeckApiError("The devices response is invalid")
        return [device for device in devices if isinstance(device, dict)]

    async def async_get_alerts(self) -> list[dict[str, Any]]:
        response = await self._async_request("GET", "/api/v1/alerts")
        alerts = response.get("active") if isinstance(response, dict) else None
        if not isinstance(alerts, list):
            return []
        return [alert for alert in alerts if isinstance(alert, dict)]

    async def async_get_display(self, device_id: str) -> dict[str, Any]:
        response = await self._async_request("GET", f"/api/v1/displays/{device_id}", allow_not_found=True)
        return response if isinstance(response, dict) else {}

    async def async_get_device_pages(self, device_id: str) -> dict[str, Any]:
        response = await self._async_request("GET", f"/api/v1/devices/{device_id}/pages", allow_not_found=True)
        return response if isinstance(response, dict) else {}

    async def async_command(self, device_id: str, action: str, payload: dict[str, Any] | None = None) -> dict[str, Any]:
        response = await self._async_request(
            "POST",
            f"/api/v1/devices/{device_id}/commands",
            json={"action": action, "payload": payload or {}},
        )
        return response if isinstance(response, dict) else {}

    async def async_start_ota(self, device_id: str) -> dict[str, Any]:
        response = await self._async_request("POST", f"/api/v1/devices/{device_id}/ota", json={})
        return response if isinstance(response, dict) else {}

    async def async_get_preview(self, device_id: str) -> tuple[bytes, str]:
        url = f"{self.base_url}/api/v1/devices/{device_id}/preview"
        try:
            async with self.session.get(url, headers=self._headers, timeout=API_TIMEOUT_SECONDS) as response:
                if response.status == 401:
                    raise GlanceDeckApiUnauthorized("The API token was rejected")
                if response.status >= 400:
                    raise GlanceDeckApiError(f"Preview request returned HTTP {response.status}")
                return await response.read(), response.headers.get("content-type", "image/svg+xml")
        except aiohttp.ClientError as error:
            raise GlanceDeckApiError("Preview request failed") from error

    async def _async_request(
        self,
        method: str,
        path: str,
        *,
        json: dict[str, Any] | None = None,
        allow_not_found: bool = False,
    ) -> Any:
        url = f"{self.base_url}{path}"
        try:
            async with self.session.request(method, url, headers=self._headers, json=json, timeout=API_TIMEOUT_SECONDS) as response:
                if response.status == 401:
                    raise GlanceDeckApiUnauthorized("The API token was rejected")
                if allow_not_found and response.status == 404:
                    return {}
                if response.status >= 400:
                    raise GlanceDeckApiError(f"Request returned HTTP {response.status}")
                return await response.json(content_type=None)
        except aiohttp.ClientError as error:
            raise GlanceDeckApiError("Request failed") from error
