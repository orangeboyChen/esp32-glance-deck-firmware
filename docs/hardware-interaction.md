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
| First long press | Enter the local maintenance overview. | State is visible as text, an icon, and an optional audio cue. It never changes Wi-Fi, credentials, or firmware by itself. |
| Second long press in maintenance | Present `Start Wi-Fi setup?` confirmation. | The screen presents the primary action and a visible `Cancel` action. A short press cancels and returns to the last page. |
| Third long press at confirmation | Start temporary Wi-Fi setup. | Show the temporary AP name and password before the portal starts. A short press still cancels and restores the last release page. |

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
  panel or show an indefinite spinner. Cached pages remain locally switchable;
  an uncached requested page waits for connectivity without replacing the
  visible frame. The System page identifies stale data, last successful update,
  and a specific failure reason.

## Page transition and power

- A page change has no full-frame animation. The renderer flushes the verified
  target frame once, then overlays the position dots for two seconds. This is
  deliberate: repeatedly refreshing a reflective LCD for visual animation
  wastes battery and produces distracting flicker. The overlay is disabled in
  reduced-motion and critical-battery modes.
- Battery operation requires the approved external power carrier below. The
  ESP32-S3-RLCD-4.2 board must not have a Li-ion cell connected directly to
  its 5 V USB input, 3.3 V rail, or any GPIO. Its published board interface is
  not evidence of an integrated charger, load-sharing circuit, battery
  protection, or fuel gauge.
- The power driver reports source, charging state, and, when measurable,
  battery percentage and millivolts every 15 minutes, after a material power
  change, and before OTA. The System screen always shows the same current
  values and a text state such as `Battery unavailable`; it never fabricates a
  percentage.
- On battery below 20%, extend source-refresh and telemetry intervals. Below
  10%, disable optional audio and page-transition overlay. Below the configured
  OTA threshold (default 30% unless external power is present), reject OTA with
  the explicit `power_unsafe_for_ota` result.

## External battery power carrier

The battery option is an external USB-C power carrier between the USB source,
one protected single-cell Li-ion/LiPo battery, and the display board's normal
USB power input. It has a **real power-path charger**, not a simple TP4056-type
charger board. The reference design uses a BQ25895 (or an electrically
equivalent single-cell switching charger with NVDC power path, input-current
limit, JEITA/NTC support, charge termination, and power-good status) plus a
dedicated fuel gauge such as MAX17048. Final IC selection and component values
must be checked against the selected battery, enclosure temperature, and USB-C
connector layout before a PCB is ordered.

```text
USB-C 5 V VBUS
  -> resettable fuse + TVS + reverse-current protection
  -> BQ25895 VIN  ----> input-current limit / thermal regulation
                         |                    |
protected 1S cell <---- BAT              SYS --+--> high-efficiency 5 V boost
       |                         (USB first,   |       -> load switch / fuse
       +--> MAX17048 fuel gauge          battery only on loss) -> board USB-C
       +--> 10 kOhm NTC to charger TS

charger PG/STAT + gauge alert -> ESP32 GPIOs (level-safe, optional)
gauge I2C -> ESP32 I2C (only after bus-voltage validation)
```

`SYS` is the only source for the boost converter. With VBUS connected, the
charger's power path supplies the display load from USB while independently
charging the cell. At charge termination it stops charging; it does not keep
cycling the battery to power the display. If the display's instantaneous load
exceeds the negotiated/advertised USB input limit, Dynamic Power Management
may allow a bounded battery supplement rather than brown out; the carrier must
report this as `usb_and_battery`. When VBUS is removed, SYS switches to the
cell and the 5 V boost keeps the board alive. No firmware decision is involved
in this switchover.

The resulting operating states are intentionally distinct:

| VBUS | Cell current | Display source | Meaning |
| --- | --- | --- | --- |
| Present | Into cell | USB | Normal plugged-in operation; the cell is charging or already full. |
| Present | Near zero | USB | Charge terminated or suspended by temperature; no battery cycling. |
| Present | Out of cell | USB + battery | Short, measured input-limit/transient supplement only. |
| Absent | Out of cell | Battery | Standalone battery operation. |

The charger must expose input-current limiting and power-good/charge status.
Firmware reports these measurements but does not implement source switching or
charge termination in software. A fuel-gauge percentage alone is never used to
infer whether the display is charging or discharging.

The boost converter must be sized for measured Wi-Fi transmit and RLCD refresh
peaks with margin, include its recommended input/output capacitors, and have a
low quiescent current appropriate for standby. It supplies **only** the board's
normal 5 V USB-power path; do not back-feed the board 3.3 V rail. The USB-C
receptacle is a 5 V sink (CC pull-downs, no unimplemented PD claim), and its
data lines are left isolated unless a separately reviewed USB data design needs
them. Add a battery connector with polarity protection, a cell protector when
the pack does not include one, and the charger-required NTC. Never charge a
bare or unprotected cell.

Charging current is a hardware configuration, not a firmware guess: cap it to
the connector/source contract and battery specification (start at USB-default
current during bring-up). A product carrier must validate worst-case charger
temperature at low cell voltage, repeated USB insertion, brown-out behavior,
and boost efficiency. The firmware can request a lower charge/input limit only
when the selected charger exposes that control safely; it must never enable a
higher current than the carrier's safe fixed limit.

The MAX17048 (or equivalent compensated fuel gauge) reports cell voltage and
state of charge. `PG`, charge state, and gauge readings map to MQTT power state:
`usb` means external power with no battery contribution, `usb_and_battery`
means a measurable supplement/charging path, `battery` means VBUS absent, and
`unavailable` means the optional carrier or its telemetry is absent. The console
must display the source, charging state, last sample time, percentage, and
voltage, and must treat unknown telemetry conservatively for OTA.

## Enrollment and reprovisioning

First boot and failed station reconnect start a WPA2 SoftAP with a random
per-start password. The maintenance page displays both the SSID and password
as text and QR data. The captive portal accepts one candidate network. Active
reprovisioning uses the three-step maintenance flow above; it is never started
by a single long press or a remote page command.

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
