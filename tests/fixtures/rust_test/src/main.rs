//! Test structs for layout-audit validation
//! Contains various struct patterns to test DWARF parsing

use std::hint::black_box;

// Well-aligned struct - should show 0% padding
#[repr(C)]
pub struct WellAligned {
    pub id: u64,
    pub value: u64,
    pub count: u32,
    pub flags: u32,
}

// Poorly aligned struct - should show padding
#[repr(C)]
pub struct PoorlyAligned {
    pub flag: bool,      // 1 byte
    pub big_value: u64,  // 8 bytes, needs 7 padding
    pub small: u8,       // 1 byte
    pub medium: u32,     // 4 bytes, needs 3 padding
}

// Nested struct
#[repr(C)]
pub struct Inner {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
pub struct Outer {
    pub header: u64,
    pub inner: Inner,
    pub trailer: u64,
}

// Struct with arrays
#[repr(C)]
pub struct WithArrays {
    pub name: [u8; 32],
    pub values: [u32; 8],
    pub flags: u64,
}

// Struct with pointers
#[repr(C)]
pub struct WithPointers {
    pub data: *const u8,
    pub len: usize,
    pub capacity: usize,
}

// Generic struct
#[repr(C)]
pub struct Container<T> {
    pub value: T,
    pub count: u32,
    pub active: bool,
}

// Enum with data (tagged union)
#[repr(C)]
pub enum Message {
    Empty,
    Small(u32),
    Large { data: [u8; 64], len: usize },
}

// Option-like enum
#[repr(C)]
pub enum MaybeValue {
    None,
    Some(u64),
}

// Cache-line optimized struct (64 bytes)
#[repr(C, align(64))]
pub struct CacheAligned {
    pub hot_data: u64,
    pub counter: u64,
    pub flags: u32,
    _padding: [u8; 44],
}

// Packed struct
#[repr(C, packed)]
pub struct Packed {
    pub flag: u8,
    pub value: u64,
    pub small: u16,
}

// Mixed alignment struct
#[repr(C)]
pub struct MixedAlignment {
    pub a: u8,
    pub b: u16,
    pub c: u8,
    pub d: u32,
    pub e: u8,
    pub f: u64,
}

// Large struct spanning multiple cache lines
#[repr(C)]
pub struct LargeStruct {
    pub header: u64,
    pub data: [u8; 128],
    pub checksum: u32,
    pub flags: u32,
    pub trailer: u64,
}

fn main() {
    // Instantiate structs to ensure they're not optimized away
    let well = WellAligned { id: 1, value: 2, count: 3, flags: 4 };
    let poor = PoorlyAligned { flag: true, big_value: 100, small: 1, medium: 50 };
    let outer = Outer { header: 1, inner: Inner { x: 10, y: 20 }, trailer: 2 };
    let arrays = WithArrays { name: [0; 32], values: [0; 8], flags: 0 };
    let ptrs = WithPointers { data: std::ptr::null(), len: 0, capacity: 0 };
    let container: Container<u64> = Container { value: 42, count: 1, active: true };
    let msg = Message::Large { data: [0; 64], len: 64 };
    let maybe = MaybeValue::Some(123);
    let cache = CacheAligned { hot_data: 1, counter: 2, flags: 3, _padding: [0; 44] };
    let packed = Packed { flag: 1, value: 2, small: 3 };
    let mixed = MixedAlignment { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6 };
    let large = LargeStruct { header: 1, data: [0; 128], checksum: 0, flags: 0, trailer: 2 };

    // Use black_box to prevent optimization
    black_box(&well);
    black_box(&poor);
    black_box(&outer);
    black_box(&arrays);
    black_box(&ptrs);
    black_box(&container);
    black_box(&msg);
    black_box(&maybe);
    black_box(&cache);
    black_box(&packed);
    black_box(&mixed);
    black_box(&large);
}
