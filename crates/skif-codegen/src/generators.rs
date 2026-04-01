use crate::builder::StructBuilder;
use crate::shared::{constructor_parts, function_params, function_sig_defaults, partition_methods};
use crate::type_mapper::TypeMapper;
use skif_core::ir::{EnumDef, FunctionDef, MethodDef, ParamDef, TypeDef, TypeRef};
use std::fmt::Write;

/// Async support pattern for the backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsyncPattern {
    /// No async support
    None,
    /// PyO3: pyo3_async_runtimes::tokio::future_into_py
    Pyo3FutureIntoPy,
    /// NAPI-RS: native async fn → auto-Promise
    NapiNativeAsync,
    /// wasm-bindgen: native async fn → auto-Promise
    WasmNativeAsync,
    /// Block on Tokio runtime (Ruby, PHP)
    TokioBlockOn,
}

/// Configuration for Rust binding code generation.
pub struct RustBindingConfig<'a> {
    /// Attrs applied to generated structs, e.g. `["pyclass(frozen)"]`.
    pub struct_attrs: &'a [&'a str],
    /// Attrs applied to each field, e.g. `["pyo3(get)"]`.
    pub field_attrs: &'a [&'a str],
    /// Derives applied to generated structs, e.g. `["Clone"]`.
    pub struct_derives: &'a [&'a str],
    /// Attr wrapping the impl block, e.g. `Some("pymethods")`.
    pub method_block_attr: Option<&'a str>,
    /// Attr placed on the constructor, e.g. `"#[new]"`.
    pub constructor_attr: &'a str,
    /// Attr placed on static methods, e.g. `Some("staticmethod")`.
    pub static_attr: Option<&'a str>,
    /// Attr placed on free functions, e.g. `"#[pyfunction]"`.
    pub function_attr: &'a str,
    /// Attrs applied to generated enums, e.g. `["pyclass(eq, eq_int)"]`.
    pub enum_attrs: &'a [&'a str],
    /// Derives applied to generated enums, e.g. `["Clone", "PartialEq"]`.
    pub enum_derives: &'a [&'a str],
    /// Whether the backend requires `#[pyo3(signature = (...))]`-style annotations.
    pub needs_signature: bool,
    /// Prefix for the signature annotation, e.g. `"#[pyo3(signature = ("`.
    pub signature_prefix: &'a str,
    /// Suffix for the signature annotation, e.g. `"))]"`.
    pub signature_suffix: &'a str,
    /// Core crate import path, e.g. `"liter_llm"`. Used to generate calls into core.
    pub core_import: &'a str,
    /// Async pattern supported by this backend.
    pub async_pattern: AsyncPattern,
}

