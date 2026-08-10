from __future__ import annotations

import importlib
from types import SimpleNamespace
from unittest.mock import AsyncMock, Mock

import pytest

from custom_components.glance_deck.api import GlanceDeckApiError, GlanceDeckApiUnauthorized
from custom_components.glance_deck.config_flow import GlanceDeckConfigFlow
from custom_components.glance_deck.const import CONF_API_TOKEN, CONF_BASE_URL, DOMAIN, SERVICE_REFRESH_DEVICE, SERVICE_SHOW_PAGE, SERVICE_START_OTA

integration = importlib.import_module("custom_components.glance_deck")


class FakeServices:
    def __init__(self) -> None:
        self.registered = {}

    def has_service(self, domain, service):
        return (domain, service) in self.registered

    def async_register(self, domain, service, handler, schema=None):
        self.registered[(domain, service)] = handler


class FakeHass:
    def __init__(self) -> None:
        self.data = {}
        self.services = FakeServices()
        self.http = SimpleNamespace(register_view=Mock())
        self.entries = []
        self.config_entries = SimpleNamespace(async_entries=lambda _domain: self.entries, async_forward_entry_setups=AsyncMock(), async_unload_platforms=AsyncMock(return_value=True), async_get_entry=Mock(return_value=None))


@pytest.mark.asyncio
async def test_config_flow_success_and_errors(monkeypatch) -> None:
    hass = FakeHass()
    flow = object.__new__(GlanceDeckConfigFlow)
    flow.hass = hass
    flow.async_set_unique_id = AsyncMock()
    flow._abort_if_unique_id_configured = Mock()
    flow.async_create_entry = Mock(return_value={"type": "create_entry"})
    flow.async_show_form = Mock(return_value={"type": "form"})
    monkeypatch.setattr("custom_components.glance_deck.config_flow.async_get_clientsession", lambda _hass: object())
    monkeypatch.setattr("custom_components.glance_deck.config_flow.GlanceDeckApiClient.async_get_devices", AsyncMock(return_value=[]))
    result = await flow.async_step_user({CONF_BASE_URL: "https://deck.example/", CONF_API_TOKEN: "token"})
    assert result["type"] == "create_entry"
    flow.async_create_entry.reset_mock()
    monkeypatch.setattr("custom_components.glance_deck.config_flow.GlanceDeckApiClient.async_get_devices", AsyncMock(side_effect=GlanceDeckApiUnauthorized()))
    await flow.async_step_user({CONF_BASE_URL: "https://deck.example", CONF_API_TOKEN: "bad"})
    assert flow.async_show_form.call_args.kwargs["errors"] == {"base": "invalid_auth"}
    monkeypatch.setattr("custom_components.glance_deck.config_flow.GlanceDeckApiClient.async_get_devices", AsyncMock(side_effect=GlanceDeckApiError()))
    await flow.async_step_user({CONF_BASE_URL: "https://deck.example", CONF_API_TOKEN: "bad"})
    assert flow.async_show_form.call_args.kwargs["errors"] == {"base": "cannot_connect"}
    await flow.async_step_user()
    assert flow.async_show_form.called


@pytest.mark.asyncio
async def test_setup_preview_and_services(monkeypatch) -> None:
    hass = FakeHass()
    api = SimpleNamespace(async_command=AsyncMock(), async_start_ota=AsyncMock(), async_get_preview=AsyncMock(return_value=(b"svg", "image/svg+xml")))
    coordinator = SimpleNamespace(api=api, data={"deck-a": {"id": "deck-a"}}, device=lambda device_id: {"id": device_id} if device_id == "deck-a" else None, async_request_refresh=AsyncMock())
    coordinator.async_config_entry_first_refresh = AsyncMock()
    monkeypatch.setattr(integration, "async_get_clientsession", lambda _hass: object())
    monkeypatch.setattr(integration, "GlanceDeckCoordinator", lambda *_args: coordinator)
    entry = SimpleNamespace(data={CONF_BASE_URL: "https://deck.example", CONF_API_TOKEN: "token"}, entry_id="entry-a", runtime_data=None)
    hass.entries.append(entry)
    result = await integration.async_setup_entry(hass, entry)
    assert result is True
    assert entry.runtime_data is coordinator
    assert hass.http.register_view.called
    await hass.services.registered[(DOMAIN, SERVICE_SHOW_PAGE)](SimpleNamespace(data={"device_id": "deck-a", "page_id": "alerts"}))
    await hass.services.registered[(DOMAIN, SERVICE_REFRESH_DEVICE)](SimpleNamespace(data={"device_id": "deck-a"}))
    await hass.services.registered[(DOMAIN, SERVICE_START_OTA)](SimpleNamespace(data={"device_id": "deck-a"}))
    view = integration.GlanceDeckPreviewView(hass)
    assert (await view.get(SimpleNamespace(), "missing", "deck-a")).status == 404
    hass.config_entries.async_get_entry.return_value = SimpleNamespace(domain=DOMAIN, runtime_data=coordinator)
    assert (await view.get(SimpleNamespace(), "entry-a", "deck-a")).body == b"svg"
    assert await integration.async_unload_entry(hass, entry) is True
