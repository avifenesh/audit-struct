use crate::error::{Error, Result};
use gimli::{Dwarf, EndianSlice, RunTimeEndian, SectionId};
use memmap2::Mmap;
use object::{Object, ObjectSection};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::pin::Pin;

pub struct BinaryData {
    pub mmap: Mmap,
}

pub type DwarfSlice<'a> = EndianSlice<'a, RunTimeEndian>;

/// Storage for decompressed DWARF sections.
/// Pinned to ensure stable addresses for slices pointing into the data.
pub struct DecompressedSections {
    sections: HashMap<&'static str, Vec<u8>>,
}

impl DecompressedSections {
    fn new() -> Pin<Box<Self>> {
        Box::pin(Self { sections: HashMap::new() })
    }

    fn insert(self: &mut Pin<Box<Self>>, name: &'static str, data: Vec<u8>) {
        // SAFETY: We only modify the HashMap contents, not the Box location.
        // The HashMap's heap allocations (Vec<u8>) have stable addresses.
        unsafe { self.as_mut().get_unchecked_mut() }.sections.insert(name, data);
    }

    fn get(&self, name: &str) -> Option<&[u8]> {
        self.sections.get(name).map(|v| v.as_slice())
    }
}

pub struct LoadedDwarf<'a> {
    pub dwarf: Dwarf<DwarfSlice<'a>>,
    pub address_size: u8,
    pub endian: RunTimeEndian,
    /// Pinned storage for decompressed sections. The Dwarf object holds slices
    /// pointing into this data, so it must remain at a stable address.
    /// Named with underscore prefix to indicate intentional non-use (kept for lifetime).
    _decompressed_sections: Pin<Box<DecompressedSections>>,
}

/// Standard DWARF section names that we need to load.
const DEBUG_SECTIONS: &[&str] = &[
    "abbrev",
    "addr",
    "aranges",
    "info",
    "line",
    "line_str",
    "loc",
    "loclists",
    "ranges",
    "rnglists",
    "str",
    "str_offsets",
    "types",
];

impl BinaryData {
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        // SAFETY: The file is opened read-only and we keep the mmap alive
        // for the lifetime of BinaryData.
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(Self { mmap })
    }

    pub fn load_dwarf(&self) -> Result<LoadedDwarf<'_>> {
        let object = object::File::parse(&*self.mmap)?;

        if !matches!(
            object.format(),
            object::BinaryFormat::Elf | object::BinaryFormat::MachO | object::BinaryFormat::Pe
        ) {
            return Err(Error::UnsupportedFormat);
        }

        let endian =
            if object.is_little_endian() { RunTimeEndian::Little } else { RunTimeEndian::Big };

        // Create pinned storage for decompressed sections
        let mut decompressed_sections = DecompressedSections::new();

        // Pre-decompress any compressed sections and store them
        for &base_name in DEBUG_SECTIONS {
            let debug_name = format!(".debug_{}", base_name);
            let zdebug_name = format!(".zdebug_{}", base_name);

            // Try .debug_* first, then .zdebug_*
            for name in [&debug_name, &zdebug_name] {
                if let Some(section) = object.section_by_name(name) {
                    if let Ok(Cow::Owned(vec)) = section.uncompressed_data() {
                        // Leak the string to get a 'static lifetime - this is fine since
                        // these are a fixed set of section names used for the program lifetime
                        let static_name: &'static str = match name.as_str() {
                            n if n == debug_name => leak_section_name(&debug_name),
                            _ => leak_section_name(&zdebug_name),
                        };
                        decompressed_sections.insert(static_name, vec);
                    }
                }
            }
        }

        // Create a raw pointer to the pinned storage for use in the closure.
        // SAFETY: The Pin<Box<DecompressedSections>> ensures the data won't move.
        // We only read from it in the closure, and the LoadedDwarf keeps it alive.
        let decompressed_ptr = &*decompressed_sections as *const DecompressedSections;

        let load_section = |id: SectionId| -> std::result::Result<DwarfSlice<'_>, gimli::Error> {
            let section_name = id.name();
            let zdebug_name = section_name.replace(".debug_", ".zdebug_");

            let try_load = |name: &str| -> Option<&[u8]> {
                // SAFETY: decompressed_ptr points to pinned data that outlives this closure
                let decompressed = unsafe { &*decompressed_ptr };
                if let Some(slice) = decompressed.get(name) {
                    return Some(slice);
                }

                // Fall back to borrowing directly from mmap for uncompressed sections
                object.section_by_name(name).and_then(|s| s.uncompressed_data().ok()).and_then(
                    |data| match data {
                        Cow::Borrowed(b) => Some(b),
                        Cow::Owned(_) => None,
                    },
                )
            };

            let slice = try_load(section_name).or_else(|| try_load(&zdebug_name)).unwrap_or(&[]);

            Ok(EndianSlice::new(slice, endian))
        };

        let dwarf = Dwarf::load(load_section).map_err(|e| Error::Dwarf(e.to_string()))?;

        let mut units = dwarf.units();
        if units.next().map_err(|e| Error::Dwarf(e.to_string()))?.is_none() {
            return Err(Error::NoDebugInfo);
        }

        Ok(LoadedDwarf {
            dwarf,
            address_size: if object.is_64() { 8 } else { 4 },
            endian,
            _decompressed_sections: decompressed_sections,
        })
    }
}

/// Leak a section name string to get a 'static lifetime.
/// This is acceptable because we only call this for a fixed set of ~26 section names.
fn leak_section_name(name: &str) -> &'static str {
    // Use a simple cache to avoid leaking duplicates
    use std::sync::OnceLock;
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    // Recover from poisoned lock - cache is just an optimization, safe to continue
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(&cached) = guard.get(name) {
        return cached;
    }

    let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
    guard.insert(name.to_string(), leaked);
    leaked
}
