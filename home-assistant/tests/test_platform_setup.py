from __future__ import annotations

from types import SimpleNamespace

import pytest

from custom_components.glance_deck import binary_sensor, button, select, sensor, update


class FakeCoordinator:
    def __init__(self) -> None:
        self.data = {"deck-a": {"id": "deck-a"}}
        self.listeners = []

    def async_add_listener(self, listener):
        self.listeners.append(listener)
        return lambda: None

    def device(self, device_id):
        return self.data.get(device_id)


@pytest.mark.asyncio
@pytest.mark.parametrize("platform", [binary_sensor, button, select, sensor, update])
async def test_platform_setup_adds_entities_and_tracks_new_devices(platform) -> None:
    coordinator = FakeCoordinator()
    entry = SimpleNamespace(runtime_data=coordinator, async_on_unload=lambda callback: callback)
    added = []
    await platform.async_setup_entry(SimpleNamespace(), entry, lambda entities: added.extend(list(entities)))
    assert added
    initial_count = len(added)
    coordinator.data["deck-b"] = {"id": "deck-b"}
    coordinator.listeners[0]()
    assert len(added) > initial_count
