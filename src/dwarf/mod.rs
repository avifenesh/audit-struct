mod context;
mod expr;
mod types;

pub use context::DwarfContext;
pub use types::TypeResolver;

use crate::loader::DwarfSlice;
use gimli::AttributeValue;

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
