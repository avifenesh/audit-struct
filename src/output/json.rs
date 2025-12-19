use crate::types::StructLayout;
use serde::Serialize;

#[derive(Serialize)]
struct Output<'a> {
    version: &'static str,
    structs: &'a [StructLayout],
}

pub struct JsonFormatter {
    pretty: bool,
}

impl JsonFormatter {
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    pub fn format(&self, layouts: &[StructLayout]) -> String {
        let output = Output { version: env!("CARGO_PKG_VERSION"), structs: layouts };

        if self.pretty {
            serde_json::to_string_pretty(&output)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
        } else {
            serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LayoutMetrics, StructLayout};

    fn layout(name: &str) -> StructLayout {
        let mut s = StructLayout::new(name.to_string(), 8, Some(8));
        s.metrics = LayoutMetrics { padding_bytes: 0, total_size: 8, ..LayoutMetrics::default() };
        s
    }

    #[test]
    fn json_formatter_pretty() {
        let formatter = JsonFormatter::new(true);
        let out = formatter.format(&[layout("Foo")]);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert!(parsed["structs"].is_array());
    }

    #[test]
    fn json_formatter_compact() {
        let formatter = JsonFormatter::new(false);
        let out = formatter.format(&[layout("Foo")]);
        assert!(out.contains("\"structs\""));
    }
}
