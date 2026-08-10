from __future__ import annotations

import pytest
from aiohttp import ClientSession, web

from custom_components.glance_deck.api import GlanceDeckApiClient, GlanceDeckApiError, GlanceDeckApiUnauthorized


@pytest.fixture
async def server() -> str:
    app = web.Application()

    async def devices(request: web.Request) -> web.Response:
        if request.headers.get("Authorization") != "Bearer valid":
            return web.json_response({"error": "unauthorized"}, status=401)
        return web.json_response({"devices": [{"id": "deck-a"}]})

    async def display(_request: web.Request) -> web.Response:
        return web.json_response({"version": 1})

    async def pages(_request: web.Request) -> web.Response:
        return web.json_response({"enabled_page_ids": ["usage", "alerts"]})

    async def command(request: web.Request) -> web.Response:
        return web.json_response(await request.json(), status=202)

    async def preview(_request: web.Request) -> web.Response:
        return web.Response(body=b"<svg/>", content_type="image/svg+xml")

    app.router.add_get("/api/v1/devices", devices)
    app.router.add_get("/api/v1/displays/{device_id}", display)
    app.router.add_get("/api/v1/devices/{device_id}/pages", pages)
    app.router.add_post("/api/v1/devices/{device_id}/commands", command)
    app.router.add_post("/api/v1/devices/{device_id}/ota", command)
    app.router.add_get("/api/v1/devices/{device_id}/preview", preview)
    runner = web.AppRunner(app)
    await runner.setup()
    site = web.TCPSite(runner, "127.0.0.1", 0)
    await site.start()
    port = site._server.sockets[0].getsockname()[1]
    yield f"http://127.0.0.1:{port}"
    await runner.cleanup()


@pytest.mark.asyncio
async def test_client_reads_devices_and_writes_commands(server: str) -> None:
    async with ClientSession() as session:
        client = GlanceDeckApiClient(session, server, "valid")
        assert await client.async_get_devices() == [{"id": "deck-a"}]
        assert await client.async_get_display("deck-a") == {"version": 1}
        assert await client.async_get_device_pages("deck-a") == {"enabled_page_ids": ["usage", "alerts"]}
        assert (await client.async_command("deck-a", "next_page"))["action"] == "next_page"
        assert await client.async_start_ota("deck-a") == {}
        assert await client.async_get_preview("deck-a") == (b"<svg/>", "image/svg+xml")


@pytest.mark.asyncio
async def test_client_distinguishes_unauthorized(server: str) -> None:
    async with ClientSession() as session:
        with pytest.raises(GlanceDeckApiUnauthorized):
            await GlanceDeckApiClient(session, server, "wrong").async_get_devices()


@pytest.mark.asyncio
async def test_client_rejects_invalid_payload(server: str) -> None:
    async with ClientSession() as session:
        client = GlanceDeckApiClient(session, server, "valid")
        client.base_url = "http://127.0.0.1:1"
        with pytest.raises(GlanceDeckApiError):
            await client.async_get_devices()
