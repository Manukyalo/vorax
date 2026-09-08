#!/usr/bin/env bash
# VORAX ESP32-S3 Flash & Monitor Automation Script (Linux/macOS)

set -e

PORT=${1:-/dev/ttyUSB0}
BAUD=${2:-921600}

echo "========================================================"
echo "  VORAX ESP32-S3 ONE-CLICK FLASH SCRIPT"
echo "========================================================"
echo "Target Port: ${PORT}"
echo "Target Baud: ${BAUD}"
echo ""

idf.py -p "${PORT}" -b "${BAUD}" flash monitor