/// Build call argument expressions from parameters, applying `.into()` on Named types.
fn gen_call_args(params: &[ParamDef]) -> String {
    params
        .iter()
        .map(|p| match &p.ty {
            TypeRef::Named(_) => {
                if p.optional {
                    format!("{}.map(Into::into)", p.name)
                } else {
                    format!("{}.into()", p.name)
                }
            }
            _ => p.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generate a struct definition using the builder.
pub fn gen_struct(typ: &TypeDef, mapper: &dyn TypeMapper, cfg: &RustBindingConfig) -> String {
    let mut sb = StructBuilder::new(&typ.name);
    for attr in cfg.struct_attrs {
        sb.add_attr(attr);
    }
    for d in cfg.struct_derives {
        sb.add_derive(d);
    }
    for field in &typ.fields {
        let ty = if field.optional {
            mapper.optional(&mapper.map_type(&field.ty))
        } else {
            mapper.map_type(&field.ty)
        };
        let attrs: Vec<String> = cfg.field_attrs.iter().map(|a| a.to_string()).collect();
        sb.add_field(&field.name, &ty, attrs);
    }
    sb.build()
}

/// Generate a constructor method.
pub fn gen_constructor(typ: &TypeDef, mapper: &dyn TypeMapper, cfg: &RustBindingConfig) -> String {
    let map_fn = |ty: &skif_core::ir::TypeRef| mapper.map_type(ty);
    let (param_list, sig_defaults, assignments) = constructor_parts(&typ.fields, &map_fn);

    let mut out = String::new();
    if cfg.needs_signature {
        writeln!(
            out,
            "    {}{}{}",
            cfg.signature_prefix, sig_defaults, cfg.signature_suffix
        )
        .ok();
    }
    write!(
        out,
        "    {}\n    pub fn new({param_list}) -> Self {{\n        Self {{ {assignments} }}\n    }}",
        cfg.constructor_attr
    )
    .ok();
    out
}

/// Generate an instance method.
pub fn gen_method(method: &MethodDef, mapper: &dyn TypeMapper, cfg: &RustBindingConfig, type_name: &str) -> String {
    let map_fn = |ty: &skif_core::ir::TypeRef| mapper.map_type(ty);
    let params = function_params(&method.params, &map_fn);
    let return_type = mapper.map_type(&method.return_type);
    let ret = mapper.wrap_return(&return_type, method.error_type.is_some());

    let call_args = gen_call_args(&method.params);
    let core_import = cfg.core_import;

    // Generate the body: convert self to core type, call method, convert result back
    let body = if method.is_async {
        match cfg.async_pattern {
            AsyncPattern::Pyo3FutureIntoPy => {
                let core_call = format!(
                    "{core_import}::{type_name}::from(self.clone()).{method_name}({call_args})",
                    method_name = method.name
                );
                let result_handling = if method.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n            \
                         .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;"
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "pyo3_async_runtimes::tokio::future_into_py(py, async move {{\n            \
                     {result_handling}\n            \
                     Ok({return_type}::from(result))\n        }})"
                )
            }
            AsyncPattern::WasmNativeAsync => {
                let core_call = format!(
                    "{core_import}::{type_name}::from(self.clone()).{method_name}({call_args})",
                    method_name = method.name
                );
                let result_handling = if method.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n        \
                         .map_err(|e| JsValue::from_str(&e.to_string()))?;"
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "{result_handling}\n        \
                     Ok({return_type}::from(result))"
                )
            }
            AsyncPattern::NapiNativeAsync => {
                let core_call = format!(
                    "{core_import}::{type_name}::from(self.clone()).{method_name}({call_args})",
                    method_name = method.name
                );
                let result_handling = if method.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n            \
                         .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;"
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "{result_handling}\n            \
                     Ok({return_type}::from(result))"
                )
            }
            AsyncPattern::TokioBlockOn => {
                let core_call = format!(
                    "{core_import}::{type_name}::from(self.clone()).{method_name}({call_args})",
                    method_name = method.name
                );
                if method.error_type.is_some() {
                    format!(
                        "let rt = tokio::runtime::Runtime::new()?;\n        \
                         rt.block_on(async {{ {core_call}.await.map_err(|e| e.into()) }})"
                    )
                } else {
                    format!(
                        "let rt = tokio::runtime::Runtime::new()?;\n        \
                         rt.block_on(async {{ {core_call}.await }})"
                    )
                }
            }
            AsyncPattern::None => "todo!(\"async not supported by backend\")".to_string(),
        }
    } else {
        let core_call = format!(
            "{}::{}::from(self.clone()).{}({call_args})",
            core_import, type_name, method.name
        );
        if method.error_type.is_some() {
            format!("{core_call}.map_err(|e| e.into())")
        } else {
            core_call
        }
    };

    let self_param = if params.is_empty() && !method.is_async {
        "&self"
    } else if params.is_empty() && method.is_async && cfg.async_pattern == AsyncPattern::Pyo3FutureIntoPy {
        "py: Python<'_>, &self"
    } else if method.is_async && cfg.async_pattern == AsyncPattern::Pyo3FutureIntoPy {
        "py: Python<'_>, &self, "
    } else if params.is_empty() {
        "&self"
    } else {
        "&self, "
    };

    // Wrap long signature if necessary
    let (sig_start, sig_params, sig_end) = if self_param.len() + params.len() > 100 {
        let wrapped_params = method
            .params
            .iter()
            .map(|p| {
                let ty = if p.optional {
                    format!("Option<{}>", mapper.map_type(&p.ty))
                } else {
                    mapper.map_type(&p.ty)
                };
                format!("{}: {}", p.name, ty)
            })
            .collect::<Vec<_>>()
            .join(",\n        ");
        (
            format!("pub fn {}(\n        &self,\n        ", method.name),
            wrapped_params,
            "\n    ) -> ".to_string(),
        )
    } else {
        (
            format!("pub fn {}({}", method.name, self_param),
            params,
            ") -> ".to_string(),
        )
    };

    let mut out = String::new();
    if cfg.needs_signature {
        let sig = function_sig_defaults(&method.params);
        writeln!(out, "    {}{}{}", cfg.signature_prefix, sig, cfg.signature_suffix).ok();
    }
    write!(
        out,
        "    {}{}{}{} {{\n        \
         {body}\n    }}",
        sig_start, sig_params, sig_end, ret,
    )
    .ok();
    out
}

