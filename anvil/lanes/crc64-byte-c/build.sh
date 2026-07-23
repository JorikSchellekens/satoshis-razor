#!/usr/bin/env bash
# Build the C lane for this machine. The lane's native artifact sits next
# to its lane.json; the harness runs it through the external-lane protocol.
set -euo pipefail
cd "$(dirname "$0")"
cc -O3 -o crc64-byte-c crc64_byte_c.c
echo "built $(pwd)/crc64-byte-c"
