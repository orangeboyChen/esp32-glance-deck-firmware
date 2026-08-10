#pragma once

#include "esp_err.h"

/** Initialize the Waveshare RLCD display adapter. */
esp_err_t display_init(void);

/** Render the most recently validated display document. */
esp_err_t display_render(void);
