/**
 * @file packet_builder.c
 * @brief Implementation of VORAX Binary Wire Format Serialization
 */

#include "packet_builder.h"
#include <string.h>

size_t vorax_packet_serialize(const vorax_frame_item_t *frame, uint8_t *out_buf, size_t buf_len) {
    if (!frame || !out_buf || buf_len < VORAX_WIRE_PACKET_SIZE) {
        return 0;
    }

    // 1. Copy 20-byte packed header
    memcpy(out_buf, &frame->header, sizeof(vorax_csi_header_t));

    // 2. Copy 64 interleaved subcarrier pairs (128 bytes)
    memcpy(out_buf + sizeof(vorax_csi_header_t), 
           frame->subcarriers, 
           sizeof(vorax_subcarrier_t) * VORAX_MAX_SUBCARRIERS);

    return VORAX_WIRE_PACKET_SIZE;
}
