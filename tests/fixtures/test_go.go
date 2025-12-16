package main

// PoorlyAligned has internal padding due to field ordering
type PoorlyAligned struct {
	flag     bool   // 1 byte + 7 padding
	bigValue uint64 // 8 bytes
	small    uint8  // 1 byte + 3 padding
	medium   uint32 // 4 bytes
} // Total: 24 bytes

// WellAligned is optimally ordered
type WellAligned struct {
	bigValue uint64 // 8 bytes
	medium   uint32 // 4 bytes
	small    uint8  // 1 byte
	flag     bool   // 1 byte + 2 tail padding
} // Total: 16 bytes

// WithSlice contains a slice (24-byte header on 64-bit)
type WithSlice struct {
	count int
	items []int
}

// Inner is a simple nested struct
type Inner struct {
	x, y int32
}

// Outer has padding around nested struct
type Outer struct {
	prefix byte
	inner  Inner
	suffix byte
}

// WithPointer has pointer alignment issues
type WithPointer struct {
	tag   byte
	ptr   *int
	value int32
}

// BitFields simulates packed boolean flags
type Flags struct {
	a, b, c, d bool
	value      uint32
}

func main() {
	_ = PoorlyAligned{}
	_ = WellAligned{}
	_ = WithSlice{}
	_ = Outer{}
	_ = WithPointer{}
	_ = Flags{}
}
