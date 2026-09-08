/**
 * @file udp_sender.c
 * @brief High-Throughput UDP Multicast Task Implementation
 */

#include "udp_sender.h"
#include <string.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "csi_rx.h"
#include "packet_builder.h"

static const char *TAG = "vorax_udp";

static uint32_t s_packets_sent = 0;
static uint32_t s_socket_errors = 0;
static char s_target_ip[64] = "255.255.255.255";
static uint16_t s_target_port = 5555;

static void udp_tx_task(void *pvParameters) {
    int sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_IP);
    if (sock < 0) {
        ESP_LOGE(TAG, "Unable to create UDP socket: errno %d", errno);
        vTaskDelete(NULL);
        return;
    }

    // Enable broadcast transmissions
    int broadcast_enable = 1;
    setsockopt(sock, SOL_SOCKET, SO_BROADCAST, &broadcast_enable, sizeof(broadcast_enable));

    struct sockaddr_in dest_addr;
    dest_addr.sin_addr.s_addr = inet_addr(s_target_ip);
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons(s_target_port);

    ESP_LOGI(TAG, "VORAX UDP TX Task running on Core %d -> %s:%u", 
             xPortGetCoreID(), s_target_ip, s_target_port);

    uint8_t wire_buffer[VORAX_WIRE_PACKET_SIZE];
    vorax_frame_item_t frame;

    while (1) {
        // Block until a frame is ready from the Wi-Fi RX callback
        if (vorax_csi_rx_dequeue(&frame, 10)) {
            size_t len = vorax_packet_serialize(&frame, wire_buffer, sizeof(wire_buffer));
            if (len > 0) {
                int err = sendto(sock, wire_buffer, len, 0, (struct sockaddr *)&dest_addr, sizeof(dest_addr));
                if (err < 0) {
                    s_socket_errors++;
                } else {
                    s_packets_sent++;
                }
            }
        }
    }
}

esp_err_t vorax_udp_sender_start(const char *target_ip, uint16_t port) {
    if (target_ip) {
        strncpy(s_target_ip, target_ip, sizeof(s_target_ip) - 1);
    }
    s_target_port = port;

    // Pin transmission task to Core 1 with high priority (5)
    BaseType_t ret = xTaskCreatePinnedToCore(
        udp_tx_task,
        "vorax_udp_tx",
        4096,
        NULL,
        5,
        NULL,
        1
    );

    return (ret == pdPASS) ? ESP_OK : ESP_FAIL;
}

void vorax_udp_sender_get_stats(uint32_t *out_sent, uint32_t *out_errors) {
    if (out_sent) *out_sent = s_packets_sent;
    if (out_errors) *out_errors = s_socket_errors;
}