/// Generate a static method.
pub fn gen_static_method(
    method: &MethodDef,
    mapper: &dyn TypeMapper,
    cfg: &RustBindingConfig,
    type_name: &str,
) -> String {
    let map_fn = |ty: &skif_core::ir::TypeRef| mapper.map_type(ty);
    let params = function_params(&method.params, &map_fn);
    let return_type = mapper.map_type(&method.return_type);
    let ret = mapper.wrap_return(&return_type, method.error_type.is_some());

    let call_args = gen_call_args(&method.params);
    let core_import = cfg.core_import;

    let body = if method.is_async {
        match cfg.async_pattern {
            AsyncPattern::Pyo3FutureIntoPy => {
                let core_call = format!("{core_import}::{type_name}::{}({call_args})", method.name);
                let result_handling = if method.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n            \
                         .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;"
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "pyo3_async_runtimes::tokio::future_into_py(py, async move {{\n            \
                     {result_handling}\n            \
                     Ok({return_type}::from(result))\n        }})"
                )
            }
            AsyncPattern::WasmNativeAsync => {
                let core_call = format!("{core_import}::{type_name}::{}({call_args})", method.name);
                let result_handling = if method.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n        \
                         .map_err(|e| JsValue::from_str(&e.to_string()))?;"
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "{result_handling}\n        \
                     Ok({return_type}::from(result))"
                )
            }
            AsyncPattern::NapiNativeAsync => {
                let core_call = format!("{core_import}::{type_name}::{}({call_args})", method.name);
                let result_handling = if method.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n            \
                         .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;"
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "{result_handling}\n            \
                     Ok({return_type}::from(result))"
                )
            }
            AsyncPattern::TokioBlockOn => {
                let core_call = format!("{core_import}::{type_name}::{}({call_args})", method.name);
                if method.error_type.is_some() {
                    format!(
                        "let rt = tokio::runtime::Runtime::new()?;\n        \
                         rt.block_on(async {{ {core_call}.await.map_err(|e| e.into()) }})"
                    )
                } else {
                    format!(
                        "let rt = tokio::runtime::Runtime::new()?;\n        \
                         rt.block_on(async {{ {core_call}.await }})"
                    )
                }
            }
            AsyncPattern::None => "todo!(\"async not supported by backend\")".to_string(),
        }
    } else {
        let core_call = format!("{core_import}::{type_name}::{}({call_args})", method.name);
        if method.error_type.is_some() {
            format!("{core_call}.map_err(|e| e.into())")
        } else {
            core_call
        }
    };

    // Wrap long signature if necessary
    let (sig_start, sig_params, sig_end) = if params.len() > 100 {
        let wrapped_params = method
            .params
            .iter()
            .map(|p| {
                let ty = if p.optional {
                    format!("Option<{}>", mapper.map_type(&p.ty))
                } else {
                    mapper.map_type(&p.ty)
                };
                format!("{}: {}", p.name, ty)
            })
            .collect::<Vec<_>>()
            .join(",\n        ");
        // For async PyO3, add py parameter
        if method.is_async && cfg.async_pattern == AsyncPattern::Pyo3FutureIntoPy {
            (
                format!("pub fn {}(py: Python<'_>,\n        ", method.name),
                wrapped_params,
                "\n    ) -> ".to_string(),
            )
        } else {
            (
                format!("pub fn {}(\n        ", method.name),
                wrapped_params,
                "\n    ) -> ".to_string(),
            )
        }
    } else {
        if method.is_async && cfg.async_pattern == AsyncPattern::Pyo3FutureIntoPy {
            (
                format!("pub fn {}(py: Python<'_>, ", method.name),
                params,
                ") -> ".to_string(),
            )
        } else {
            (format!("pub fn {}(", method.name), params, ") -> ".to_string())
        }
    };

    let mut out = String::new();
    if let Some(attr) = cfg.static_attr {
        writeln!(out, "    #[{attr}]").ok();
    }
    if cfg.needs_signature {
        let sig = function_sig_defaults(&method.params);
        writeln!(out, "    {}{}{}", cfg.signature_prefix, sig, cfg.signature_suffix).ok();
    }
    write!(
        out,
        "    {}{}{}{} {{\n        \
         {body}\n    }}",
        sig_start, sig_params, sig_end, ret,
    )
    .ok();
    out
}

