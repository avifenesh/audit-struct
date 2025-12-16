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

//go:noinline
func usePoorlyAligned(p *PoorlyAligned) uint64 {
	return p.bigValue + uint64(p.medium) + uint64(p.small)
}

//go:noinline
func useWellAligned(w *WellAligned) uint64 {
	return w.bigValue + uint64(w.medium) + uint64(w.small)
}

//go:noinline
func useWithSlice(s *WithSlice) int {
	if len(s.items) > 0 {
		return s.count + s.items[0]
	}
	return s.count
}

//go:noinline
func useOuter(o *Outer) int32 {
	return o.inner.x + o.inner.y + int32(o.prefix) + int32(o.suffix)
}

//go:noinline
func useWithPointer(w *WithPointer) int32 {
	if w.ptr != nil {
		return w.value + int32(*w.ptr)
	}
	return w.value
}

//go:noinline
func useFlags(f *Flags) uint32 {
	var v uint32
	if f.a {
		v += 1
	}
	if f.b {
		v += 2
	}
	return v + f.value
}

var globalResult uint64

func main() {
	p := &PoorlyAligned{flag: true, bigValue: 42, small: 1, medium: 100}
	w := &WellAligned{bigValue: 42, medium: 100, small: 1, flag: true}
	s := &WithSlice{count: 5, items: []int{1, 2, 3}}
	o := &Outer{prefix: 'a', inner: Inner{x: 10, y: 20}, suffix: 'z'}
	ptr := &WithPointer{tag: 'x', ptr: new(int), value: 50}
	f := &Flags{a: true, b: false, c: true, d: false, value: 99}

	globalResult = usePoorlyAligned(p) + useWellAligned(w) + uint64(useWithSlice(s)) +
		uint64(useOuter(o)) + uint64(useWithPointer(ptr)) + uint64(useFlags(f))
}
