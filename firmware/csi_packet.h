/**
 * @file csi_packet.h
 * @brief VORAX CSI Packet Definition for ESP32-S3 Firmware
 * 
 * Wire format matches core/src/protocol.rs in Rust exactly.
 */

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define VORAX_PACKET_MAGIC 0x5658 // "VX"

/**
 * @brief 18-byte packed header sent before subcarrier data
 */
typedef struct __attribute__((packed)) {
    uint16_t magic;            /**< 0x5658 ("VX") */
    uint16_t node_id;          /**< Unique sensor identifier */
    uint32_t sequence_number;  /**< Monotonic packet sequence counter */
    uint64_t timestamp_us;     /**< Monotonic microsecond clock from esp_timer_get_time() */
    uint8_t  channel;          /**< Wi-Fi channel index (1-14) */
    int8_t   rssi;             /**< Received Signal Strength in dBm */
    int8_t   noise_floor;      /**< Noise floor in dBm */
    uint8_t  subcarrier_count; /**< Number of subcarriers (64 for HT20) */
} vorax_csi_header_t;

/**
 * @brief Per-subcarrier complex sample (2 bytes)
 * Order matches ESP-IDF promiscuous CSI callback buffer
 */
typedef struct __attribute__((packed)) {
    int8_t imaginary;          /**< Imaginary component Q */
    int8_t real;               /**< Real component I */
} vorax_subcarrier_t;

#ifdef __cplusplus
}
#endif
