/**
 * @file packet_builder.h
 * @brief VORAX Wire Packet Serialization and Validation
 */

#pragma once

#include <stdint.h>
#include <stddef.h>
#include "csi_packet.h"
#include "csi_rx.h"

#ifdef __cplusplus
extern "C" {
#endif

#define VORAX_WIRE_PACKET_SIZE (sizeof(vorax_csi_header_t) + sizeof(vorax_subcarrier_t) * VORAX_MAX_SUBCARRIERS)

/**
 * @brief Serializes a captured CSI frame item into a contiguous wire buffer
 * 
 * @param frame Input frame item dequeued from RX
 * @param out_buf Output buffer (must be at least VORAX_WIRE_PACKET_SIZE bytes)
 * @param buf_len Length of out_buf
 * @return size_t Total bytes serialized (148 bytes on HT20), or 0 on error
 */
size_t vorax_packet_serialize(const vorax_frame_item_t *frame, uint8_t *out_buf, size_t buf_len);

#ifdef __cplusplus
}
#endif
