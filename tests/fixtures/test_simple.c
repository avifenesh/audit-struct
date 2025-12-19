// Test structures for layout-audit

// No padding - perfectly packed
struct NoPadding {
    int a;
    int b;
    int c;
};

// Internal padding between char and int
struct InternalPadding {
    char a;      // 1 byte
    // 3 bytes padding
    int b;       // 4 bytes
    char c;      // 1 byte
    // 3 bytes padding
    int d;       // 4 bytes
};

// Tail padding only
struct TailPadding {
    int a;       // 4 bytes
    char b;      // 1 byte
    // 3 bytes tail padding for alignment
};

// Nested struct
struct Inner {
    int x;
    int y;
};

struct Outer {
    char prefix;
    // 3 bytes padding
    struct Inner inner;
    char suffix;
    // 3 bytes tail padding
};

// Array member
struct WithArray {
    int count;
    char data[10];
    // 2 bytes padding
    int flags;
};

// Pointer member
struct WithPointer {
    char tag;
    // 7 bytes padding on 64-bit
    void *ptr;
    int value;
    // 4 bytes tail padding on 64-bit
};

// Bitfields (often use DW_AT_data_bit_offset / DW_AT_bit_offset)
struct BitfieldFlags {
    unsigned int a:1;
    unsigned int b:3;
    unsigned int c:28;
};

// Typedefs, enums, qualifiers, and atomics
typedef int MyInt;

enum Color {
    Red,
    Green,
    Blue
};

struct WithQualifiers {
    const int *cp;
    volatile int v;
    int * restrict rp;
};

typedef struct {
    int x;
} AliasStruct;

struct WithTypedef {
    MyInt value;
    AliasStruct inner;
};

struct WithEnum {
    enum Color c;
};

struct WithFuncPtr {
    int (*cb)(int);
};

struct WithAtomic {
    _Atomic int value;
};

struct WithAtomics {
    _Atomic int a;
    _Atomic int b;
};

static int sample_fn(int x) {
    return x + 1;
}

int main() {
    struct NoPadding np;
    struct InternalPadding ip;
    struct TailPadding tp;
    struct Outer outer;
    struct WithArray wa;
    struct WithPointer wp;
    struct BitfieldFlags bf;
    struct WithQualifiers wq;
    struct WithTypedef wt;
    struct WithEnum we;
    struct WithFuncPtr wfp;
    struct WithAtomic wa2;
    struct WithAtomics wa3;

    (void)np;
    (void)ip;
    (void)tp;
    (void)outer;
    (void)wa;
    (void)wp;
    (void)bf;
    (void)wq;
    (void)wt;
    (void)we;
    (void)wfp;
    (void)wa2;
    (void)wa3;
    (void)sample_fn;

    return 0;
}
