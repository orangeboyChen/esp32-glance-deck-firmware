#include "rlcd.h"
#include <driver/gpio.h>
#include <driver/spi_master.h>
#include <esp_check.h>
#include <esp_lcd_panel_io.h>
#include <freertos/FreeRTOS.h>
#include <freertos/task.h>

// GPIO pins from Waveshare's ESP-IDF ST7305 BSP.
#define RLCD_DC_GPIO GPIO_NUM_5
#define RLCD_SCLK_GPIO GPIO_NUM_11
#define RLCD_MOSI_GPIO GPIO_NUM_12
#define RLCD_CS_GPIO GPIO_NUM_40
#define RLCD_RESET_GPIO GPIO_NUM_41
static esp_lcd_panel_io_handle_t io;

static esp_err_t command(uint8_t value) {
    return esp_lcd_panel_io_tx_param(io, value, NULL, 0);
}

static esp_err_t data(uint8_t value) {
    return esp_lcd_panel_io_tx_param(io, -1, &value, 1);
}

static esp_err_t data_bytes(const uint8_t *values, size_t length) {
    for (size_t index = 0; index < length; index++) {
        ESP_RETURN_ON_ERROR(data(values[index]), "rlcd", "setup data");
    }
    return ESP_OK;
}

esp_err_t glance_deck_rlcd_init(void) {
    spi_bus_config_t bus = { .mosi_io_num = RLCD_MOSI_GPIO, .miso_io_num = -1, .sclk_io_num = RLCD_SCLK_GPIO, .quadwp_io_num = -1, .quadhd_io_num = -1, .max_transfer_sz = GLANCE_DECK_RLCD_FRAME_BYTES };
    ESP_RETURN_ON_ERROR(spi_bus_initialize(SPI3_HOST, &bus, SPI_DMA_CH_AUTO), "rlcd", "SPI init");
    esp_lcd_panel_io_spi_config_t spi = {
        .cs_gpio_num = RLCD_CS_GPIO,
        .dc_gpio_num = RLCD_DC_GPIO,
        .pclk_hz = 20000000,
        .lcd_cmd_bits = 8,
        .lcd_param_bits = 8,
        .spi_mode = 0,
        .trans_queue_depth = 1,
    };
    ESP_RETURN_ON_ERROR(esp_lcd_new_panel_io_spi((esp_lcd_spi_bus_handle_t)SPI3_HOST, &spi, &io), "rlcd", "panel IO init");
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

    ESP_RETURN_ON_ERROR(command(0xD6), "rlcd", "NVM load command"); ESP_RETURN_ON_ERROR(data_bytes(nvm_load, sizeof(nvm_load)), "rlcd", "NVM load data");
    ESP_RETURN_ON_ERROR(command(0xD1), "rlcd", "booster command"); ESP_RETURN_ON_ERROR(data(0x01), "rlcd", "booster data");
    ESP_RETURN_ON_ERROR(command(0xC0), "rlcd", "gate voltage command"); ESP_RETURN_ON_ERROR(data_bytes(gate_voltage, sizeof(gate_voltage)), "rlcd", "gate voltage data");
    ESP_RETURN_ON_ERROR(command(0xC1), "rlcd", "positive source command"); ESP_RETURN_ON_ERROR(data_bytes(source_high, sizeof(source_high)), "rlcd", "positive source data");
    ESP_RETURN_ON_ERROR(command(0xC2), "rlcd", "positive source low command"); ESP_RETURN_ON_ERROR(data_bytes(source_low, sizeof(source_low)), "rlcd", "positive source low data");
    ESP_RETURN_ON_ERROR(command(0xC4), "rlcd", "negative source command"); ESP_RETURN_ON_ERROR(data_bytes(source_negative, sizeof(source_negative)), "rlcd", "negative source data");
    ESP_RETURN_ON_ERROR(command(0xC5), "rlcd", "negative source low command"); ESP_RETURN_ON_ERROR(data_bytes(source_low, sizeof(source_low)), "rlcd", "negative source low data");
    ESP_RETURN_ON_ERROR(command(0xD8), "rlcd", "oscillator command"); ESP_RETURN_ON_ERROR(data_bytes(oscillator, sizeof(oscillator)), "rlcd", "oscillator data");
    ESP_RETURN_ON_ERROR(command(0xB2), "rlcd", "frame rate command"); ESP_RETURN_ON_ERROR(data(0x02), "rlcd", "frame rate data");
    ESP_RETURN_ON_ERROR(command(0xB3), "rlcd", "gate high command"); ESP_RETURN_ON_ERROR(data_bytes(gate_high, sizeof(gate_high)), "rlcd", "gate high data");
    ESP_RETURN_ON_ERROR(command(0xB4), "rlcd", "gate low command"); ESP_RETURN_ON_ERROR(data_bytes(gate_low, sizeof(gate_low)), "rlcd", "gate low data");
    ESP_RETURN_ON_ERROR(command(0x62), "rlcd", "gate timing command"); ESP_RETURN_ON_ERROR(data_bytes(gate_timing, sizeof(gate_timing)), "rlcd", "gate timing data");
    ESP_RETURN_ON_ERROR(command(0xB7), "rlcd", "source EQ command"); ESP_RETURN_ON_ERROR(data(0x13), "rlcd", "source EQ data");
    ESP_RETURN_ON_ERROR(command(0xB0), "rlcd", "gate line command"); ESP_RETURN_ON_ERROR(data(0x64), "rlcd", "gate line data");
    ESP_RETURN_ON_ERROR(command(0x11), "rlcd", "sleep out"); vTaskDelay(pdMS_TO_TICKS(200));
    ESP_RETURN_ON_ERROR(command(0xC9), "rlcd", "format"); ESP_RETURN_ON_ERROR(data(0x00), "rlcd", "format data");
    ESP_RETURN_ON_ERROR(command(0x36), "rlcd", "MADCTL"); ESP_RETURN_ON_ERROR(data(0x48), "rlcd", "MADCTL data");
    ESP_RETURN_ON_ERROR(command(0x3A), "rlcd", "pixel format"); ESP_RETURN_ON_ERROR(data(0x11), "rlcd", "pixel format data");
    ESP_RETURN_ON_ERROR(command(0xB9), "rlcd", "enable"); ESP_RETURN_ON_ERROR(data(0x20), "rlcd", "enable data");
    ESP_RETURN_ON_ERROR(command(0xB8), "rlcd", "enable"); ESP_RETURN_ON_ERROR(data(0x29), "rlcd", "enable data");
    ESP_RETURN_ON_ERROR(command(0x21), "rlcd", "inversion");
    ESP_RETURN_ON_ERROR(command(0x2A), "rlcd", "column"); ESP_RETURN_ON_ERROR(data(0x12), "rlcd", "column start"); ESP_RETURN_ON_ERROR(data(0x2A), "rlcd", "column end");
    ESP_RETURN_ON_ERROR(command(0x2B), "rlcd", "row"); ESP_RETURN_ON_ERROR(data(0x00), "rlcd", "row start"); ESP_RETURN_ON_ERROR(data(0xC7), "rlcd", "row end");
    ESP_RETURN_ON_ERROR(command(0x35), "rlcd", "TE"); ESP_RETURN_ON_ERROR(data(0x00), "rlcd", "TE data");
    ESP_RETURN_ON_ERROR(command(0xD0), "rlcd", "frame format"); ESP_RETURN_ON_ERROR(data(0xFF), "rlcd", "frame format data");
    ESP_RETURN_ON_ERROR(command(0x38), "rlcd", "idle off"); return command(0x29);
}
esp_err_t glance_deck_rlcd_flush(const uint8_t *frame, size_t length) {
    if (!io || !frame || length != GLANCE_DECK_RLCD_FRAME_BYTES) return ESP_ERR_INVALID_ARG;

    ESP_RETURN_ON_ERROR(command(0x2A), "rlcd", "column command");
    ESP_RETURN_ON_ERROR(data(0x12), "rlcd", "column start");
    ESP_RETURN_ON_ERROR(data(0x2A), "rlcd", "column end");
    ESP_RETURN_ON_ERROR(command(0x2B), "rlcd", "row command");
    ESP_RETURN_ON_ERROR(data(0x00), "rlcd", "row start");
    ESP_RETURN_ON_ERROR(data(0xC7), "rlcd", "row end");

    ESP_RETURN_ON_ERROR(command(0x2C), "rlcd", "RAM write");
    return esp_lcd_panel_io_tx_color(io, -1, frame, length);
}
