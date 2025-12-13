// order.h - Order management structures
#ifndef ORDER_H
#define ORDER_H

#include "types.h"

// Hot path order struct - should be cache-line optimized
// Target: fit in single 64-byte cache line
struct HotOrder {
    uint64_t order_id;          // 8 bytes, offset 0
    struct Price price;         // 16 bytes, offset 8
    struct Quantity quantity;   // 16 bytes, offset 24
    struct Symbol symbol;       // 16 bytes, offset 40
    OrderSide side;             // 4 bytes, offset 56
    OrderType type;             // 4 bytes, offset 60
    // Total: 64 bytes - exactly one cache line!
};

// Full order with all metadata - not cache-line critical
struct Order {
    uint64_t order_id;
    uint64_t client_order_id;
    uint64_t account_id;
    struct Symbol symbol;
    struct Price price;
    struct Price stop_price;
    struct Quantity quantity;
    struct Quantity filled_quantity;
    struct Quantity remaining_quantity;
    struct Timestamp created_at;
    struct Timestamp updated_at;
    OrderSide side;
    OrderType type;
    OrderStatus status;
    bool is_hidden;
    bool is_post_only;
    bool is_reduce_only;
    uint8_t _padding[1];
};

// Order with poor layout - for testing padding detection
struct PoorlyAlignedOrder {
    char tag;                   // 1 byte
    // 7 bytes padding
    uint64_t id;                // 8 bytes
    char status;                // 1 byte
    // 7 bytes padding
    double price;               // 8 bytes
    char side;                  // 1 byte
    // 3 bytes padding
    int32_t quantity;           // 4 bytes
    char type;                  // 1 byte
    // 7 bytes padding
    uint64_t timestamp;         // 8 bytes
};

// Nested struct example
struct OrderExecution {
    struct Order order;
    char notes[256];
    struct Timestamp execution_time;
    uint64_t execution_id;
    struct Price execution_price;
    struct Quantity executed_quantity;
};

#endif // ORDER_H
