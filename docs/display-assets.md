# Display assets and CJK text

ESP32 Glance Deck does not render text on-device. The control plane converts a
display document into an immutable 300 × 400 `mono1-msb` frame before it is
published. A frame is exactly 15,000 bytes: one most-significant-bit-first
pixel per display position, where a set bit is black.

The console bundles the OFL-licensed Noto Sans SC font subset and uses it while
rasterizing release text. Chinese support therefore exists in the released
pixel frame, not through a browser font fallback or an ESP32 font lookup. The
same SVG is retained strictly as the Web and Home Assistant preview.

## Release contract

The control plane sends only the following release metadata to a device:

```json
{
  "document_version": 1,
  "image_format": "mono1-msb",
  "image_width": 300,
  "image_height": 400,
  "image_bytes": 15000
}
```

Firmware must reject every other format, dimension, or byte count before
replacing its cached image. It validates the SHA-256 hash before invoking the
board-specific RLCD transfer adapter. SVG, PNG, CSS, JavaScript, and font files
are never sent to the device.

## Font limitation

Noto Sans SC is Simplified Chinese. Traditional Chinese, Japanese, Korean, and
other scripts require their own explicitly bundled, redistributable font and a
release test that rasterizes representative glyphs. Do not claim support for a
script merely because the administrator's browser renders it.
