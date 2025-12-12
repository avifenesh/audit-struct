use crate::error::{Error, Result};
use gimli::{Dwarf, EndianSlice, RunTimeEndian, SectionId};
use memmap2::Mmap;
use object::{Object, ObjectSection};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

pub struct BinaryData {
    pub mmap: Mmap,
}

pub type DwarfSlice<'a> = EndianSlice<'a, RunTimeEndian>;

pub struct LoadedDwarf<'a> {
    pub dwarf: Dwarf<DwarfSlice<'a>>,
    pub address_size: u8,
    #[allow(dead_code)]
    decompressed_sections: HashMap<&'static str, Vec<u8>>,
}

impl BinaryData {
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
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

        let mut decompressed_sections: HashMap<&'static str, Vec<u8>> = HashMap::new();

        let section_names: &[&'static str] = &[
            ".debug_abbrev",
            ".debug_addr",
            ".debug_aranges",
            ".debug_info",
            ".debug_line",
            ".debug_line_str",
            ".debug_loc",
            ".debug_loclists",
            ".debug_ranges",
            ".debug_rnglists",
            ".debug_str",
            ".debug_str_offsets",
            ".debug_types",
            ".zdebug_abbrev",
            ".zdebug_addr",
            ".zdebug_aranges",
            ".zdebug_info",
            ".zdebug_line",
            ".zdebug_line_str",
            ".zdebug_loc",
            ".zdebug_loclists",
            ".zdebug_ranges",
            ".zdebug_rnglists",
            ".zdebug_str",
            ".zdebug_str_offsets",
            ".zdebug_types",
        ];

        for &name in section_names {
            if let Some(section) = object.section_by_name(name) {
                if let Ok(Cow::Owned(vec)) = section.uncompressed_data() {
                    decompressed_sections.insert(name, vec);
                }
            }
        }

        // SAFETY: We need raw pointer here because:
        // 1. load_section closure returns slices pointing into decompressed_sections
        // 2. These slices are stored in Dwarf by Dwarf::load
        // 3. We then move decompressed_sections into LoadedDwarf
        // 4. Vec heap data doesn't move when HashMap is moved, so slices remain valid
        // 5. LoadedDwarf keeps decompressed_sections alive for dwarf's lifetime
        let decompressed_ptr = &decompressed_sections as *const HashMap<&'static str, Vec<u8>>;

        let load_section = |id: SectionId| -> std::result::Result<DwarfSlice<'_>, gimli::Error> {
            let section_name = id.name();
            let zdebug_name = section_name.replace(".debug_", ".zdebug_");

            let try_load = |name: &str| -> Option<&[u8]> {
                let decompressed = unsafe { &*decompressed_ptr };
                if let Some(vec) = decompressed.get(name) {
                    return Some(vec.as_slice());
                }

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
            decompressed_sections,
        })
    }
}
