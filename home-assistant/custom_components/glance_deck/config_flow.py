from __future__ import annotations

from typing import Any

import voluptuous as vol

from homeassistant import config_entries
from homeassistant.data_entry_flow import FlowResult
from homeassistant.helpers.aiohttp_client import async_get_clientsession

from .api import GlanceDeckApiClient, GlanceDeckApiError, GlanceDeckApiUnauthorized
from .const import CONF_API_TOKEN, CONF_BASE_URL, DOMAIN


class GlanceDeckConfigFlow(config_entries.ConfigFlow, domain=DOMAIN):
    """Handle the Glance Deck control-plane connection."""

    VERSION = 1

    async def async_step_user(
        self, user_input: dict[str, Any] | None = None
    ) -> FlowResult:
        errors: dict[str, str] = {}
        if user_input is not None:
            base_url = user_input[CONF_BASE_URL].rstrip("/")
            client = GlanceDeckApiClient(
                async_get_clientsession(self.hass), base_url, user_input[CONF_API_TOKEN]
            )
            try:
                await client.async_get_devices()
            except GlanceDeckApiUnauthorized:
                errors["base"] = "invalid_auth"
            except GlanceDeckApiError:
                errors["base"] = "cannot_connect"
            else:
                await self.async_set_unique_id(base_url)
                self._abort_if_unique_id_configured()
                return self.async_create_entry(
                    title=f"Glance Deck ({base_url})",
                    data={CONF_BASE_URL: base_url, CONF_API_TOKEN: user_input[CONF_API_TOKEN]},
                )

        return self.async_show_form(
            step_id="user",
            data_schema=vol.Schema({
                vol.Required(CONF_BASE_URL): str,
                vol.Required(CONF_API_TOKEN): str,
            }),
            errors=errors,
        )
