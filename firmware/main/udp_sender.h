/**
 * @file udp_sender.h
 * @brief High-Speed UDP Multicast Task pinned to Core 1
 */

#pragma once

#include <stdint.h>
#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Starts the UDP streaming task on Core 1
 * 
 * @param target_ip IPv4 destination address (e.g. "255.255.255.255" or "192.168.1.100")
 * @param port UDP destination port (VORAX default is 5555)
 * @return esp_err_t ESP_OK on success
 */
esp_err_t vorax_udp_sender_start(const char *target_ip, uint16_t port);

/**
 * @brief Queries cumulative transmission metrics
 */
void vorax_udp_sender_get_stats(uint32_t *out_sent, uint32_t *out_errors);

#ifdef __cplusplus
}
#endif
