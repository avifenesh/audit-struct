// market_data.h - Market data structures
#ifndef MARKET_DATA_H
#define MARKET_DATA_H

#include "types.h"

// Level 1 quote - hot path, needs to be small
struct Quote {
    struct Price bid;
    struct Price ask;
    struct Quantity bid_size;
    struct Quantity ask_size;
    struct Timestamp timestamp;
    struct Symbol symbol;
};

// Single price level in order book
struct PriceLevel {
    struct Price price;
    struct Quantity quantity;
    uint32_t order_count;
    // 4 bytes padding
};

// Order book snapshot - array of levels
struct OrderBook {
    struct Symbol symbol;
    struct Timestamp timestamp;
    uint32_t bid_levels_count;
    uint32_t ask_levels_count;
    struct PriceLevel bids[10];
    struct PriceLevel asks[10];
};

// Trade tick
struct Trade {
    uint64_t trade_id;
    struct Symbol symbol;
    struct Price price;
    struct Quantity quantity;
    struct Timestamp timestamp;
    OrderSide aggressor_side;
    // padding
};

// OHLCV bar
struct Bar {
    struct Symbol symbol;
    struct Timestamp open_time;
    struct Timestamp close_time;
    struct Price open;
    struct Price high;
    struct Price low;
    struct Price close;
    struct Quantity volume;
    uint64_t trade_count;
};

// Bitfield example - market data flags
struct MarketDataMessage {
    uint64_t sequence_num;
    struct Symbol symbol;
    struct Price price;
    struct Quantity quantity;
    unsigned int is_bid : 1;
    unsigned int is_snapshot : 1;
    unsigned int is_last : 1;
    unsigned int channel : 5;
    unsigned int _reserved : 24;
};

// Union example for polymorphic messages
union MarketDataPayload {
    struct Quote quote;
    struct Trade trade;
    struct Bar bar;
};

struct MarketDataEnvelope {
    uint8_t message_type;
    // 7 bytes padding
    struct Timestamp received_at;
    union MarketDataPayload payload;
};

#endif // MARKET_DATA_H
