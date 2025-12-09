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
