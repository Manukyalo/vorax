/**
 * @file csi_rx.c
 * @brief Implementation of VORAX Zero-Copy Promiscuous CSI Capture
 */

#include "csi_rx.h"
#include <string.h>
#include "esp_log.h"
#include "esp_wifi.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h"
#include "freertos/queue.h"

static const char *TAG = "vorax_csi_rx";

static QueueHandle_t s_csi_queue = NULL;
static uint16_t s_node_id = 1;
static uint8_t s_channel = 6;
static uint32_t s_sequence_counter = 0;
static uint32_t s_frames_received = 0;
static uint32_t s_frames_dropped = 0;

/**
 * @brief High-frequency Wi-Fi CSI Hardware Callback
 * 
 * Invoked directly from the Wi-Fi task/ISR context upon physical packet demodulation.
 * Must remain strictly non-blocking.
 */
static void wifi_csi_raw_cb(void *ctx, wifi_csi_info_t *data) {
    if (!data || !data->buf || data->len < sizeof(vorax_subcarrier_t) * VORAX_MAX_SUBCARRIERS) {
        return;
    }

    BaseType_t xHigherPriorityTaskWoken = pdFALSE;
    vorax_frame_item_t frame;

    // Hardware timestamp in microseconds
    frame.header.magic = VORAX_PACKET_MAGIC;
    frame.header.node_id = s_node_id;
    frame.header.sequence_number = s_sequence_counter++;
    frame.header.timestamp_us = (uint64_t)esp_timer_get_time();
    frame.header.channel = s_channel;
    frame.header.rssi = (int8_t)data->rx_ctrl.rssi;
    frame.header.noise_floor = (int8_t)data->rx_ctrl.noise_floor;
    frame.header.subcarrier_count = VORAX_MAX_SUBCARRIERS;

    // Zero-copy direct memory copy of interleaved I/Q subcarrier bytes
    memcpy(frame.subcarriers, data->buf, sizeof(vorax_subcarrier_t) * VORAX_MAX_SUBCARRIERS);

    s_frames_received++;

    // Attempt non-blocking enqueue into the FreeRTOS buffer
    if (s_csi_queue != NULL) {
        if (xQueueSendFromISR(s_csi_queue, &frame, &xHigherPriorityTaskWoken) != pdPASS) {
            s_frames_dropped++;
        }
    }

    if (xHigherPriorityTaskWoken == pdTRUE) {
        portYIELD_FROM_ISR();
    }
}

esp_err_t vorax_csi_rx_init(uint8_t channel, uint16_t node_id) {
    s_channel = channel;
    s_node_id = node_id;
    s_sequence_counter = 0;
    s_frames_received = 0;
    s_frames_dropped = 0;

    // Allocate static FreeRTOS bounded frame queue
    s_csi_queue = xQueueCreate(VORAX_CSI_QUEUE_DEPTH, sizeof(vorax_frame_item_t));
    if (s_csi_queue == NULL) {
        ESP_LOGE(TAG, "Failed to allocate CSI FreeRTOS queue");
        return ESP_ERR_NO_MEM;
    }

    // Configure Wi-Fi in Promiscuous Mode
    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));
    ESP_ERROR_CHECK(esp_wifi_set_storage(WIFI_STORAGE_RAM));
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_NULL));
    ESP_ERROR_CHECK(esp_wifi_start());

    // Lock to desired RF channel
    ESP_ERROR_CHECK(esp_wifi_set_promiscuous(true));
    ESP_ERROR_CHECK(esp_wifi_set_channel(s_channel, WIFI_SECOND_CHAN_NONE));

    // Enable CSI capture on HT20 / OFDM frames
    wifi_csi_config_t csi_config = {
        .lltf_en           = true,
        .htltf_en          = true,
        .stbc_htltf2_en    = true,
        .ltf_merge_en      = true,
        .channel_filter_en = false,
        .manu_scale        = false,
        .shift             = false,
    };
    ESP_ERROR_CHECK(esp_wifi_set_csi_config(&csi_config));
    ESP_ERROR_CHECK(esp_wifi_set_csi_rx_cb(wifi_csi_raw_cb, NULL));
    ESP_ERROR_CHECK(esp_wifi_set_csi(true));

    ESP_LOGI(TAG, "VORAX CSI Promiscuous Capture initialized on Channel %u (Node ID: %u)", s_channel, s_node_id);
    return ESP_OK;
}

bool vorax_csi_rx_dequeue(vorax_frame_item_t *item, uint32_t timeout_ms) {
    if (s_csi_queue == NULL || item == NULL) {
        return false;
    }
    TickType_t ticks = timeout_ms == 0 ? 0 : pdMS_TO_TICKS(timeout_ms);
    return xQueueReceive(s_csi_queue, item, ticks) == pdPASS;
}

void vorax_csi_rx_get_stats(uint32_t *out_received, uint32_t *out_dropped) {
    if (out_received) *out_received = s_frames_received;
    if (out_dropped) *out_dropped = s_frames_dropped;
}
