#include "rlcd.h"
#include <driver/gpio.h>
#include <driver/spi_master.h>
#include <esp_check.h>
#include <esp_lcd_panel_io.h>
#include <freertos/FreeRTOS.h>
#include <freertos/semphr.h>
#include <freertos/task.h>

// GPIO pins from Waveshare's ESP-IDF ST7305 BSP.
#define RLCD_DC_GPIO GPIO_NUM_5
#define RLCD_SCLK_GPIO GPIO_NUM_11
#define RLCD_MOSI_GPIO GPIO_NUM_12
#define RLCD_CS_GPIO GPIO_NUM_40
#define RLCD_RESET_GPIO GPIO_NUM_41
static esp_lcd_panel_io_handle_t io;
static SemaphoreHandle_t color_transfer_done;

static bool color_transfer_complete(
    esp_lcd_panel_io_handle_t panel_io,
    esp_lcd_panel_io_event_data_t *event_data,
    void *user_context
) {
    (void)panel_io;
    (void)event_data;
    (void)user_context;

    BaseType_t higher_priority_task_woken = pdFALSE;
    xSemaphoreGiveFromISR(color_transfer_done, &higher_priority_task_woken);
    return higher_priority_task_woken == pdTRUE;
}

static esp_err_t command(uint8_t value) {
    return esp_lcd_panel_io_tx_param(io, value, NULL, 0);
}

static esp_err_t command_data(uint8_t value, const uint8_t *values, size_t length) {
    ESP_RETURN_ON_ERROR(command(value), "rlcd", "command");
    for (size_t index = 0; index < length; index++) {
        ESP_RETURN_ON_ERROR(
            esp_lcd_panel_io_tx_param(io, -1, &values[index], 1),
            "rlcd",
            "parameter"
        );
    }
    return ESP_OK;
}

