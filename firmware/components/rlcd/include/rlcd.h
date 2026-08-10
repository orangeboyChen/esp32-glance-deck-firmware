#pragma once
#include <stddef.h>
#include <stdint.h>
#include <esp_err.h>

#define GLANCE_DECK_RLCD_WIDTH 400
#define GLANCE_DECK_RLCD_HEIGHT 300
#define GLANCE_DECK_RLCD_FRAME_BYTES ((GLANCE_DECK_RLCD_WIDTH * GLANCE_DECK_RLCD_HEIGHT) / 8)
esp_err_t glance_deck_rlcd_init(void);
esp_err_t glance_deck_rlcd_flush(const uint8_t *frame, size_t length);
