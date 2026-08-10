#include <inttypes.h>
#include <stdio.h>

#include "esp_event.h"
#include "esp_log.h"
#include "esp_netif.h"
#include "nvs_flash.h"

static const char *TAG = "glance_deck";

static void initialize_platform(void)
{
    ESP_ERROR_CHECK(nvs_flash_init());
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
}

void app_main(void)
{
    initialize_platform();

    ESP_LOGI(TAG, "ESP32 Glance Deck starting");
    ESP_LOGI(TAG, "awaiting Wi-Fi provisioning and display initialization");
}
