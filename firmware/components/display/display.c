#include "display.h"

#include "esp_log.h"

static const char *TAG = "display";

esp_err_t display_init(void)
{
    ESP_LOGI(TAG, "display adapter is not connected yet");
    return ESP_OK;
}

esp_err_t display_render(void)
{
    return ESP_OK;
}
