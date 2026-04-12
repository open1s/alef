use crate::generators::RustBindingConfig;
use alef_core::ir::EnumDef;
use std::fmt::Write;

/// Returns true if any variant of the enum has data fields.
/// These enums cannot be represented as flat integer enums in bindings.
pub fn enum_has_data_variants(enum_def: &EnumDef) -> bool {
    enum_def.variants.iter().any(|v| !v.fields.is_empty())
}

/// Generate a PyO3 data enum as a `#[pyclass]` struct wrapping the core type.
///
/// Data enums (tagged unions like `AuthConfig`) can't be flat int enums in PyO3.
/// Instead, generate a frozen struct with `inner` that accepts a Python dict,
/// serializes it to JSON, and deserializes into the core Rust type via serde.
pub fn gen_pyo3_data_enum(enum_def: &EnumDef, core_import: &str) -> String {
    let name = &enum_def.name;
    let core_path = format!("{core_import}::{name}");
    let mut out = String::with_capacity(512);

    writeln!(out, "#[derive(Clone)]").ok();
    writeln!(out, "#[pyclass(frozen)]").ok();
    writeln!(out, "pub struct {name} {{").ok();
    writeln!(out, "    pub(crate) inner: {core_path},").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    writeln!(out, "#[pymethods]").ok();
    writeln!(out, "impl {name} {{").ok();
    writeln!(out, "    #[new]").ok();
    writeln!(
        out,
        "    fn new(py: Python<'_>, value: &Bound<'_, pyo3::types::PyDict>) -> PyResult<Self> {{"
    )
    .ok();
    writeln!(out, "        let json_mod = py.import(\"json\")?;").ok();
    writeln!(
        out,
        "        let json_str: String = json_mod.call_method1(\"dumps\", (value,))?.extract()?;"
    )
    .ok();
    writeln!(out, "        let inner: {core_path} = serde_json::from_str(&json_str)").ok();
    writeln!(
        out,
        "            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!(\"Invalid {name}: {{e}}\")))?;"
    )
    .ok();
    writeln!(out, "        Ok(Self {{ inner }})").ok();
    writeln!(out, "    }}").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    // From binding → core
    writeln!(out, "impl From<{name}> for {core_path} {{").ok();
    writeln!(out, "    fn from(val: {name}) -> Self {{ val.inner }}").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    // From core → binding
    writeln!(out, "impl From<{core_path}> for {name} {{").ok();
    writeln!(out, "    fn from(val: {core_path}) -> Self {{ Self {{ inner: val }} }}").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    // Serialize: forward to inner so parent structs that derive serde::Serialize compile.
    writeln!(out, "impl serde::Serialize for {name} {{").ok();
    writeln!(
        out,
        "    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {{"
    )
    .ok();
    writeln!(out, "        self.inner.serialize(serializer)").ok();
    writeln!(out, "    }}").ok();
    writeln!(out, "}}").ok();
    writeln!(out).ok();

    // Default: forward to inner's Default so parent structs that derive Default compile.
    writeln!(out, "impl Default for {name} {{").ok();
    writeln!(
        out,
        "    fn default() -> Self {{ Self {{ inner: Default::default() }} }}"
    )
    .ok();
    writeln!(out, "}}").ok();

    out
}

/// Generate an enum.
pub fn gen_enum(enum_def: &EnumDef, cfg: &RustBindingConfig) -> String {
    // All enums are generated as unit-variant-only in the binding layer.
    // Data variants are flattened to unit variants; the From/Into conversions
    // handle the lossy mapping (discarding / providing defaults for field data).
    let mut out = String::with_capacity(512);
    let mut derives: Vec<&str> = cfg.enum_derives.to_vec();
    if cfg.has_serde {
        derives.push("serde::Serialize");
    }
    if !derives.is_empty() {
        writeln!(out, "#[derive({})]", derives.join(", ")).ok();
    }
    for attr in cfg.enum_attrs {
        writeln!(out, "#[{attr}]").ok();
    }
    writeln!(out, "pub enum {} {{", enum_def.name).ok();
    for (idx, variant) in enum_def.variants.iter().enumerate() {
        writeln!(out, "    {} = {idx},", variant.name).ok();
    }
    writeln!(out, "}}").ok();

    // Generate Default impl (first variant) so enums can be used with unwrap_or_default()
    // in config constructors for types with has_default.
    if let Some(first) = enum_def.variants.first() {
        writeln!(out).ok();
        writeln!(out, "#[allow(clippy::derivable_impls)]").ok();
        writeln!(out, "impl Default for {} {{", enum_def.name).ok();
        writeln!(out, "    fn default() -> Self {{ Self::{} }}", first.name).ok();
        writeln!(out, "}}").ok();
    }

    out
}