/// Generate an enum.
pub fn gen_enum(enum_def: &EnumDef, cfg: &RustBindingConfig) -> String {
    let mut out = String::new();
    if !cfg.enum_derives.is_empty() {
        writeln!(out, "#[derive({})]", cfg.enum_derives.join(", ")).ok();
    }
    for attr in cfg.enum_attrs {
        writeln!(out, "#[{attr}]").ok();
    }
    writeln!(out, "pub enum {} {{", enum_def.name).ok();
    for (idx, variant) in enum_def.variants.iter().enumerate() {
        writeln!(out, "    {} = {idx},", variant.name).ok();
    }
    write!(out, "}}").ok();
    out
}

/// Generate a free function.
pub fn gen_function(func: &FunctionDef, mapper: &dyn TypeMapper, cfg: &RustBindingConfig) -> String {
    let map_fn = |ty: &skif_core::ir::TypeRef| mapper.map_type(ty);
    let params = function_params(&func.params, &map_fn);
    let return_type = mapper.map_type(&func.return_type);
    let ret = mapper.wrap_return(&return_type, func.error_type.is_some());

    let call_args = gen_call_args(&func.params);
    let core_import = cfg.core_import;

    // Generate the body based on async pattern
    let body = if func.is_async {
        match cfg.async_pattern {
            AsyncPattern::Pyo3FutureIntoPy => {
                let core_call = format!("{core_import}::{}({call_args})", func.name);
                let result_handling = if func.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n            \
                         .map_err(|e| PyErr::new::<PyRuntimeError, _>(e.to_string()))?;",
                        core_call = core_call
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "pyo3_async_runtimes::tokio::future_into_py(py, async move {{\n            \
                     {result_handling}\n            \
                     Ok({return_type}::from(result))\n        }})"
                )
            }
            AsyncPattern::WasmNativeAsync => {
                let core_call = format!("{core_import}::{}({call_args})", func.name);
                let result_handling = if func.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n        \
                         .map_err(|e| JsValue::from_str(&e.to_string()))?;",
                        core_call = core_call
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "{result_handling}\n        \
                     Ok({return_type}::from(result))"
                )
            }
            AsyncPattern::NapiNativeAsync => {
                let core_call = format!("{core_import}::{}({call_args})", func.name);
                let result_handling = if func.error_type.is_some() {
                    format!(
                        "let result = {core_call}.await\n            \
                         .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;",
                        core_call = core_call
                    )
                } else {
                    format!("let result = {core_call}.await;")
                };
                format!(
                    "{result_handling}\n            \
                     Ok({return_type}::from(result))"
                )
            }
            AsyncPattern::TokioBlockOn => {
                let core_call = format!("{core_import}::{}({call_args})", func.name);

                if func.error_type.is_some() {
                    format!(
                        "let rt = tokio::runtime::Runtime::new()?;\n    \
                         rt.block_on(async {{ {core_call}.await.map_err(|e| e.into()) }})"
                    )
                } else {
                    format!(
                        "let rt = tokio::runtime::Runtime::new()?;\n    \
                         rt.block_on(async {{ {core_call}.await }})"
                    )
                }
            }
            AsyncPattern::None => "todo!(\"async not supported by backend\")".to_string(),
        }
    } else {
        let core_call = format!("{core_import}::{}({call_args})", func.name);
        if func.error_type.is_some() {
            format!("{core_call}.map_err(|e| e.into())")
        } else {
            core_call
        }
    };

    // Wrap long signature if necessary
    let async_kw = if func.is_async { "async " } else { "" };
    let (func_sig, _params_formatted) = if params.len() > 100 {
        let wrapped_params = func
            .params
            .iter()
            .map(|p| {
                let ty = if p.optional {
                    format!("Option<{}>", mapper.map_type(&p.ty))
                } else {
                    mapper.map_type(&p.ty)
                };
                format!("{}: {}", p.name, ty)
            })
            .collect::<Vec<_>>()
            .join(",\n    ");

        // For async PyO3, we need special signature handling
        if func.is_async && cfg.async_pattern == AsyncPattern::Pyo3FutureIntoPy {
            (
                format!(
                    "pub fn {}(py: Python<'_>,\n    {}\n) -> {ret}",
                    func.name,
                    wrapped_params,
                    ret = ret
                ),
                "",
            )
        } else {
            (
                format!(
                    "pub {async_kw}fn {}(\n    {}\n) -> {ret}",
                    func.name,
                    wrapped_params,
                    ret = ret
                ),
                "",
            )
        }
    } else {
        if func.is_async && cfg.async_pattern == AsyncPattern::Pyo3FutureIntoPy {
            (format!("pub fn {}(py: Python<'_>, {params}) -> {ret}", func.name), "")
        } else {
            (format!("pub {async_kw}fn {}({params}) -> {ret}", func.name), "")
        }
    };

    let mut out = String::new();
    let attr_inner = cfg
        .function_attr
        .trim_start_matches('#')
        .trim_start_matches('[')
        .trim_end_matches(']');
    writeln!(out, "#[{attr_inner}]").ok();
    if cfg.needs_signature {
        let sig = function_sig_defaults(&func.params);
        writeln!(out, "{}{}{}", cfg.signature_prefix, sig, cfg.signature_suffix).ok();
    }
    write!(out, "{} {{\n    {body}\n}}", func_sig,).ok();
    out
}

/// Generate a full methods impl block.
pub fn gen_impl_block(typ: &TypeDef, mapper: &dyn TypeMapper, cfg: &RustBindingConfig) -> String {
    let (instance, statics) = partition_methods(&typ.methods);
    if instance.is_empty() && statics.is_empty() && typ.fields.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    if let Some(block_attr) = cfg.method_block_attr {
        writeln!(out, "#[{block_attr}]").ok();
    }
    writeln!(out, "impl {} {{", typ.name).ok();

    // Constructor
    if !typ.fields.is_empty() {
        out.push_str(&gen_constructor(typ, mapper, cfg));
        out.push_str("\n\n");
    }

    // Instance methods
    for m in &instance {
        out.push_str(&gen_method(m, mapper, cfg, &typ.name));
        out.push_str("\n\n");
    }

    // Static methods
    for m in &statics {
        out.push_str(&gen_static_method(m, mapper, cfg, &typ.name));
        out.push_str("\n\n");
    }

    // Trim trailing newlines inside impl block
    let trimmed = out.trim_end();
    let mut result = trimmed.to_string();
    result.push_str("\n}");
    result
}
