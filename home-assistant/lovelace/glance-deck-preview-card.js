class GlanceDeckPreviewCard extends HTMLElement {
  static getStubConfig() {
    return { entity: "sensor.glance_deck_office_current_page" };
  }

  setConfig(config) {
    if (!config.entity) throw new Error("Set an entity from the Glance Deck integration");
    this._config = config;
  }

  set hass(hass) {
    this._hass = hass;
    const state = hass.states[this._config.entity];
    if (!state) {
      this.innerHTML = `<ha-card><div class="empty">Entity not found: ${this._config.entity}</div></ha-card>`;
      return;
    }
    const attributes = state.attributes;
    const previewUrl = attributes.preview_url;
    const deviceId = attributes.glance_deck_device_id;
    const online = state.attributes.status === "online";
    this.innerHTML = `
      <ha-card>
        <div class="header"><span>Glance Deck</span><span class="status ${online ? "online" : "offline"}">${online ? "Online" : "Offline"}</span></div>
        ${previewUrl ? `<img class="preview" src="${previewUrl}" alt="Current Glance Deck display">` : "<div class=\"empty\">Preview unavailable</div>"}
        <div class="footer"><span>Page: ${state.state}</span></div>
      </ha-card>`;
  }

  getCardSize() { return 5; }

  static get styles() { return ""; }
}

customElements.define("glance-deck-preview-card", GlanceDeckPreviewCard);

window.customCards = window.customCards || [];
window.customCards.push({
  type: "glance-deck-preview-card",
  name: "Glance Deck Preview",
  description: "Displays the exact immutable preview assigned by Glance Deck.",
});
