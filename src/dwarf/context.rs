use crate::error::{Error, Result};
use crate::loader::{DwarfSlice, LoadedDwarf};
use crate::types::{MemberLayout, SourceLocation, StructLayout};
use gimli::{AttributeValue, DebuggingInformationEntry, Dwarf, Unit};

use super::TypeResolver;
use super::expr::{evaluate_member_offset, try_simple_offset};
use super::{debug_info_ref_to_unit_offset, read_u64_from_attr};

/// Check if a type name is a Go runtime internal type that should be filtered.
/// These are compiler/runtime-generated types not useful for layout analysis.
pub fn is_go_internal_type(name: &str) -> bool {
    // Go runtime and standard library internals
    name.starts_with("runtime.")
        || name.starts_with("runtime/")
        || name.starts_with("internal/")
        || name.starts_with("reflect.")
        || name.starts_with("sync.")
        || name.starts_with("sync/")
        || name.starts_with("syscall.")
        || name.starts_with("unsafe.")
        // Go internal symbol separator (middle dot)
        || name.contains('\u{00B7}')
        // Runtime type descriptors
        || name.starts_with("type:")
        || name.starts_with("type..")
        // Go map/channel internal types
        || name.starts_with("hash<")
        || name.starts_with("bucket<")
        || name.starts_with("hmap")
        || name.starts_with("hchan")
        || name.starts_with("waitq")
        || name.starts_with("sudog")
        // Goroutine internals
        || name == "g"
        || name == "m"
        || name == "p"
        || name.starts_with("stack")
}

pub struct DwarfContext<'a> {
    dwarf: &'a Dwarf<DwarfSlice<'a>>,
    address_size: u8,
    endian: gimli::RunTimeEndian,
}

impl<'a> DwarfContext<'a> {
    pub fn new(loaded: &'a LoadedDwarf<'a>) -> Self {
        Self { dwarf: &loaded.dwarf, address_size: loaded.address_size, endian: loaded.endian }
    }

    /// Find all structs in the binary.
    ///
    /// - `filter`: Optional substring filter for struct names
    /// - `include_go_runtime`: If false, Go runtime internal types are filtered out
    pub fn find_structs(
        &self,
        filter: Option<&str>,
        include_go_runtime: bool,
    ) -> Result<Vec<StructLayout>> {
        let mut structs = Vec::new();
        let mut units = self.dwarf.units();

        while let Some(header) =
            units.next().map_err(|e| Error::Dwarf(format!("Failed to read unit header: {}", e)))?
        {
            let unit = self
                .dwarf
                .unit(header)
                .map_err(|e| Error::Dwarf(format!("Failed to parse unit: {}", e)))?;

            self.process_unit(&unit, filter, include_go_runtime, &mut structs)?;
        }

        // DWARF can contain duplicate identical type entries (e.g., across units or due to
        // language/compiler quirks). Deduplicate exact duplicates to avoid double-counting in
        // `check` and unstable matching in `diff`.
        // Use enumerated index as tiebreaker for stable deduplication (Rust's sort_by is unstable).
        let mut with_fp: Vec<(StructFingerprint, usize, StructLayout)> =
            structs.into_iter().enumerate().map(|(i, s)| (struct_fingerprint(&s), i, s)).collect();
        with_fp.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        with_fp.dedup_by(|a, b| a.0 == b.0);

        Ok(with_fp.into_iter().map(|(_, _, s)| s).collect())
    }

    fn process_unit(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        filter: Option<&str>,
        include_go_runtime: bool,
        structs: &mut Vec<StructLayout>,
    ) -> Result<()> {
        let mut type_resolver = TypeResolver::new(self.dwarf, unit, self.address_size);
        let mut entries = unit.entries();

        while let Some((_, entry)) =
            entries.next_dfs().map_err(|e| Error::Dwarf(format!("Failed to read DIE: {}", e)))?
        {
            if !matches!(entry.tag(), gimli::DW_TAG_structure_type | gimli::DW_TAG_class_type) {
                continue;
            }

            if let Some(layout) = self.process_struct_entry(
                unit,
                entry,
                filter,
                include_go_runtime,
                &mut type_resolver,
            )? {
                structs.push(layout);
            }
        }

        Ok(())
    }

