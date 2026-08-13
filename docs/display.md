# Display specification

The canonical, versioned display contract is
[`../../docs/display.md`](../../docs/display.md). Firmware must implement that
document; this file intentionally contains no duplicate screen descriptions or
ASCII mockups.

The implementation lives in `src/local_screen/`. Local states use the same
Noto Sans SC source font as console pages, pre-rasterized to a small embedded
ASCII glyph subset at build time so pairing and recovery remain available
offline.
