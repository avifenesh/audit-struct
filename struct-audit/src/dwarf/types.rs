use crate::error::{Error, Result};
use crate::loader::DwarfSlice;
use gimli::{AttributeValue, Dwarf, Unit, UnitOffset};
use std::collections::HashMap;

pub struct TypeResolver<'a, 'b> {
    dwarf: &'b Dwarf<DwarfSlice<'a>>,
    unit: &'b Unit<DwarfSlice<'a>>,
    address_size: u8,
    cache: HashMap<UnitOffset, (String, Option<u64>)>,
}

impl<'a, 'b> TypeResolver<'a, 'b> {
    pub fn new(
        dwarf: &'b Dwarf<DwarfSlice<'a>>,
        unit: &'b Unit<DwarfSlice<'a>>,
        address_size: u8,
    ) -> Self {
        Self { dwarf, unit, address_size, cache: HashMap::new() }
    }

    pub fn resolve_type(&mut self, offset: UnitOffset) -> Result<(String, Option<u64>)> {
        if let Some(cached) = self.cache.get(&offset) {
            return Ok(cached.clone());
        }

        let result = self.resolve_type_inner(offset, 0)?;
        self.cache.insert(offset, result.clone());
        Ok(result)
    }

    fn resolve_type_inner(
        &mut self,
        offset: UnitOffset,
        depth: usize,
    ) -> Result<(String, Option<u64>)> {
        if depth > 20 {
            return Ok(("...".to_string(), None));
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
                Ok((name, size))
            }

            gimli::DW_TAG_pointer_type => {
                let pointee = if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (pointee_name, _) = self.resolve_type_inner(type_offset, depth + 1)?;
                    pointee_name
                } else {
                    "void".to_string()
                };
                Ok((format!("*{}", pointee), Some(self.address_size as u64)))
            }

            gimli::DW_TAG_reference_type => {
                let referee = if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (referee_name, _) = self.resolve_type_inner(type_offset, depth + 1)?;
                    referee_name
                } else {
                    "void".to_string()
                };
                Ok((format!("&{}", referee), Some(self.address_size as u64)))
            }

            gimli::DW_TAG_const_type
            | gimli::DW_TAG_volatile_type
            | gimli::DW_TAG_restrict_type => {
                let prefix = match tag {
                    gimli::DW_TAG_const_type => "const ",
                    gimli::DW_TAG_volatile_type => "volatile ",
                    gimli::DW_TAG_restrict_type => "restrict ",
                    _ => "",
                };
                if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (inner_name, size) = self.resolve_type_inner(type_offset, depth + 1)?;
                    Ok((format!("{}{}", prefix, inner_name), size))
                } else {
                    Ok((format!("{}void", prefix), None))
                }
            }

            gimli::DW_TAG_typedef => {
                let name = self.get_type_name(&entry)?;
                if let Some(type_offset) = self.get_type_ref(&entry)? {
                    let (_, size) = self.resolve_type_inner(type_offset, depth + 1)?;
                    Ok((name.unwrap_or_else(|| "typedef".to_string()), size))
                } else {
                    Ok((name.unwrap_or_else(|| "typedef".to_string()), None))
                }
            }

            gimli::DW_TAG_array_type => {
                let element_type = if let Some(type_offset) = self.get_type_ref(&entry)? {
                    self.resolve_type_inner(type_offset, depth + 1)?
                } else {
                    ("?".to_string(), None)
                };

                let count = self.get_array_count(&entry)?;
                let size = match (element_type.1, count) {
                    (Some(elem_size), Some(c)) => Some(elem_size * c),
                    _ => self.get_byte_size(&entry)?,
                };

                let count_str = count.map(|c| c.to_string()).unwrap_or_else(|| "?".to_string());
                Ok((format!("[{}; {}]", element_type.0, count_str), size))
            }

            gimli::DW_TAG_structure_type | gimli::DW_TAG_class_type | gimli::DW_TAG_union_type => {
                let name = self.get_type_name(&entry)?.unwrap_or_else(|| "<anonymous>".to_string());
                let size = self.get_byte_size(&entry)?;
                Ok((name, size))
            }

            gimli::DW_TAG_enumeration_type => {
                let name = self.get_type_name(&entry)?.unwrap_or_else(|| "enum".to_string());
                let size = self.get_byte_size(&entry)?;
                Ok((name, size))
            }

            gimli::DW_TAG_subroutine_type => {
                Ok(("fn(...)".to_string(), Some(self.address_size as u64)))
            }

            _ => {
                let name = self.get_type_name(&entry)?.unwrap_or_else(|| format!("?<{:?}>", tag));
                let size = self.get_byte_size(&entry)?;
                Ok((name, size))
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
                Ok(Some(name.to_string_lossy().to_string()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Dwarf(format!("Failed to read name attr: {}", e))),
        }
    }

    fn get_byte_size(
        &self,
        entry: &gimli::DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<u64>> {
        match entry.attr_value(gimli::DW_AT_byte_size) {
            Ok(Some(AttributeValue::Udata(s))) => Ok(Some(s)),
            Ok(Some(AttributeValue::Data1(s))) => Ok(Some(s as u64)),
            Ok(Some(AttributeValue::Data2(s))) => Ok(Some(s as u64)),
            Ok(Some(AttributeValue::Data4(s))) => Ok(Some(s as u64)),
            Ok(Some(AttributeValue::Data8(s))) => Ok(Some(s)),
            _ => Ok(None),
        }
    }

    fn get_type_ref(
        &self,
        entry: &gimli::DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<UnitOffset>> {
        match entry.attr_value(gimli::DW_AT_type) {
            Ok(Some(AttributeValue::UnitRef(offset))) => Ok(Some(offset)),
            Ok(Some(AttributeValue::DebugInfoRef(debug_info_offset))) => {
                // Convert section offset to unit offset
                let unit_offset = UnitOffset(
                    debug_info_offset
                        .0
                        .saturating_sub(self.unit.header.offset().as_debug_info_offset().unwrap().0),
                );
                Ok(Some(unit_offset))
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
                // Fall back to DW_AT_upper_bound (0-indexed, so add 1)
                if let Some(upper) =
                    self.extract_count_attr(child_entry, gimli::DW_AT_upper_bound)?
                {
                    return Ok(Some(upper + 1));
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
        match entry.attr_value(attr) {
            Ok(Some(AttributeValue::Udata(v))) => Ok(Some(v)),
            Ok(Some(AttributeValue::Data1(v))) => Ok(Some(v as u64)),
            Ok(Some(AttributeValue::Data2(v))) => Ok(Some(v as u64)),
            Ok(Some(AttributeValue::Data4(v))) => Ok(Some(v as u64)),
            Ok(Some(AttributeValue::Data8(v))) => Ok(Some(v)),
            Ok(Some(AttributeValue::Sdata(v))) if v >= 0 => Ok(Some(v as u64)),
            _ => Ok(None),
        }
    }
}
