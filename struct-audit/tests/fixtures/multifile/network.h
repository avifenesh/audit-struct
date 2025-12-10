// network.h - Network protocol structures
#ifndef NETWORK_H
#define NETWORK_H

#include <stdint.h>

// Packed network header - uses pragma pack
#pragma pack(push, 1)
struct PackedHeader {
    uint16_t magic;
    uint16_t version;
    uint32_t length;
    uint64_t sequence;
    uint32_t checksum;
    uint16_t msg_type;
    uint16_t flags;
};
#pragma pack(pop)

// Same struct without packing - for comparison
struct UnpackedHeader {
    uint16_t magic;
    uint16_t version;
    uint32_t length;
    uint64_t sequence;
    uint32_t checksum;
    uint16_t msg_type;
    uint16_t flags;
};

// Connection info with string
struct Connection {
    char hostname[64];
    uint16_t port;
    // padding
    uint32_t timeout_ms;
    uint64_t last_heartbeat;
    uint8_t is_connected;
    uint8_t is_authenticated;
    uint8_t reconnect_count;
    uint8_t _padding[5];
};

// Buffer with flexible array member (C99)
struct Buffer {
    uint32_t capacity;
    uint32_t length;
    uint8_t data[];  // flexible array member
};

// Ring buffer metadata
struct RingBuffer {
    uint64_t head;
    uint64_t tail;
    uint64_t capacity;
    uint64_t mask;
    void* buffer;
    uint8_t is_full;
    // padding
};

#endif // NETWORK_H
