use eisberg_core::ir::ErrorDef;

/// Generate `pyo3::create_exception!` macros for each error variant plus the base error type.
pub fn gen_pyo3_error_types(error: &ErrorDef, module_name: &str) -> String {
    let mut lines = Vec::with_capacity(error.variants.len() + 2);
    lines.push("// Error types".to_string());

    // One exception per variant
    for variant in &error.variants {
        lines.push(format!(
            "pyo3::create_exception!({module_name}, {}, pyo3::exceptions::PyException);",
            variant.name
        ));
    }

    // Base exception for the enum itself
    lines.push(format!(
        "pyo3::create_exception!({module_name}, {}, pyo3::exceptions::PyException);",
        error.name
    ));

    lines.join("\n")
}

/// Generate a `to_py_err` converter function that maps each Rust error variant to a Python exception.
pub fn gen_pyo3_error_converter(error: &ErrorDef, core_import: &str) -> String {
    let rust_path = if error.rust_path.is_empty() {
        format!("{core_import}::{}", error.name)
    } else {
        error.rust_path.replace('-', "_")
    };

    let fn_name = format!("{}_to_py_err", to_snake_case(&error.name));

    let mut lines = Vec::new();
    lines.push(format!("/// Convert a `{rust_path}` error to a Python exception."));
    lines.push(format!("fn {fn_name}(e: {rust_path}) -> pyo3::PyErr {{"));
    lines.push("    let msg = e.to_string();".to_string());
    lines.push("    match &e {".to_string());

    for variant in &error.variants {
        let pattern = if variant.is_unit {
            format!("{rust_path}::{}", variant.name)
        } else {
            format!("{rust_path}::{}(..)", variant.name)
        };
        lines.push(format!("        {pattern} => {}::new_err(msg),", variant.name));
    }

    lines.push("    }".to_string());
    lines.push("}".to_string());
    lines.join("\n")
}

/// Generate `m.add(...)` registration calls for each exception type.
pub fn gen_pyo3_error_registration(error: &ErrorDef) -> Vec<String> {
    let mut registrations = Vec::with_capacity(error.variants.len() + 1);

    for variant in &error.variants {
        registrations.push(format!(
            "    m.add(\"{}\", m.py().get_type::<{}>())?;",
            variant.name, variant.name
        ));
    }

    // Base exception
    registrations.push(format!(
        "    m.add(\"{}\", m.py().get_type::<{}>())?;",
        error.name, error.name
    ));

    registrations
}

/// Return the converter function name for a given error type.
pub fn converter_fn_name(error: &ErrorDef) -> String {
    format!("{}_to_py_err", to_snake_case(&error.name))
}

/// Simple CamelCase to snake_case conversion.
fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use eisberg_core::ir::{ErrorDef, ErrorVariant};

    fn sample_error() -> ErrorDef {
        ErrorDef {
            name: "ConversionError".to_string(),
            rust_path: "html_to_markdown_rs::ConversionError".to_string(),
            variants: vec![
                ErrorVariant {
                    name: "ParseError".to_string(),
                    message_template: Some("HTML parsing error: {0}".to_string()),
                    fields: vec![],
                    has_source: false,
                    has_from: false,
                    is_unit: false,
                    doc: String::new(),
                },
                ErrorVariant {
                    name: "IoError".to_string(),
                    message_template: Some("I/O error: {0}".to_string()),
                    fields: vec![],
                    has_source: false,
                    has_from: true,
                    is_unit: false,
                    doc: String::new(),
                },
                ErrorVariant {
                    name: "Other".to_string(),
                    message_template: Some("Conversion error: {0}".to_string()),
                    fields: vec![],
                    has_source: false,
                    has_from: false,
                    is_unit: false,
                    doc: String::new(),
                },
            ],
            doc: "Error type for conversion operations.".to_string(),
        }
    }

    #[test]
    fn test_gen_error_types() {
        let error = sample_error();
        let output = gen_pyo3_error_types(&error, "_module");
        assert!(output.contains("pyo3::create_exception!(_module, ParseError, pyo3::exceptions::PyException);"));
        assert!(output.contains("pyo3::create_exception!(_module, IoError, pyo3::exceptions::PyException);"));
        assert!(output.contains("pyo3::create_exception!(_module, Other, pyo3::exceptions::PyException);"));
        assert!(output.contains("pyo3::create_exception!(_module, ConversionError, pyo3::exceptions::PyException);"));
    }

    #[test]
    fn test_gen_error_converter() {
        let error = sample_error();
        let output = gen_pyo3_error_converter(&error, "html_to_markdown_rs");
        assert!(
            output.contains("fn conversion_error_to_py_err(e: html_to_markdown_rs::ConversionError) -> pyo3::PyErr {")
        );
        assert!(output.contains("html_to_markdown_rs::ConversionError::ParseError(..) => ParseError::new_err(msg),"));
        assert!(output.contains("html_to_markdown_rs::ConversionError::IoError(..) => IoError::new_err(msg),"));
    }

    #[test]
    fn test_gen_error_registration() {
        let error = sample_error();
        let regs = gen_pyo3_error_registration(&error);
        assert_eq!(regs.len(), 4); // 3 variants + 1 base
        assert!(regs[0].contains("\"ParseError\""));
        assert!(regs[3].contains("\"ConversionError\""));
    }

    #[test]
    fn test_unit_variant_pattern() {
        let error = ErrorDef {
            name: "MyError".to_string(),
            rust_path: "my_crate::MyError".to_string(),
            variants: vec![ErrorVariant {
                name: "NotFound".to_string(),
                message_template: Some("not found".to_string()),
                fields: vec![],
                has_source: false,
                has_from: false,
                is_unit: true,
                doc: String::new(),
            }],
            doc: String::new(),
        };
        let output = gen_pyo3_error_converter(&error, "my_crate");
        assert!(output.contains("my_crate::MyError::NotFound => NotFound::new_err(msg),"));
        // Ensure no (..) for unit variants
        assert!(!output.contains("NotFound(..)"));
    }
}