esp_err_t glance_deck_rlcd_init(void) {
    spi_bus_config_t bus = { .mosi_io_num = RLCD_MOSI_GPIO, .miso_io_num = -1, .sclk_io_num = RLCD_SCLK_GPIO, .quadwp_io_num = -1, .quadhd_io_num = -1, .max_transfer_sz = GLANCE_DECK_RLCD_WIDTH * GLANCE_DECK_RLCD_HEIGHT };
    ESP_RETURN_ON_ERROR(spi_bus_initialize(SPI3_HOST, &bus, SPI_DMA_CH_AUTO), "rlcd", "SPI init");
    esp_lcd_panel_io_spi_config_t spi = {
        .cs_gpio_num = RLCD_CS_GPIO,
        .dc_gpio_num = RLCD_DC_GPIO,
        .pclk_hz = 20000000,
        .lcd_cmd_bits = 8,
        .lcd_param_bits = 8,
        .spi_mode = 0,
        .trans_queue_depth = 10,
        .on_color_trans_done = color_transfer_complete,
    };
    ESP_RETURN_ON_ERROR(esp_lcd_new_panel_io_spi((esp_lcd_spi_bus_handle_t)SPI3_HOST, &spi, &io), "rlcd", "panel IO init");
    color_transfer_done = xSemaphoreCreateBinary();
    ESP_RETURN_ON_FALSE(color_transfer_done, ESP_ERR_NO_MEM, "rlcd", "color transfer semaphore");
    gpio_config_t outputs = { .pin_bit_mask = 1ULL << RLCD_RESET_GPIO, .mode = GPIO_MODE_OUTPUT, .pull_up_en = GPIO_PULLUP_ENABLE, .pull_down_en = GPIO_PULLDOWN_DISABLE, .intr_type = GPIO_INTR_DISABLE };
    ESP_RETURN_ON_ERROR(gpio_config(&outputs), "rlcd", "reset GPIO init");
    gpio_set_level(RLCD_RESET_GPIO, 1); vTaskDelay(pdMS_TO_TICKS(50)); gpio_set_level(RLCD_RESET_GPIO, 0); vTaskDelay(pdMS_TO_TICKS(20)); gpio_set_level(RLCD_RESET_GPIO, 1); vTaskDelay(pdMS_TO_TICKS(50));
    const uint8_t nvm_load[] = {0x17, 0x02};
    const uint8_t gate_voltage[] = {0x11, 0x04};
    const uint8_t source_high[] = {0x69, 0x69, 0x69, 0x69};
    const uint8_t source_negative[] = {0x4B, 0x4B, 0x4B, 0x4B};
    const uint8_t source_low[] = {0x19, 0x19, 0x19, 0x19};
    const uint8_t oscillator[] = {0x80, 0xE9};
    const uint8_t gate_high[] = {0xE5, 0xF6, 0x05, 0x46, 0x77, 0x77, 0x77, 0x77, 0x76, 0x45};
    const uint8_t gate_low[] = {0x05, 0x46, 0x77, 0x77, 0x77, 0x77, 0x76, 0x45};
    const uint8_t gate_timing[] = {0x32, 0x03, 0x1F};

    ESP_RETURN_ON_ERROR(command_data(0xD6, nvm_load, sizeof(nvm_load)), "rlcd", "NVM load");
    ESP_RETURN_ON_ERROR(command_data(0xD1, (const uint8_t[]){0x01}, 1), "rlcd", "booster");
    ESP_RETURN_ON_ERROR(command_data(0xC0, gate_voltage, sizeof(gate_voltage)), "rlcd", "gate voltage");
    ESP_RETURN_ON_ERROR(command_data(0xC1, source_high, sizeof(source_high)), "rlcd", "positive source");
    ESP_RETURN_ON_ERROR(command_data(0xC2, source_low, sizeof(source_low)), "rlcd", "positive source low");
    ESP_RETURN_ON_ERROR(command_data(0xC4, source_negative, sizeof(source_negative)), "rlcd", "negative source");
    ESP_RETURN_ON_ERROR(command_data(0xC5, source_low, sizeof(source_low)), "rlcd", "negative source low");
    ESP_RETURN_ON_ERROR(command_data(0xD8, oscillator, sizeof(oscillator)), "rlcd", "oscillator");
    ESP_RETURN_ON_ERROR(command_data(0xB2, (const uint8_t[]){0x02}, 1), "rlcd", "frame rate");
    ESP_RETURN_ON_ERROR(command_data(0xB3, gate_high, sizeof(gate_high)), "rlcd", "gate high");
    ESP_RETURN_ON_ERROR(command_data(0xB4, gate_low, sizeof(gate_low)), "rlcd", "gate low");
    ESP_RETURN_ON_ERROR(command_data(0x62, gate_timing, sizeof(gate_timing)), "rlcd", "gate timing");
    ESP_RETURN_ON_ERROR(command_data(0xB7, (const uint8_t[]){0x13}, 1), "rlcd", "source EQ");
    ESP_RETURN_ON_ERROR(command_data(0xB0, (const uint8_t[]){0x64}, 1), "rlcd", "gate line");
    ESP_RETURN_ON_ERROR(command(0x11), "rlcd", "sleep out"); vTaskDelay(pdMS_TO_TICKS(200));
    ESP_RETURN_ON_ERROR(command_data(0xC9, (const uint8_t[]){0x00}, 1), "rlcd", "format");
    ESP_RETURN_ON_ERROR(command_data(0x36, (const uint8_t[]){0x48}, 1), "rlcd", "MADCTL");
    ESP_RETURN_ON_ERROR(command_data(0x3A, (const uint8_t[]){0x11}, 1), "rlcd", "pixel format");
    ESP_RETURN_ON_ERROR(command_data(0xB9, (const uint8_t[]){0x20}, 1), "rlcd", "enable 1");
    ESP_RETURN_ON_ERROR(command_data(0xB8, (const uint8_t[]){0x29}, 1), "rlcd", "enable 2");
    ESP_RETURN_ON_ERROR(command(0x21), "rlcd", "inversion");
    ESP_RETURN_ON_ERROR(command_data(0x2A, (const uint8_t[]){0x12, 0x2A}, 2), "rlcd", "column");
    ESP_RETURN_ON_ERROR(command_data(0x2B, (const uint8_t[]){0x00, 0xC7}, 2), "rlcd", "row");
    ESP_RETURN_ON_ERROR(command_data(0x35, (const uint8_t[]){0x00}, 1), "rlcd", "TE");
    ESP_RETURN_ON_ERROR(command_data(0xD0, (const uint8_t[]){0xFF}, 1), "rlcd", "frame format");
    ESP_RETURN_ON_ERROR(command(0x38), "rlcd", "idle off"); return command(0x29);
}
esp_err_t glance_deck_rlcd_flush(const uint8_t *frame, size_t length) {
    if (!io || !color_transfer_done || !frame || length != GLANCE_DECK_RLCD_FRAME_BYTES) return ESP_ERR_INVALID_ARG;

    ESP_RETURN_ON_ERROR(command_data(0x2A, (const uint8_t[]){0x12, 0x2A}, 2), "rlcd", "column");
    ESP_RETURN_ON_ERROR(command_data(0x2B, (const uint8_t[]){0x00, 0xC7}, 2), "rlcd", "row");

    ESP_RETURN_ON_ERROR(command(0x2C), "rlcd", "RAM write");
    ESP_RETURN_ON_ERROR(esp_lcd_panel_io_tx_color(io, -1, frame, length), "rlcd", "frame transfer");
    return xSemaphoreTake(color_transfer_done, pdMS_TO_TICKS(1000)) == pdTRUE
        ? ESP_OK
        : ESP_ERR_TIMEOUT;
}
