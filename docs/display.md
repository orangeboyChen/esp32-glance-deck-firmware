# Display screens

This document describes every 400 x 300 one-bit screen in the current device
contract. The control plane renders normal pages to verified bitmaps; firmware
renders pairing, maintenance, and Wi-Fi recovery screens locally.

## Common behavior

- A release enables one to ten pages in a console-defined order.
- A page change flushes a verified frame once. It does not use a full-frame
  animation because reflective-LCD animation consumes power and flickers.
- After a successful local or remote page change, bottom dots appear for two
  seconds: `●` is the active page and `○` is another enabled page.
- When offline, an uncached target never replaces the visible frame. Cached
  pages remain switchable and the last verified frame remains visible.
- Normal pages can contain CJK text because the server rasterizes bundled
  fonts. Local recovery screens use a bounded uppercase glyph set.

## Normal pages

The page editor supplies a title, optional subtitle, and up to seven label /
value rows. `usage`, `alerts`, `home`, `environment`, and `system` are useful
conventions, not special firmware page types.

### Usage

```text
┌──────────────────────────────────────┐
│ ↗ Usage                       Online │
│ Pro plan                             │
│                                      │
│ Used 72%                  72 / 100   │
│ [██████████████████░░░░░░░░]         │
│ Today remaining                  28% │
│ This week used                   411 │
│ Resets                    Tomorrow │
│                                      │
│ Updated 09:30               ● ○ ○ ○ │
└──────────────────────────────────────┘
```

The usage icon (`↗`) and the filled meter are rendered into the immutable
bitmap by the console. Firmware validates the image hash and only flushes the
verified frame; it does not need a font or token-service credentials.

The console derives daily and weekly values from source snapshots. It can use
this format for any aggregated usage provider, not only a quota service.

### Alerts

```text
┌──────────────────────────────────────┐
│ ! Alert                    Attention │
│ Monthly quota low                    │
│                                      │
│ Remaining                     8%     │
│ Action             Review plan usage │
│ Triggered                 09:28      │
│                                      │
│ Updated 09:30               ○ ● ○ ○ │
└──────────────────────────────────────┘
```

Test-only alerts never control a device. Optional audio is supplementary; the
visible text and glyph remain the authoritative alert feedback.

### Home, environment, and custom pages

```text
┌──────────────────────────────────────┐
│ Home                         Online │
│ Good morning                         │
│                                      │
│ Calendar                    2 events │
│ Temperature                    24 C │
│ Humidity                        48% │
│                                      │
│ Updated 09:30               ○ ○ ● ○ │
└──────────────────────────────────────┘
```

The labels and values are fully controlled by the display document. A device
downloads only the page it is about to show; it reuses a matching cached hash.

### System and offline state

```text
┌──────────────────────────────────────┐
│ System                        Offline │
│ Last verified page retained          │
│                                      │
│ Wi-Fi                       Reconnect │
│ Last update                   09:30  │
│ Power             Battery unavailable│
│ Firmware                       0.1.0 │
│                                      │
│ Offline                       ○ ○ ○ ●│
└──────────────────────────────────────┘
```

Until the optional power carrier is installed, `Battery unavailable` is the
correct state. The device never invents a percentage or charging status.

## Local navigation overlay

```text
┌──────────────────────────────────────┐
│                                      │
│          verified page frame         │
│                                      │
│                                      │
│                    ● ○ ○ ○           │
└──────────────────────────────────────┘
```

A short KEY press advances to the next enabled cached page and wraps around.
Firmware flushes the frame before it reports the confirmed page to MQTT. The
indicator expires after two seconds and can be disabled in critical-battery or
reduced-motion modes.

## Maintenance and Wi-Fi setup

Long-pressing KEY on a normal page enters maintenance. A short press cancels
from either maintenance state and restores the last normal page.

### Maintenance overview

```text
┌──────────────────────────────────────┐
│                 ⚙                    │
│             MAINTENANCE              │
│                                      │
│ Short: check update                  │
│ Long: Wi-Fi setup                    │
│                                      │
└──────────────────────────────────────┘
```

The local renderer displays `MAINTENANCE`. The update check can then display a
bounded status such as `CHECKING`, `UP TO DATE`, or `UPDATE READY`; an update
still needs a separate explicit confirmation.

### Wi-Fi setup confirmation

```text
┌──────────────────────────────────────┐
│                                      │
│            START WIFI SETUP          │
│                                      │
│ Long press again to confirm          │
│ Short press to cancel                │
│                                      │
└──────────────────────────────────────┘
```

The third long press starts the temporary provisioning access point. A
candidate network only replaces the active NVS configuration after it obtains
DHCP successfully.

### Wi-Fi setup credentials

```text
┌──────────────────────────────────────┐
│                                      │
│                 )))                  │
│              WIFI SETUP              │
│                                      │
│              GD12AB34EF              │
│                                      │
└──────────────────────────────────────┘
```

The shown ten-character value is the per-start WPA2 password. It is not sent
to MQTT or stored in normal display documents.

The `)))` glyph is drawn locally by firmware, so it remains available while
the device is offline and running its temporary provisioning access point.

## Enrollment pairing

```text
┌──────────────────────────────────────┐
│                 🔗                    │
│            1  2  3  4  5  6          │
│                                      │
│       Enter this code in Console     │
│                                      │
└──────────────────────────────────────┘
```

Firmware renders the short-lived six-digit pairing code and link glyph locally
inside a border. The explanatory sentence is documentation only; the current
local renderer contains the code rather than arbitrary prose.

## OTA status

```text
┌──────────────────────────────────────┐
│ System                               │
│                                      │
│ OTA: Verifying                       │
│ Keep external power connected        │
│                                      │
│ Downloading -> Verifying -> Restart  │
└──────────────────────────────────────┘
```

Remote OTA and locally confirmed OTA share these states: `Downloading`,
`Verifying`, `Restarting`, `Checking`, `Complete`, `Rolled back`, or a concrete
failure. Firmware refuses OTA below its configured safe battery threshold
unless it measures external power.

## State transitions

```text
Normal page --short KEY / show_page--> verified target --2 s--> normal page
     | long KEY
     v
Maintenance --long KEY--> Wi-Fi confirmation --long KEY--> Wi-Fi setup
     | short KEY                    | short KEY
     +------------------------------+---------------------> last normal page
```
