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
}

#[derive(Debug, Clone, Serialize)]
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

impl Default for LayoutMetrics {
    fn default() -> Self {
        Self {
            total_size: 0,
            useful_size: 0,
            padding_bytes: 0,
            padding_percentage: 0.0,
            cache_lines_spanned: 0,
            cache_line_density: 0.0,
            padding_holes: Vec::new(),
            partial: false,
        }
    }
}

impl MemberLayout {
    pub fn new(name: String, type_name: String, offset: Option<u64>, size: Option<u64>) -> Self {
        Self { name, type_name, offset, size, bit_offset: None, bit_size: None }
    }

    pub fn end_offset(&self) -> Option<u64> {
        match (self.offset, self.size) {
            (Some(off), Some(sz)) => Some(off + sz),
            _ => None,
        }
    }
}
