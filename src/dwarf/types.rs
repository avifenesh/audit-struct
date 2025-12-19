use crate::error::{Error, Result};
use crate::loader::DwarfSlice;
use gimli::{AttributeValue, Dwarf, Unit, UnitOffset};
use std::collections::HashMap;

use super::{debug_info_ref_to_unit_offset, read_u64_from_attr};

/// Result of resolving a type: (type_name, size, is_atomic)
pub type TypeInfo = (String, Option<u64>, bool);

pub struct TypeResolver<'a, 'b> {
    dwarf: &'b Dwarf<DwarfSlice<'a>>,
    unit: &'b Unit<DwarfSlice<'a>>,
    address_size: u8,
    cache: HashMap<UnitOffset, TypeInfo>,
}

impl<'a, 'b> TypeResolver<'a, 'b> {
    pub fn new(
        dwarf: &'b Dwarf<DwarfSlice<'a>>,
        unit: &'b Unit<DwarfSlice<'a>>,
        address_size: u8,
    ) -> Self {
        Self { dwarf, unit, address_size, cache: HashMap::new() }
    }

    pub fn resolve_type(&mut self, offset: UnitOffset) -> Result<TypeInfo> {
        if let Some(cached) = self.cache.get(&offset) {
            return Ok(cached.clone());
        }

        let result = self.resolve_type_inner(offset, 0, false)?;
        self.cache.insert(offset, result.clone());
        Ok(result)
    }

    fn resolve_type_inner(
        &mut self,
        offset: UnitOffset,
        depth: usize,
        is_atomic: bool,
    ) -> Result<TypeInfo> {
        if depth > 20 {
            return Ok(("...".to_string(), None, is_atomic));
        }

        let entry = self
            .unit
            .entry(offset)
            .map_err(|e| Error::Dwarf(format!("Failed to get type entry: {}", e)))?;

        let tag = entry.tag();

        match tag {
            gimli::DW_TAG_base_type => {
                let name = self.get_type_name(&entry)?.unwrap_or_else(|| "?".to_string());
                let size = self.get_byte_size(&entry)?;
                Ok((name, size, is_atomic))
            }

            gimli::DW_TAG_pointer_type => {
                let pointee = if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (pointee_name, _, _) =
                        self.resolve_type_inner(type_offset, depth + 1, false)?;
                    pointee_name
                } else {
                    "void".to_string()
                };
                Ok((format!("*{}", pointee), Some(self.address_size as u64), is_atomic))
            }

            gimli::DW_TAG_reference_type => {
                let referee = if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (referee_name, _, _) =
                        self.resolve_type_inner(type_offset, depth + 1, false)?;
                    referee_name
                } else {
                    "void".to_string()
                };
                Ok((format!("&{}", referee), Some(self.address_size as u64), is_atomic))
            }

