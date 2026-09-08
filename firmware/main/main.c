/**
 * @file main.c
 * @brief VORAX ESP32-S3 Firmware Application Entrypoint
 * 
 * Configures the ESP32-S3 as a high-frequency RF CSI probe:
 * 1. Promiscuous 64-subcarrier HT20 CSI capture on Core 0
 * 2. Real-time FreeRTOS queue handoff
 * 3. Dedicated UDP multicast streaming task on Core 1
 * 4. 1 Hz diagnostic heartbeat logging
 */

#include <stdio.h>
#include "esp_log.h"
#include "nvs_flash.h"
#include "esp_event.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "csi_rx.h"
#include "udp_sender.h"

static const char *TAG = "vorax_main";

#define DEFAULT_WIFI_CHANNEL 6
#define DEFAULT_NODE_ID      1
#define DEFAULT_TARGET_IP    "255.255.255.255"
#define DEFAULT_TARGET_PORT  5555

static void diagnostics_task(void *pvParameters) {
    uint32_t last_rx = 0;
    uint32_t last_tx = 0;

    while (1) {
        vTaskDelay(pdMS_TO_TICKS(1000));

        uint32_t rx_total = 0;
        uint32_t rx_dropped = 0;
        uint32_t tx_total = 0;
        uint32_t tx_errors = 0;

        vorax_csi_rx_get_stats(&rx_total, &rx_dropped);
        vorax_udp_sender_get_stats(&tx_total, &tx_errors);

        uint32_t rx_pps = rx_total - last_rx;
        uint32_t tx_pps = tx_total - last_tx;
        last_rx = rx_total;
        last_tx = tx_total;

        ESP_LOGI(TAG, "⚡ [METRICS] RX Rate: %u pkts/s | TX Rate: %u pkts/s | Dropped: %u | Errors: %u",
                 rx_pps, tx_pps, rx_dropped, tx_errors);
    }
}

void app_main(void) {
    ESP_LOGI(TAG, "=================================================");
    ESP_LOGI(TAG, "     VORAX ESP32-S3 CSI MULTICAST PROBE v0.1.0   ");
    ESP_LOGI(TAG, "=================================================");

    // 1. Initialize NVS (required for Wi-Fi stack)
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }
    ESP_ERROR_CHECK(ret);

    // 2. Initialize default event loop
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    // 3. Initialize Wi-Fi CSI promiscuous capture on Channel 6
    ESP_ERROR_CHECK(vorax_csi_rx_init(DEFAULT_WIFI_CHANNEL, DEFAULT_NODE_ID));

    // 4. Start high-speed UDP transmission task on Core 1
    ESP_ERROR_CHECK(vorax_udp_sender_start(DEFAULT_TARGET_IP, DEFAULT_TARGET_PORT));

    // 5. Spawn 1 Hz diagnostics task
    xTaskCreate(diagnostics_task, "diag_task", 2048, NULL, 1, NULL);

    ESP_LOGI(TAG, "System operational — streaming 64-subcarrier CSI to port %u", DEFAULT_TARGET_PORT);
}
