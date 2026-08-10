from __future__ import annotations

from types import SimpleNamespace
from unittest.mock import AsyncMock

import pytest
from homeassistant.helpers.update_coordinator import UpdateFailed

from custom_components.glance_deck.api import GlanceDeckApiError
from custom_components.glance_deck.coordinator import GlanceDeckCoordinator


def make_coordinator(api):
    coordinator = object.__new__(GlanceDeckCoordinator)
    coordinator.api = api
    coordinator.data = None
    return coordinator


@pytest.mark.asyncio
async def test_coordinator_combines_device_and_display() -> None:
    api = SimpleNamespace(async_get_devices=AsyncMock(return_value=[{"id": "deck-a"}, {"id": 1}]), async_get_display=AsyncMock(return_value={"version": 2}))
    coordinator = make_coordinator(api)
    assert await coordinator._async_update_data() == {"deck-a": {"id": "deck-a", "display": {"version": 2}}}
    coordinator.data = {"deck-a": {"id": "deck-a"}}
    assert coordinator.device("deck-a") == {"id": "deck-a"}
    assert coordinator.device("missing") is None


@pytest.mark.asyncio
async def test_coordinator_wraps_api_errors() -> None:
    coordinator = make_coordinator(SimpleNamespace(async_get_devices=AsyncMock(side_effect=GlanceDeckApiError("offline"))))
    with pytest.raises(UpdateFailed):
        await coordinator._async_update_data()
