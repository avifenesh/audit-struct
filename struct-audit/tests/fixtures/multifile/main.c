// main.c - Test binary entry point
// Instantiates all structs to ensure they appear in DWARF

#include <stdio.h>
#include <string.h>
#include "types.h"
#include "order.h"
#include "market_data.h"
#include "network.h"

// Prevent optimizer from removing unused structs
#define USE(x) do { volatile void* p = &(x); (void)p; } while(0)

int main(void) {
    // types.h structs
    struct Symbol sym = {0};
    struct Timestamp ts = {0};
    struct Price price = {0};
    struct Quantity qty = {0};

    // order.h structs
    struct HotOrder hot_order = {0};
    struct Order order = {0};
    struct PoorlyAlignedOrder bad_order = {0};
    struct OrderExecution exec = {0};

    // market_data.h structs
    struct Quote quote = {0};
    struct PriceLevel level = {0};
    struct OrderBook book = {0};
    struct Trade trade = {0};
    struct Bar bar = {0};
    struct MarketDataMessage msg = {0};
    struct MarketDataEnvelope env = {0};

    // network.h structs
    struct PackedHeader packed = {0};
    struct UnpackedHeader unpacked = {0};
    struct Connection conn = {0};
    struct RingBuffer rb = {0};

    // Use all variables to prevent optimization
    USE(sym);
    USE(ts);
    USE(price);
    USE(qty);
    USE(hot_order);
    USE(order);
    USE(bad_order);
    USE(exec);
    USE(quote);
    USE(level);
    USE(book);
    USE(trade);
    USE(bar);
    USE(msg);
    USE(env);
    USE(packed);
    USE(unpacked);
    USE(conn);
    USE(rb);

    // Print sizes for verification
    printf("Struct sizes:\n");
    printf("  Symbol: %zu\n", sizeof(struct Symbol));
    printf("  Timestamp: %zu\n", sizeof(struct Timestamp));
    printf("  Price: %zu\n", sizeof(struct Price));
    printf("  Quantity: %zu\n", sizeof(struct Quantity));
    printf("  HotOrder: %zu (target: 64 for cache line)\n", sizeof(struct HotOrder));
    printf("  Order: %zu\n", sizeof(struct Order));
    printf("  PoorlyAlignedOrder: %zu\n", sizeof(struct PoorlyAlignedOrder));
    printf("  OrderExecution: %zu\n", sizeof(struct OrderExecution));
    printf("  Quote: %zu\n", sizeof(struct Quote));
    printf("  PriceLevel: %zu\n", sizeof(struct PriceLevel));
    printf("  OrderBook: %zu\n", sizeof(struct OrderBook));
    printf("  Trade: %zu\n", sizeof(struct Trade));
    printf("  Bar: %zu\n", sizeof(struct Bar));
    printf("  MarketDataMessage: %zu\n", sizeof(struct MarketDataMessage));
    printf("  MarketDataEnvelope: %zu\n", sizeof(struct MarketDataEnvelope));
    printf("  PackedHeader: %zu\n", sizeof(struct PackedHeader));
    printf("  UnpackedHeader: %zu\n", sizeof(struct UnpackedHeader));
    printf("  Connection: %zu\n", sizeof(struct Connection));
    printf("  RingBuffer: %zu\n", sizeof(struct RingBuffer));

    return 0;
}
