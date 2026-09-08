/**
 * @file csi_rx.h
 * @brief VORAX Zero-Copy Promiscuous CSI Receiver
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "esp_err.h"
#include "esp_wifi_types.h"
#include "csi_packet.h"

#ifdef __cplusplus
extern "C" {
#endif

#define VORAX_MAX_SUBCARRIERS 64
#define VORAX_CSI_QUEUE_DEPTH 32

/**
 * @brief Raw captured CSI frame queued from the Wi-Fi RX ISR
 */
typedef struct {
    vorax_csi_header_t header;
    vorax_subcarrier_t subcarriers[VORAX_MAX_SUBCARRIERS];
} vorax_frame_item_t;

/**
 * @brief Initializes Wi-Fi in promiscuous mode and enables CSI capture
 * 
 * @param channel Wi-Fi channel (1-14)
 * @param node_id Unique 16-bit identifier for this physical board
 * @return esp_err_t ESP_OK on success
 */
esp_err_t vorax_csi_rx_init(uint8_t channel, uint16_t node_id);

/**
 * @brief Retrieves the next CSI frame from the ISR handoff queue
 * 
 * @param item Pointer to receive frame data
 * @param timeout_ms FreeRTOS timeout in milliseconds
 * @return bool true if a frame was successfully dequeued
 */
bool vorax_csi_rx_dequeue(vorax_frame_item_t *item, uint32_t timeout_ms);

/**
 * @brief Fetches cumulative reception and drop counters
 */
void vorax_csi_rx_get_stats(uint32_t *out_received, uint32_t *out_dropped);

#ifdef __cplusplus
}
#endif
