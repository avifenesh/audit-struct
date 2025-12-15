use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StructLayout {
    pub name: String,
    pub size: u64,
    pub alignment: Option<u64>,
    pub members: Vec<MemberLayout>,
    pub metrics: LayoutMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberLayout {
    pub name: String,
    pub type_name: String,
    pub offset: Option<u64>,
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_size: Option<u64>,
    /// True if the type was marked with DW_TAG_atomic_type in DWARF debug info.
    /// This provides more reliable atomic detection than string pattern matching.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_atomic: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LayoutMetrics {
    pub total_size: u64,
    pub useful_size: u64,
    pub padding_bytes: u64,
    pub padding_percentage: f64,
    pub cache_lines_spanned: u32,
    pub cache_line_density: f64,
    pub padding_holes: Vec<PaddingHole>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub partial: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub false_sharing: Option<FalseSharingAnalysis>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaddingHole {
    pub offset: u64,
    pub size: u64,
    pub after_member: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FalseSharingWarning {
    pub member_a: String,
    pub member_b: String,
    pub cache_line: u64,
    /// Gap in bytes between member_a's end and member_b's start.
    /// Negative = overlap, Zero = adjacent, Positive = gap
    pub gap_bytes: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CacheLineSpanningWarning {
    pub member: String,
    pub type_name: String,
    pub offset: u64,
    pub size: u64,
    pub start_cache_line: u64,
    pub end_cache_line: u64,
    pub lines_spanned: u64,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
pub struct FalseSharingAnalysis {
    pub atomic_members: Vec<AtomicMember>,
    pub warnings: Vec<FalseSharingWarning>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub spanning_warnings: Vec<CacheLineSpanningWarning>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AtomicMember {
    pub name: String,
    pub type_name: String,
    pub offset: u64,
    pub size: u64,
    pub cache_line: u64,
    pub end_cache_line: u64,
    pub spans_cache_lines: bool,
}

impl StructLayout {
    pub fn new(name: String, size: u64, alignment: Option<u64>) -> Self {
        Self {
            name,
            size,
            alignment,
            members: Vec::new(),
            metrics: LayoutMetrics::default(),
            source_location: None,
        }
    }
}

impl MemberLayout {
    pub fn new(name: String, type_name: String, offset: Option<u64>, size: Option<u64>) -> Self {
        Self { name, type_name, offset, size, bit_offset: None, bit_size: None, is_atomic: false }
    }

    pub fn with_atomic(mut self, is_atomic: bool) -> Self {
        self.is_atomic = is_atomic;
        self
    }

    pub fn end_offset(&self) -> Option<u64> {
        match (self.offset, self.size) {
            // Use checked_add to prevent overflow for malformed DWARF data.
            (Some(off), Some(sz)) => off.checked_add(sz),
            _ => None,
        }
    }
}