            gimli::DW_TAG_const_type
            | gimli::DW_TAG_volatile_type
            | gimli::DW_TAG_restrict_type => {
                // All three tags are matched in the outer arm, so this is exhaustive.
                let prefix = match tag {
                    gimli::DW_TAG_const_type => "const ",
                    gimli::DW_TAG_volatile_type => "volatile ",
                    _ => "restrict ", // DW_TAG_restrict_type
                };
                if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (inner_name, size, inner_atomic) =
                        self.resolve_type_inner(type_offset, depth + 1, is_atomic)?;
                    Ok((format!("{}{}", prefix, inner_name), size, inner_atomic))
                } else {
                    Ok((format!("{}void", prefix), None, is_atomic))
                }
            }

            gimli::DW_TAG_atomic_type => {
                // Mark as atomic and propagate through the type chain
                if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (inner_name, size, _) =
                        self.resolve_type_inner(type_offset, depth + 1, true)?;
                    Ok((format!("_Atomic {}", inner_name), size, true))
                } else {
                    Ok(("_Atomic void".to_string(), None, true))
                }
            }

            gimli::DW_TAG_typedef => {
                let name = self.get_type_name(&entry)?;
                if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (_, size, inner_atomic) =
                        self.resolve_type_inner(type_offset, depth + 1, is_atomic)?;
                    // Propagate atomic flag through typedefs
                    Ok((
                        name.unwrap_or_else(|| "typedef".to_string()),
                        size,
                        inner_atomic || is_atomic,
                    ))
                } else {
                    Ok((name.unwrap_or_else(|| "typedef".to_string()), None, is_atomic))
                }
            }

            gimli::DW_TAG_array_type => {
                let element_type = if let Some(type_offset) = self.get_type_ref(&entry)? {
                    self.resolve_type_inner(type_offset, depth + 1, is_atomic)?
                } else {
                    ("?".to_string(), None, is_atomic)
                };

                let count = self.get_array_count(&entry)?;
                let size = match (element_type.1, count) {
                    // Use checked_mul to prevent overflow for very large arrays.
                    // Fall back to DW_AT_byte_size if multiplication overflows.
                    (Some(elem_size), Some(c)) => elem_size
                        .checked_mul(c)
                        .or_else(|| self.get_byte_size(&entry).ok().flatten()),
                    _ => self.get_byte_size(&entry)?,
                };

                let count_str = count.map(|c| c.to_string()).unwrap_or_else(|| "?".to_string());
                Ok((format!("[{}; {}]", element_type.0, count_str), size, element_type.2))
            }

            gimli::DW_TAG_structure_type | gimli::DW_TAG_class_type | gimli::DW_TAG_union_type => {
                let name = self.get_type_name(&entry)?.unwrap_or_else(|| "<anonymous>".to_string());
                let size = self.get_byte_size(&entry)?;
                Ok((name, size, is_atomic))
            }

            gimli::DW_TAG_enumeration_type => {
                let name = self.get_type_name(&entry)?.unwrap_or_else(|| "enum".to_string());
                let size = self.get_byte_size(&entry)?;
                Ok((name, size, is_atomic))
            }

            gimli::DW_TAG_subroutine_type => {
                Ok(("fn(...)".to_string(), Some(self.address_size as u64), is_atomic))
            }

            _ => {
                let name = self.get_type_name(&entry)?.unwrap_or_else(|| format!("?<{:?}>", tag));
                let size = self.get_byte_size(&entry)?;
                Ok((name, size, is_atomic))
            }
        }
    }

    fn get_type_name(
        &self,
        entry: &gimli::DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<String>> {
        match entry.attr_value(gimli::DW_AT_name) {
            Ok(Some(attr)) => {
                let name = self
                    .dwarf
                    .attr_string(self.unit, attr)
                    .map_err(|e| Error::Dwarf(format!("Failed to read type name: {}", e)))?;
                Ok(Some(name.to_string_lossy().into_owned()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Dwarf(format!("Failed to read name attr: {}", e))),
        }
    }

    fn get_byte_size(
        &self,
        entry: &gimli::DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<u64>> {
        // Use shared helper for consistent attribute extraction.
        Ok(read_u64_from_attr(entry.attr_value(gimli::DW_AT_byte_size).ok().flatten()))
    }

    fn get_type_ref(
        &self,
        entry: &gimli::DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<UnitOffset>> {
        match entry.attr_value(gimli::DW_AT_type) {
            Ok(Some(AttributeValue::UnitRef(offset))) => Ok(Some(offset)),
            Ok(Some(AttributeValue::DebugInfoRef(debug_info_offset))) => {
                // Use shared helper for cross-unit reference conversion.
                Ok(debug_info_ref_to_unit_offset(debug_info_offset, &self.unit.header))
            }
            _ => Ok(None),
        }
    }

    fn get_array_count(
        &self,
        entry: &gimli::DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<u64>> {
        let mut tree = self
            .unit
            .entries_tree(Some(entry.offset()))
            .map_err(|e| Error::Dwarf(format!("Failed to create tree: {}", e)))?;

        let root = tree.root().map_err(|e| Error::Dwarf(format!("Failed to get root: {}", e)))?;

        let mut children = root.children();
        while let Some(child) =
            children.next().map_err(|e| Error::Dwarf(format!("Failed to iterate: {}", e)))?
        {
            let child_entry = child.entry();
            if child_entry.tag() == gimli::DW_TAG_subrange_type {
                // Try DW_AT_count first (can be various data encodings)
                if let Some(count) = self.extract_count_attr(child_entry, gimli::DW_AT_count)? {
                    return Ok(Some(count));
                }
                // Fall back to DW_AT_upper_bound (0-indexed, so add 1).
                // Use checked_add to handle corrupted DWARF with upper == u64::MAX.
                if let Some(upper) =
                    self.extract_count_attr(child_entry, gimli::DW_AT_upper_bound)?
                {
                    return Ok(upper.checked_add(1));
                }
            }
        }

        Ok(None)
    }

    fn extract_count_attr(
        &self,
        entry: &gimli::DebuggingInformationEntry<DwarfSlice<'a>>,
        attr: gimli::DwAt,
    ) -> Result<Option<u64>> {
        // Use shared helper for consistent attribute extraction.
        Ok(read_u64_from_attr(entry.attr_value(attr).ok().flatten()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::BinaryData;
    use gimli::DwTag;
    use std::path::{Path, PathBuf};

    fn find_fixture_path(name: &str) -> Option<PathBuf> {
        let base = Path::new("tests/fixtures/bin");
        let dsym_path = base.join(format!("{}.dSYM/Contents/Resources/DWARF/{}", name, name));
        if dsym_path.exists() {
            return Some(dsym_path);
        }

        let exe_path = base.join(format!("{}.exe", name));
        if exe_path.exists() {
            return Some(exe_path);
        }

        let direct_path = base.join(name);
        if direct_path.exists() {
            return Some(direct_path);
        }

        None
    }

    fn find_type_offset(unit: &Unit<DwarfSlice<'_>>, tag: DwTag) -> Option<UnitOffset> {
        let mut entries = unit.entries();
        while let Some((_, entry)) = entries.next_dfs().ok().flatten() {
            if entry.tag() == tag {
                return Some(entry.offset());
            }
        }
        None
    }

    #[test]
    fn resolve_common_c_types() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let binary = match BinaryData::load(&path) {
            Ok(b) => b,
            Err(_) => return,
        };
        let loaded = match binary.load_dwarf() {
            Ok(l) => l,
            Err(_) => return,
        };
        let dwarf = &loaded.dwarf;
        let mut units = dwarf.units();
        let header = match units.next() {
            Ok(Some(h)) => h,
            _ => return,
        };
        let unit = match dwarf.unit(header) {
            Ok(u) => u,
            Err(_) => return,
        };

        let mut resolver = TypeResolver::new(&loaded.dwarf, &unit, loaded.address_size);

        let tags = [
            gimli::DW_TAG_base_type,
            gimli::DW_TAG_pointer_type,
            gimli::DW_TAG_const_type,
            gimli::DW_TAG_volatile_type,
            gimli::DW_TAG_restrict_type,
            gimli::DW_TAG_typedef,
            gimli::DW_TAG_array_type,
            gimli::DW_TAG_enumeration_type,
            gimli::DW_TAG_subroutine_type,
            gimli::DW_TAG_atomic_type,
            gimli::DW_TAG_structure_type,
        ];

        for tag in tags {
            if let Some(offset) = find_type_offset(&unit, tag) {
                let _ = resolver.resolve_type(offset).expect("resolve type");
            }
        }
    }

    #[test]
    fn resolve_reference_type_from_cpp() {
        let path = match find_fixture_path("test_cpp_templates") {
            Some(p) => p,
            None => return,
        };

        let binary = match BinaryData::load(&path) {
            Ok(b) => b,
            Err(_) => return,
        };
        let loaded = match binary.load_dwarf() {
            Ok(l) => l,
            Err(_) => return,
        };
        let dwarf = &loaded.dwarf;
        let mut units = dwarf.units();
        let header = match units.next() {
            Ok(Some(h)) => h,
            _ => return,
        };
        let unit = match dwarf.unit(header) {
            Ok(u) => u,
            Err(_) => return,
        };

        let mut resolver = TypeResolver::new(&loaded.dwarf, &unit, loaded.address_size);
        if let Some(offset) = find_type_offset(&unit, gimli::DW_TAG_reference_type) {
            let _ = resolver.resolve_type(offset).expect("resolve reference type");
        }
    }

    #[test]
    fn resolve_cpp_class_and_union_types() {
        let path = match find_fixture_path("test_cpp_templates") {
            Some(p) => p,
            None => return,
        };

        let binary = match BinaryData::load(&path) {
            Ok(b) => b,
            Err(_) => return,
        };
        let loaded = match binary.load_dwarf() {
            Ok(l) => l,
            Err(_) => return,
        };
        let dwarf = &loaded.dwarf;
        let mut units = dwarf.units();
        let header = match units.next() {
            Ok(Some(h)) => h,
            _ => return,
        };
        let unit = match dwarf.unit(header) {
            Ok(u) => u,
            Err(_) => return,
        };

        let mut resolver = TypeResolver::new(&loaded.dwarf, &unit, loaded.address_size);
        for tag in [gimli::DW_TAG_class_type, gimli::DW_TAG_union_type] {
            if let Some(offset) = find_type_offset(&unit, tag) {
                let _ = resolver.resolve_type(offset).expect("resolve cpp type");
            }
        }
    }

    #[test]
    fn resolve_all_type_entries_best_effort() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let binary = match BinaryData::load(&path) {
            Ok(b) => b,
            Err(_) => return,
        };
        let loaded = match binary.load_dwarf() {
            Ok(l) => l,
            Err(_) => return,
        };
        let dwarf = &loaded.dwarf;
        let mut units = dwarf.units();
        let header = match units.next() {
            Ok(Some(h)) => h,
            _ => return,
        };
        let unit = match dwarf.unit(header) {
            Ok(u) => u,
            Err(_) => return,
        };

        let mut resolver = TypeResolver::new(&loaded.dwarf, &unit, loaded.address_size);
        let mut entries = unit.entries();
        while let Some((_, entry)) = entries.next_dfs().ok().flatten() {
            let tag = entry.tag();
            if matches!(
                tag,
                gimli::DW_TAG_base_type
                    | gimli::DW_TAG_pointer_type
                    | gimli::DW_TAG_reference_type
                    | gimli::DW_TAG_const_type
                    | gimli::DW_TAG_volatile_type
                    | gimli::DW_TAG_restrict_type
                    | gimli::DW_TAG_atomic_type
                    | gimli::DW_TAG_typedef
                    | gimli::DW_TAG_array_type
                    | gimli::DW_TAG_structure_type
                    | gimli::DW_TAG_class_type
                    | gimli::DW_TAG_union_type
                    | gimli::DW_TAG_enumeration_type
                    | gimli::DW_TAG_subroutine_type
            ) {
                let _ = resolver.resolve_type(entry.offset());
            }
        }
    }
}
