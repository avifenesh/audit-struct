use crate::error::{Error, Result};
use gimli::{Dwarf, EndianSlice, RunTimeEndian, SectionId};
use memmap2::Mmap;
use object::{Object, ObjectSection};
use std::borrow::Cow;
use std::fs::File;
use std::path::Path;

pub struct BinaryData {
    pub mmap: Mmap,
}

pub type DwarfSlice<'a> = EndianSlice<'a, RunTimeEndian>;

pub struct LoadedDwarf<'a> {
    pub dwarf: Dwarf<DwarfSlice<'a>>,
    pub address_size: u8,
}

impl BinaryData {
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        Ok(Self { mmap })
    }

    pub fn load_dwarf(&self) -> Result<LoadedDwarf<'_>> {
        let object = object::File::parse(&*self.mmap)?;

        if !matches!(object.format(), object::BinaryFormat::Elf | object::BinaryFormat::MachO) {
            return Err(Error::UnsupportedFormat);
        }

        let endian =
            if object.is_little_endian() { RunTimeEndian::Little } else { RunTimeEndian::Big };

        let load_section = |id: SectionId| -> std::result::Result<DwarfSlice<'_>, gimli::Error> {
            let data = object
                .section_by_name(id.name())
                .and_then(|s| s.uncompressed_data().ok())
                .unwrap_or(Cow::Borrowed(&[]));

            // Convert Cow to slice - we know this is safe because we keep mmap alive
            let slice: &[u8] = match &data {
                Cow::Borrowed(b) => b,
                Cow::Owned(_) => {
                    // For compressed sections, we'd need different handling
                    // For now, skip them
                    &[]
                }
            };

            Ok(EndianSlice::new(slice, endian))
        };

        let dwarf = Dwarf::load(load_section).map_err(|e| Error::Dwarf(e.to_string()))?;

        // Check if we actually have debug info
        let mut units = dwarf.units();
        if units.next().map_err(|e| Error::Dwarf(e.to_string()))?.is_none() {
            return Err(Error::NoDebugInfo);
        }

        Ok(LoadedDwarf { dwarf, address_size: if object.is_64() { 8 } else { 4 } })
    }
}
