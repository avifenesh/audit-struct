// C++ template test structures for layout-audit
// Tests various template instantiations and C++ specific features

#include <cstdint>
#include <cstddef>

// Simple template with single type parameter
template<typename T>
struct Container {
    T value;
    uint32_t count;
    bool active;
};

// Template with multiple type parameters
template<typename K, typename V>
struct Pair {
    K key;
    V value;
};

// Template with non-type parameter
template<typename T, size_t N>
struct FixedArray {
    T data[N];
    size_t length;
};

// Nested templates
template<typename T>
struct Optional {
    T value;
    bool has_value;
};

template<typename T>
struct Node {
    T data;
    Node<T>* next;
    Node<T>* prev;
};

// Template specialization base
template<typename T>
struct SmartPtr {
    T* ptr;
    size_t ref_count;
};

// Template with inheritance
struct BaseMetrics {
    uint64_t created_at;
    uint64_t updated_at;
};

template<typename T>
struct TrackedValue : public BaseMetrics {
    T value;
    uint32_t version;
};

// Variadic-like with fixed sizes (simulating std::tuple layout)
template<typename T1, typename T2, typename T3>
struct Triple {
    T1 first;
    T2 second;
    T3 third;
};

// Template with alignment
template<typename T>
struct alignas(64) CacheAlignedValue {
    T value;
    char padding[64 - sizeof(T) > 0 ? 64 - sizeof(T) : 1];
};

// Complex nested template
template<typename T>
struct Result {
    union {
        T value;
        uint64_t error_code;
    };
    bool is_ok;
};

// STL-like container layouts
template<typename T>
struct Vector {
    T* data;
    size_t size;
    size_t capacity;
};

template<typename K, typename V>
struct MapEntry {
    K key;
    V value;
    MapEntry<K, V>* left;
    MapEntry<K, V>* right;
    int8_t balance;
};

// String-like with SSO (Small String Optimization)
template<size_t SSO_SIZE = 15>
struct SmallString {
    union {
        char small[SSO_SIZE + 1];
        struct {
            char* ptr;
            size_t size;
            size_t capacity;
        } large;
    };
    bool is_small;
};

// Type traits helper struct
template<typename T>
struct TypeInfo {
    static constexpr size_t size = sizeof(T);
    static constexpr size_t align = alignof(T);
    const char* name;
    size_t runtime_size;
};

// CRTP pattern
template<typename Derived>
struct Countable {
    static size_t instance_count;
    uint64_t id;
};

struct Widget : Countable<Widget> {
    int32_t x;
    int32_t y;
    uint32_t width;
    uint32_t height;
};

class WidgetClass {
public:
    int32_t id;
    int32_t flags;
};

// Template with enum
enum class Status : uint8_t {
    Pending,
    Running,
    Complete,
    Failed
};

// Reference type holder
struct RefHolder {
    int& ref;
};

template<typename T>
struct Task {
    T payload;
    Status status;
    uint32_t priority;
};

// Instantiate templates with various types to generate DWARF entries
int main() {
    // Simple containers
    Container<int32_t> c_int{};
    Container<double> c_double{};
    Container<char*> c_ptr{};

    // Pairs with different layouts
    Pair<int32_t, int32_t> p_int_int{};
    Pair<char, double> p_char_double{};  // Has padding
    Pair<uint64_t, uint8_t> p_u64_u8{};  // Has tail padding

    // Fixed arrays
    FixedArray<int32_t, 4> fa_int4{};
    FixedArray<char, 32> fa_char32{};
    FixedArray<double, 8> fa_double8{};

    // Optional values
    Optional<int64_t> opt_i64{};
    Optional<char> opt_char{};  // Lots of padding

    // Linked list nodes
    Node<int32_t> node_int{};
    Node<Pair<int, int>> node_pair{};

    // Smart pointers
    SmartPtr<int32_t> sp_int{};
    SmartPtr<Node<int32_t>> sp_node{};

    // Tracked values with inheritance
    TrackedValue<int32_t> tv_int{};
    TrackedValue<double> tv_double{};

    // Triples with various padding scenarios
    Triple<char, int32_t, char> t_cic{};  // Internal padding
    Triple<int64_t, int64_t, int64_t> t_iii{};  // No padding
    Triple<char, char, int64_t> t_cci{};  // Padding before third

    // Cache aligned
    CacheAlignedValue<int32_t> cav_int{};
    CacheAlignedValue<uint64_t> cav_u64{};

    // Result type
    Result<int64_t> res_i64{};
    Result<Pair<int, int>> res_pair{};

    // Vector-like
    Vector<int32_t> vec_int{};
    Vector<Pair<char, double>> vec_pair{};

    // Map entries
    MapEntry<int32_t, int32_t> me_int_int{};
    MapEntry<uint64_t, Vector<int32_t>> me_complex{};

    // Small string
    SmallString<15> ss15{};
    SmallString<31> ss31{};

    // Type info
    TypeInfo<int32_t> ti_int{};
    TypeInfo<double> ti_double{};

    // Widget with CRTP
    Widget widget{};
    WidgetClass widget_class{};

    // Tasks
    Task<int32_t> task_int{};
    Task<Vector<char>> task_vec{};

    int ref_value = 7;
    RefHolder ref_holder{ref_value};

    // Prevent optimization
    (void)c_int; (void)c_double; (void)c_ptr;
    (void)p_int_int; (void)p_char_double; (void)p_u64_u8;
    (void)fa_int4; (void)fa_char32; (void)fa_double8;
    (void)opt_i64; (void)opt_char;
    (void)node_int; (void)node_pair;
    (void)sp_int; (void)sp_node;
    (void)tv_int; (void)tv_double;
    (void)t_cic; (void)t_iii; (void)t_cci;
    (void)cav_int; (void)cav_u64;
    (void)res_i64; (void)res_pair;
    (void)vec_int; (void)vec_pair;
    (void)me_int_int; (void)me_complex;
    (void)ss15; (void)ss31;
    (void)ti_int; (void)ti_double;
    (void)widget;
    (void)widget_class;
    (void)task_int; (void)task_vec;
    (void)ref_holder;

    return 0;
}
