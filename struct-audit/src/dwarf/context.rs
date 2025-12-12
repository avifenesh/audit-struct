use crate::error::{Error, Result};
use crate::loader::{DwarfSlice, LoadedDwarf};
use crate::types::{MemberLayout, SourceLocation, StructLayout};
use gimli::{AttributeValue, DebuggingInformationEntry, Dwarf, Unit};

use super::TypeResolver;
use super::expr::{evaluate_member_offset, try_simple_offset};

pub struct DwarfContext<'a> {
    dwarf: &'a Dwarf<DwarfSlice<'a>>,
    address_size: u8,
}

impl<'a> DwarfContext<'a> {
    pub fn new(loaded: &'a LoadedDwarf<'a>) -> Self {
        Self { dwarf: &loaded.dwarf, address_size: loaded.address_size }
    }

    pub fn find_structs(&self, filter: Option<&str>) -> Result<Vec<StructLayout>> {
        let mut structs = Vec::new();
        let mut units = self.dwarf.units();

        while let Some(header) =
            units.next().map_err(|e| Error::Dwarf(format!("Failed to read unit header: {}", e)))?
        {
            let unit = self
                .dwarf
                .unit(header)
                .map_err(|e| Error::Dwarf(format!("Failed to parse unit: {}", e)))?;

            self.process_unit(&unit, filter, &mut structs)?;
        }

        Ok(structs)
    }

    fn process_unit(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        filter: Option<&str>,
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

            if let Some(layout) =
                self.process_struct_entry(unit, entry, filter, &mut type_resolver)?
            {
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
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<Option<StructLayout>> {
        let size = match entry.attr_value(gimli::DW_AT_byte_size) {
            Ok(Some(AttributeValue::Udata(s))) => s,
            Ok(Some(AttributeValue::Data1(s))) => s as u64,
            Ok(Some(AttributeValue::Data2(s))) => s as u64,
            Ok(Some(AttributeValue::Data4(s))) => s as u64,
            Ok(Some(AttributeValue::Data8(s))) => s,
            _ => return Ok(None), // Forward declaration or no size
        };

        let name = self.get_die_name(unit, entry)?;
        let name = match name {
            Some(n) if !n.starts_with("__") => n, // Skip compiler-generated
            None => return Ok(None),              // Anonymous struct
            _ => return Ok(None),
        };

        if filter.is_some_and(|f| !name.contains(f)) {
            return Ok(None);
        }

        let alignment = match entry.attr_value(gimli::DW_AT_alignment) {
            Ok(Some(AttributeValue::Udata(a))) => Some(a),
            _ => None,
        };

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

    fn process_inheritance(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<Option<MemberLayout>> {
        let offset = self.get_member_offset(unit, entry)?;

        let (type_name, size) = match entry.attr_value(gimli::DW_AT_type) {
            Ok(Some(AttributeValue::UnitRef(type_offset))) => {
                type_resolver.resolve_type(type_offset)?
            }
            Ok(Some(AttributeValue::DebugInfoRef(debug_info_offset))) => {
                if let Some(unit_debug_offset) = unit.header.offset().as_debug_info_offset() {
                    let unit_offset =
                        gimli::UnitOffset(debug_info_offset.0.saturating_sub(unit_debug_offset.0));
                    type_resolver.resolve_type(unit_offset)?
                } else {
                    ("unknown".to_string(), None)
                }
            }
            _ => ("unknown".to_string(), None),
        };

        let name = format!("<base: {}>", type_name);
        Ok(Some(MemberLayout::new(name, type_name, offset, size)))
    }

    fn process_member(
        &self,
        unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
        type_resolver: &mut TypeResolver<'a, '_>,
    ) -> Result<Option<MemberLayout>> {
        let name = self.get_die_name(unit, entry)?.unwrap_or_else(|| "<anonymous>".to_string());

        let offset = self.get_member_offset(unit, entry)?;

        let (type_name, size) = match entry.attr_value(gimli::DW_AT_type) {
            Ok(Some(AttributeValue::UnitRef(type_offset))) => {
                type_resolver.resolve_type(type_offset)?
            }
            Ok(Some(AttributeValue::DebugInfoRef(debug_info_offset))) => {
                // Convert section offset to unit offset
                if let Some(unit_debug_offset) = unit.header.offset().as_debug_info_offset() {
                    let unit_offset =
                        gimli::UnitOffset(debug_info_offset.0.saturating_sub(unit_debug_offset.0));
                    type_resolver.resolve_type(unit_offset)?
                } else {
                    ("unknown".to_string(), None)
                }
            }
            _ => ("unknown".to_string(), None),
        };

        let mut member = MemberLayout::new(name, type_name, offset, size);

        // Parse bitfield attributes (DWARF 4 style)
        if let Ok(Some(AttributeValue::Udata(bit_offset))) =
            entry.attr_value(gimli::DW_AT_bit_offset)
        {
            member.bit_offset = Some(bit_offset);
        }

        // DWARF 5 style: DW_AT_data_bit_offset (offset from start of containing entity)
        if let Ok(Some(AttributeValue::Udata(data_bit_offset))) =
            entry.attr_value(gimli::DW_AT_data_bit_offset)
        {
            member.bit_offset = Some(data_bit_offset);
        }

        if let Ok(Some(AttributeValue::Udata(bit_size))) = entry.attr_value(gimli::DW_AT_bit_size) {
            member.bit_size = Some(bit_size);
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
                Ok(Some(name.to_string_lossy().to_string()))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Error::Dwarf(format!("Failed to read name attribute: {}", e))),
        }
    }

    fn get_source_location(
        &self,
        _unit: &Unit<DwarfSlice<'a>>,
        entry: &DebuggingInformationEntry<DwarfSlice<'a>>,
    ) -> Result<Option<SourceLocation>> {
        let file_index = match entry.attr_value(gimli::DW_AT_decl_file) {
            Ok(Some(AttributeValue::FileIndex(idx))) => idx,
            Ok(Some(AttributeValue::Udata(idx))) => idx,
            _ => return Ok(None),
        };

        let line = match entry.attr_value(gimli::DW_AT_decl_line) {
            Ok(Some(AttributeValue::Udata(l))) => l,
            _ => return Ok(None),
        };

        // File name resolution would require parsing .debug_line
        // For MVP, just return the file index
        Ok(Some(SourceLocation { file: format!("file#{}", file_index), line }))
    }
}
