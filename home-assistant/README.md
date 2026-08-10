# Home Assistant integration

Home Assistant calls the Glance Deck control-plane API. It does not collect
subscription data, publish display documents, manage devices, or host firmware
files. The control plane owns those responsibilities.

The HA integration polls or subscribes to API resources for display data,
device state, and alerts, then exposes standard HA entities for dashboards and
automations. HA commands such as changing a page call the control-plane API;
the control plane publishes the MQTT command to the target device.

The first implementation will expose availability, current page, display
freshness, OTA version, and selected status/usage values.
