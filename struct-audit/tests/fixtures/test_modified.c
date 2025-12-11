// Modified test structures for diff testing
// Changes from test_simple.c:
// - NoPadding: added new field
// - InternalPadding: removed field, different layout
// - TailPadding: unchanged
// - NewStruct: added
// - Inner/Outer: removed

// NoPadding - MODIFIED: added field d
struct NoPadding {
    int a;
    int b;
    int c;
    int d;  // NEW FIELD
};

// InternalPadding - MODIFIED: different layout
struct InternalPadding {
    int b;       // 4 bytes (was char a first)
    char a;      // 1 byte
    // 3 bytes padding
    int d;       // 4 bytes
    // removed char c
};

// TailPadding - UNCHANGED
struct TailPadding {
    int a;       // 4 bytes
    char b;      // 1 byte
    // 3 bytes tail padding
};

// WithArray - MODIFIED: larger array
struct WithArray {
    int count;
    char data[20];  // was 10
    int flags;
};

// WithPointer - UNCHANGED
struct WithPointer {
    char tag;
    void *ptr;
    int value;
};

// NEW STRUCT - added
struct NewStruct {
    long id;
    double value;
    char name[16];
};

// Inner/Outer - REMOVED (not present)

int main() {
    struct NoPadding np;
    struct InternalPadding ip;
    struct TailPadding tp;
    struct WithArray wa;
    struct WithPointer wp;
    struct NewStruct ns;

    (void)np;
    (void)ip;
    (void)tp;
    (void)wa;
    (void)wp;
    (void)ns;

    return 0;
}