    fn process_struct_entry(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
        filter: Option<&str>,
        include_go_runtime: bool,
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<Option<StructLayout>> {
        // Use consolidated helper for attribute extraction (see read_u64_from_attr).
        let Some(size) =
            read_u64_from_attr(entry.attr_value(gimli::DW_AT_byte_size).ok().flatten())
        else {
            return Ok(None); // Forward declaration or no size
        };

        let name = self.get_die_name(unit, entry)?;
        let name = match name {
            Some(n) if !n.starts_with("__") => n, // Skip compiler-generated
            None => return Ok(None),              // Anonymous struct
            _ => return Ok(None),
        };

        // Filter Go runtime internal types unless explicitly included
        if !include_go_runtime && is_go_internal_type(&name) {
            return Ok(None);
        }

        if filter.is_some_and(|f| !name.contains(f)) {
            return Ok(None);
        }

        let alignment = read_u64_from_attr(entry.attr_value(gimli::DW_AT_alignment).ok().flatten());

        let mut layout = StructLayout::new(name, size, alignment);
        layout.source_location = self.get_source_location(unit, entry)?;
        layout.members = self.extract_members(unit, entry, type_resolver)?;

        Ok(Some(layout))
    }

    fn extract_members(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        struct_entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<Vec<MemberLayout>> {
        let mut members = Vec::new();
        let mut tree = unit
            .entries_tree(Some(struct_entry.offset()))
            .map_err(|e| Error::Dwarf(format!("Failed to create entries tree: {}", e)))?;

        let root =
            tree.root().map_err(|e| Error::Dwarf(format!("Failed to get tree root: {}", e)))?;

        let mut children = root.children();
        while let Some(child) = children
            .next()
            .map_err(|e| Error::Dwarf(format!("Failed to iterate children: {}", e)))?
        {
            let entry = child.entry();
            match entry.tag() {
                gimli::DW_TAG_member => {
                    if let Some(member) = self.process_member(unit, entry, type_resolver)? {
                        members.push(member);
                    }
                }
                gimli::DW_TAG_inheritance => {
                    if let Some(member) = self.process_inheritance(unit, entry, type_resolver)? {
                        members.push(member);
                    }
                }
                _ => {}
            }
        }

        members.sort_by_key(|m| m.offset.unwrap_or(u64::MAX));
        Ok(members)
    }

    /// Resolve type information from a DW_AT_type attribute.
    /// Returns (type_name, size, is_atomic) or a default for unknown types.
    fn resolve_type_attr(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<(String, Option<u64>, bool)> {
        match entry.attr_value(gimli::DW_AT_type) {
            Ok(Some(AttributeValue::UnitRef(type_offset))) => {
                type_resolver.resolve_type(type_offset)
            }
            Ok(Some(AttributeValue::DebugInfoRef(debug_info_offset))) => {
                // Use shared helper for cross-unit reference conversion.
                if let Some(unit_offset) =
                    debug_info_ref_to_unit_offset(debug_info_offset, &unit.header)
                {
                    type_resolver.resolve_type(unit_offset)
                } else {
                    Ok(("unknown".to_string(), None, false))
                }
            }
            _ => Ok(("unknown".to_string(), None, false)),
        }
    }

    fn process_inheritance(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<Option<MemberLayout>> {
        let offset = self.get_member_offset(unit, entry)?;
        let (type_name, size, is_atomic) = self.resolve_type_attr(unit, entry, type_resolver)?;

        let name = format!("<base: {}>", type_name);
        Ok(Some(MemberLayout::new(name, type_name, offset, size).with_atomic(is_atomic)))
    }

    fn process_member(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<Option<MemberLayout>> {
        let name = self.get_die_name(unit, entry)?.unwrap_or_else(|| "<anonymous>".to_string());
        let (type_name, size, is_atomic) = self.resolve_type_attr(unit, entry, type_resolver)?;

        let offset = self.get_member_offset(unit, entry)?;

        let mut member = MemberLayout::new(name, type_name, offset, size).with_atomic(is_atomic);

        let bit_size = read_u64_from_attr(entry.attr_value(gimli::DW_AT_bit_size).ok().flatten());
        let dwarf5_data_bit_offset =
            read_u64_from_attr(entry.attr_value(gimli::DW_AT_data_bit_offset).ok().flatten());
        let dwarf4_bit_offset =
            read_u64_from_attr(entry.attr_value(gimli::DW_AT_bit_offset).ok().flatten());

        member.bit_size = bit_size;

        if let Some(bit_size) = bit_size
            && let Some(storage_bytes) = member.size
            && storage_bytes > 0
        {
            let storage_bits = storage_bytes.saturating_mul(8);

            // Determine the containing storage unit byte offset for this bitfield.
            // Prefer DW_AT_data_member_location when present. If absent, infer the
            // storage unit start by aligning the absolute DW_AT_data_bit_offset down
            // to the storage unit size.
            let container_offset = member.offset.or_else(|| {
                let data_bit_offset = dwarf5_data_bit_offset?;
                let start_byte = data_bit_offset.checked_div(8)?;
                // storage_bytes > 0 is guaranteed by the outer if-let guard
                start_byte.checked_div(storage_bytes)?.checked_mul(storage_bytes)
            });

            if member.offset.is_none() {
                member.offset = container_offset;
            }

            // Compute bit offset within the containing storage unit.
            if let Some(container_offset) = container_offset {
                if let Some(data_bit_offset) = dwarf5_data_bit_offset {
                    // Use checked_mul to avoid overflow for large container offsets.
                    if let Some(container_bits) = container_offset.checked_mul(8) {
                        member.bit_offset = Some(data_bit_offset.saturating_sub(container_bits));
                    }
                } else if let Some(raw_bit_offset) = dwarf4_bit_offset {
                    // Use checked_add to avoid overflow in boundary check.
                    if let Some(end_bit) = raw_bit_offset.checked_add(bit_size) {
                        if end_bit <= storage_bits {
                            let bit_offset = match self.endian {
                                gimli::RunTimeEndian::Little => {
                                    storage_bits - raw_bit_offset - bit_size
                                }
                                gimli::RunTimeEndian::Big => raw_bit_offset,
                            };
                            member.bit_offset = Some(bit_offset);
                        }
                    }
                }
            }
        }

        Ok(Some(member))
    }

    fn get_member_offset(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<u64>> {
        match entry.attr_value(gimli::DW_AT_data_member_location) {
            Ok(Some(AttributeValue::Udata(offset))) => Ok(Some(offset)),
            Ok(Some(AttributeValue::Data1(offset))) => Ok(Some(offset as u64)),
            Ok(Some(AttributeValue::Data2(offset))) => Ok(Some(offset as u64)),
            Ok(Some(AttributeValue::Data4(offset))) => Ok(Some(offset as u64)),
            Ok(Some(AttributeValue::Data8(offset))) => Ok(Some(offset)),
            Ok(Some(AttributeValue::Sdata(offset))) if offset >= 0 => Ok(Some(offset as u64)),
            Ok(Some(AttributeValue::Exprloc(expr))) => {
                // Try simple constant extraction first (fast path)
                if let Some(offset) = try_simple_offset(expr, unit.encoding()) {
                    return Ok(Some(offset));
                }
                // Fall back to full expression evaluation
                evaluate_member_offset(expr, unit.encoding())
            }
            Ok(None) => Ok(None), // Missing offset - don't assume 0 (bitfields, packed structs)
            _ => Ok(None),
        }
    }

    fn get_die_name(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<String>> {
        match entry.attr_value(gimli::DW_AT_name) {
            Ok(Some(attr)) => {
                let name = self
                    .dwarf
                    .attr_string(unit, attr)
                    .map_err(|e| Error::Dwarf(format!("Failed to read name: {}", e)))?;
                Ok(Some(name.to_string_lossy().into_owned()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Dwarf(format!("Failed to read name attribute: {}", e))),
        }
    }

    fn get_source_location(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<SourceLocation>> {
        let Some(file_index) =
            read_u64_from_attr(entry.attr_value(gimli::DW_AT_decl_file).ok().flatten())
        else {
            return Ok(None);
        };
        let Some(line) =
            read_u64_from_attr(entry.attr_value(gimli::DW_AT_decl_line).ok().flatten())
        else {
            return Ok(None);
        };

        // Try to resolve the file name from the line program header
        let file_name = self.resolve_file_name(unit, file_index).unwrap_or_else(|| {
            // Fall back to file index if resolution fails
            format!("file#{}", file_index)
        });

        Ok(Some(SourceLocation { file: file_name, line }))
    }

    /// Resolve a file index to an actual file path using the .debug_line section.
    fn resolve_file_name(&self, unit: &Unit<DwarfSlice<'a>>, file_index: u64) -> Option<String> {
        // Get the line program for this unit (borrow instead of clone for efficiency)
        let line_program = unit.line_program.as_ref()?;

        let header = line_program.header();

        // File indices in DWARF are 1-based (0 means no file in DWARF 4, or the compilation
        // directory in DWARF 5). We need to handle both cases.
        let file = header.file(file_index)?;

        // Get the file name
        let file_name =
            self.dwarf.attr_string(unit, file.path_name()).ok()?.to_string_lossy().into_owned();

        // Try to get the directory
        if let Some(dir) = file.directory(header) {
            if let Ok(dir_str) = self.dwarf.attr_string(unit, dir) {
                let dir_name = dir_str.to_string_lossy();
                if !dir_name.is_empty() {
                    // Combine directory and file name
                    return Some(format!("{}/{}", dir_name, file_name));
                }
            }
        }

        Some(file_name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct StructFingerprint {
    name: String,
    size: u64,
    alignment: Option<u64>,
    source: Option<(String, u64)>,
    members: Vec<MemberFingerprint>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MemberFingerprint {
    name: String,
    type_name: String,
    offset: Option<u64>,
    size: Option<u64>,
    bit_offset: Option<u64>,
    bit_size: Option<u64>,
    is_atomic: bool,
}

fn struct_fingerprint(s: &StructLayout) -> StructFingerprint {
    StructFingerprint {
        name: s.name.clone(),
        size: s.size,
        alignment: s.alignment,
        source: s.source_location.as_ref().map(|l| (l.file.clone(), l.line)),
        members: s
            .members
            .iter()
            .map(|m| MemberFingerprint {
                name: m.name.clone(),
                type_name: m.type_name.clone(),
                offset: m.offset,
                size: m.size,
                bit_offset: m.bit_offset,
                bit_size: m.bit_size,
                is_atomic: m.is_atomic,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_go_internal_type() {
        // Should be filtered
        assert!(is_go_internal_type("runtime.g"));
        assert!(is_go_internal_type("runtime.m"));
        assert!(is_go_internal_type("runtime.stack"));
        assert!(is_go_internal_type("runtime/internal/atomic.Uint64"));
        assert!(is_go_internal_type("internal/abi.Type"));
        assert!(is_go_internal_type("reflect.Value"));
        assert!(is_go_internal_type("sync.Mutex"));
        assert!(is_go_internal_type("sync/atomic.Int64"));
        assert!(is_go_internal_type("syscall.Stat_t"));
        assert!(is_go_internal_type("unsafe.Pointer"));
        assert!(is_go_internal_type("type:main.MyStruct"));
        assert!(is_go_internal_type("type..hash.main.MyStruct"));
        assert!(is_go_internal_type("hmap"));
        assert!(is_go_internal_type("hchan"));
        assert!(is_go_internal_type("g"));
        assert!(is_go_internal_type("m"));
        assert!(is_go_internal_type("p"));
        assert!(is_go_internal_type("stackObject"));

        // Should NOT be filtered (user types)
        assert!(!is_go_internal_type("main.Order"));
        assert!(!is_go_internal_type("main.Config"));
        assert!(!is_go_internal_type("mypackage.MyStruct"));
        assert!(!is_go_internal_type("github.com/user/pkg.Type"));
        assert!(!is_go_internal_type("Order"));
        assert!(!is_go_internal_type("Config"));
    }
}
