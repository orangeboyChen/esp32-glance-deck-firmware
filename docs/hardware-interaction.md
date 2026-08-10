# Hardware interaction contract

This contract applies to the Waveshare ESP32-S3-RLCD-4.2 device. The console
and Home Assistant remain the control plane; the physical interface provides
fast, reversible local actions and recovery only.

## KEY button

The board KEY is GPIO18 and is active low. Firmware polls it at 20 ms and
debounces it before acting.

| Gesture | Result | Feedback |
| --- | --- | --- |
| Short press | Advance one enabled cached page in the configured order. | The selected cached frame flushes first. A bottom page indicator appears for 2 seconds. Publish state only after the flush succeeds. |
| Long press | Enter the local maintenance page. | State is visible as text, an icon, and an optional audio cue. It never changes Wi-Fi, credentials, or firmware by itself. |
| Second long press in maintenance | Request Wi-Fi reprovisioning confirmation. | Show the temporary AP name and password before the portal starts. A short press cancels and returns to the last page. |

The normal page order is `usage`, `alerts` (when an alert is active), `home`,
`environment`, then `system`. The control plane must keep this a flat list of
at most 10 enabled pages. A short press always remains a reversible action;
firmware installation and factory reset require an explicit remote action or a
separate physical recovery flow.

## Page indicator and display states

- Render small, equally spaced dots centered at the lower edge of the cached
  frame. The active page uses a filled dot; inactive pages are outlined dots.
- The indicator is position-only and must not replace the page title. It shows
  for 2 seconds after a local or remote page change and never animates while a
  page is changing.
- The display is reflective and monochrome. Use black/white shape and text
  differences; never use color as the only status cue. The backend must supply
  a validated `400 x 300` 1-bit ST7305 frame with a 15,000 byte payload.
- Keep text concise, use sentence case, and leave a persistent page title plus
  a visible `Updated` or `Offline` status. The backend must produce a high-
  contrast bitmap and avoid text smaller than the legible size established by
  hardware testing.
- When offline or loading, retain the last verified page. Do not blank the
  panel or show an indefinite spinner. The System page identifies stale data,
  last successful update, and a specific failure reason.

## Enrollment and reprovisioning

First boot and failed station reconnect start a WPA2 SoftAP with a random
per-start password. The maintenance page displays both the SSID and password
as text and QR data. The captive portal accepts one candidate network.

The candidate is persisted separately. On reboot it must obtain a DHCP address
before it replaces `wifi_active` in NVS. A failure leaves `wifi_active`, every
display cache entry, and the visible frame untouched. No credentials appear in
MQTT state, logs outside the local serial console, or the normal System page.

## OTA and errors

- Remote OTA state shows a named phase: `Downloading`, `Verifying`,
  `Restarting`, `Checking`, `Complete`, `Rolled back`, or the concrete failure
  reason. It does not report success merely because a command was received.
- A local OTA check is non-destructive. Starting an update requires a second
  deliberate confirmation and always presents `Cancel` first.
- Use audio only as supplementary feedback. Every success, warning, and error
  also has an on-screen text and glyph/shape cue. Audio alert playback is
  user-configurable from the console.
- Pair critical errors with the specific corrective action, for example
  `Wi-Fi connection failed. Reopen setup.` rather than `Error`.
