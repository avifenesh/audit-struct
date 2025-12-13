// types.h - Common type definitions for a trading system
#ifndef TYPES_H
#define TYPES_H

#include <stdint.h>
#include <stdbool.h>

// Enum for order side
typedef enum {
    SIDE_BUY = 0,
    SIDE_SELL = 1
} OrderSide;

// Enum for order type
typedef enum {
    ORDER_MARKET = 0,
    ORDER_LIMIT = 1,
    ORDER_STOP = 2,
    ORDER_STOP_LIMIT = 3
} OrderType;

// Enum for order status
typedef enum {
    STATUS_NEW = 0,
    STATUS_PARTIAL = 1,
    STATUS_FILLED = 2,
    STATUS_CANCELED = 3,
    STATUS_REJECTED = 4
} OrderStatus;

// Small fixed-size string for symbols
struct Symbol {
    char data[16];
};

// Timestamp with nanosecond precision
struct Timestamp {
    int64_t seconds;
    int32_t nanos;
    // 4 bytes padding here
};

// Price with decimal precision
struct Price {
    int64_t mantissa;
    int8_t exponent;
    // 7 bytes padding here
};

// Quantity
struct Quantity {
    uint64_t value;
    uint8_t decimals;
    // 7 bytes padding here
};

#endif // TYPES_H
