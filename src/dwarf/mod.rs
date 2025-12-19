mod context;
mod expr;
mod types;

pub use context::{DwarfContext, is_go_internal_type};
pub use types::TypeResolver;

use crate::loader::DwarfSlice;
use gimli::{AttributeValue, DebugInfoOffset, UnitHeader, UnitOffset};

/// Extract a u64 value from a DWARF attribute, handling various encoding forms.
/// Returns None for negative Sdata values (invalid for offsets/sizes/indices).
/// Used by both context.rs and types.rs to avoid duplication.
pub(crate) fn read_u64_from_attr(attr: Option<AttributeValue<DwarfSlice<'_>>>) -> Option<u64> {
    match attr? {
        AttributeValue::FileIndex(idx) => Some(idx),
        AttributeValue::Udata(v) => Some(v),
        AttributeValue::Data1(v) => Some(v as u64),
        AttributeValue::Data2(v) => Some(v as u64),
        AttributeValue::Data4(v) => Some(v as u64),
        AttributeValue::Data8(v) => Some(v),
        // Negative Sdata values are invalid for offsets/sizes/indices - return None.
        AttributeValue::Sdata(v) if v >= 0 => Some(v as u64),
        _ => None,
    }
}

/// Convert a DebugInfoRef (section offset) to a UnitOffset (unit-relative offset).
/// Returns None if the reference is invalid (cross-unit or corrupted DWARF).
/// Used by both types.rs and context.rs for consistent cross-unit reference handling.
pub(crate) fn debug_info_ref_to_unit_offset<R: gimli::Reader>(
    debug_info_offset: DebugInfoOffset<R::Offset>,
    unit_header: &UnitHeader<R>,
) -> Option<UnitOffset<R::Offset>>
where
    R::Offset: std::ops::Sub<Output = R::Offset>,
{
    let unit_debug_offset = unit_header.offset().as_debug_info_offset()?;
    // Use checked arithmetic to detect invalid cross-unit references.
    // If debug_info_offset < unit_debug_offset, this is corrupted DWARF.
    if debug_info_offset.0 >= unit_debug_offset.0 {
        Some(UnitOffset(debug_info_offset.0 - unit_debug_offset.0))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gimli::{
        DebugAbbrevOffset, DebugInfoOffset, Encoding, Format, UnitHeader, UnitSectionOffset,
        UnitType,
    };

    #[test]
    fn read_u64_from_attr_handles_encodings() {
        assert_eq!(read_u64_from_attr(Some(AttributeValue::Data1(1))), Some(1));
        assert_eq!(read_u64_from_attr(Some(AttributeValue::Data2(2))), Some(2));
        assert_eq!(read_u64_from_attr(Some(AttributeValue::Data4(3))), Some(3));
        assert_eq!(read_u64_from_attr(Some(AttributeValue::Data8(4))), Some(4));
        assert_eq!(read_u64_from_attr(Some(AttributeValue::Udata(5))), Some(5));
        assert_eq!(read_u64_from_attr(Some(AttributeValue::Sdata(-1))), None);
        assert_eq!(read_u64_from_attr(Some(AttributeValue::Sdata(7))), Some(7));
        assert_eq!(read_u64_from_attr(Some(AttributeValue::FileIndex(9))), Some(9));
    }

    #[test]
    fn debug_info_ref_to_unit_offset_validates_unit() {
        let encoding = Encoding { address_size: 8, format: Format::Dwarf32, version: 4 };
        let unit_offset = UnitSectionOffset::DebugInfoOffset(DebugInfoOffset(0x100));
        let header = UnitHeader::new(
            encoding,
            0,
            UnitType::Compilation,
            DebugAbbrevOffset(0),
            unit_offset,
            DwarfSlice::new(&[], gimli::RunTimeEndian::Little),
        );

        let within = debug_info_ref_to_unit_offset(DebugInfoOffset(0x100), &header);
        assert_eq!(within, Some(UnitOffset(0)));

        let outside = debug_info_ref_to_unit_offset(DebugInfoOffset(0x0ff), &header);
        assert_eq!(outside, None);
    }
}
