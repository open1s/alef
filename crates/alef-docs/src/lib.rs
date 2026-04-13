//! API reference documentation generator for alef polyglot bindings.
//!
//! Generates per-language `api-{lang}.md` files plus shared `configuration.md`
//! and `errors.md` files from the alef IR (`ApiSurface`).

use alef_core::backend::GeneratedFile;
use alef_core::config::{AlefConfig, Language};
use alef_core::ir::{
    ApiSurface, DefaultValue, EnumDef, ErrorDef, FieldDef, FunctionDef, MethodDef, PrimitiveType, TypeDef, TypeRef,
};
use heck::{ToPascalCase, ToSnakeCase, ToUpperCamelCase};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate API reference documentation for the given languages.
///
/// Produces one `api-{lang}.md` per language, plus shared `configuration.md`
/// and `errors.md` files written into `output_dir`.
pub fn generate_docs(
    api: &ApiSurface,
    config: &AlefConfig,
    languages: &[Language],
    output_dir: &str,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let mut files = Vec::new();

    for &lang in languages {
        files.push(generate_lang_doc(api, config, lang, output_dir)?);
    }

    files.push(generate_configuration_doc(api, config, output_dir)?);
    files.push(generate_errors_doc(api, output_dir)?);

    Ok(files)
}

// ---------------------------------------------------------------------------
// Per-language doc page
// ---------------------------------------------------------------------------

fn generate_lang_doc(
    api: &ApiSurface,
    config: &AlefConfig,
    lang: Language,
    output_dir: &str,
) -> anyhow::Result<GeneratedFile> {
    let lang_display = lang_display_name(lang);
    let version = &api.version;
    let lang_slug = lang_slug(lang);

    let mut out = String::new();

    // Front matter
    out.push_str(&format!("---\ntitle: \"{lang_display} API Reference\"\n---\n\n"));

    // Title
    out.push_str(&format!(
        "# {lang_display} API Reference <span class=\"version-badge\">v{version}</span>\n\n"
    ));

    // --- Functions section ---
    let public_fns: Vec<&FunctionDef> = api.functions.iter().collect();
    if !public_fns.is_empty() {
        out.push_str("## Functions\n\n");
        for func in &public_fns {
            out.push_str(&render_function(func, lang, config));
            out.push_str("\n---\n\n");
        }
    }

    // --- Types section ---
    // Order: ConversionOptions, ConversionResult, then rest alphabetical
    // Skip opaque types and *Update types in main section
    let mut types_to_doc: Vec<&TypeDef> = api.types.iter().filter(|t| !is_update_type(&t.name)).collect();

    // Sort: ConversionOptions first, ConversionResult second, rest alphabetical
    types_to_doc.sort_by(|a, b| type_sort_key(&a.name).cmp(&type_sort_key(&b.name)));

    if !types_to_doc.is_empty() {
        out.push_str("## Types\n\n");
        for ty in &types_to_doc {
            out.push_str(&render_type(ty, lang));
            out.push_str("\n---\n\n");
        }
    }

    // --- Enums section ---
    if !api.enums.is_empty() {
        out.push_str("## Enums\n\n");
        for en in &api.enums {
            out.push_str(&render_enum(en, lang));
            out.push_str("\n---\n\n");
        }
    }

    // --- Errors section ---
    if !api.errors.is_empty() {
        out.push_str("## Errors\n\n");
        for err in &api.errors {
            out.push_str(&render_error(err, lang));
            out.push_str("\n---\n\n");
        }
    }

    let path = PathBuf::from(format!("{output_dir}/api-{lang_slug}.md"));

    Ok(GeneratedFile {
        path,
        content: out,
        generated_header: false,
    })
}

// ---------------------------------------------------------------------------
// Function rendering
// ---------------------------------------------------------------------------

fn render_function(func: &FunctionDef, lang: Language, _config: &AlefConfig) -> String {
    let mut out = String::new();
    let fn_name = func_name(&func.name, lang);

    out.push_str(&format!("### {fn_name}()\n\n"));

    if !func.doc.is_empty() {
        out.push_str(&clean_doc(&func.doc));
        out.push('\n');
        out.push('\n');
    }

    // Signature
    out.push_str("**Signature:**\n\n");
    let lang_code = lang_code_fence(lang);
    let sig = render_function_signature(func, lang);
    out.push_str(&format!("```{lang_code}\n{sig}\n```\n\n"));

    // Parameters table
    if !func.params.is_empty() {
        out.push_str("**Parameters:**\n\n");
        out.push_str("| Name | Type | Required | Description |\n");
        out.push_str("|------|------|----------|-------------|\n");
        for param in &func.params {
            let pname = field_name(&param.name, lang);
            let pty = doc_type(&param.ty, lang);
            let required = if param.optional { "No" } else { "Yes" };
            let pdoc = String::new(); // ParamDef has no doc field
            out.push_str(&format!("| `{pname}` | `{pty}` | {required} | {pdoc} |\n"));
        }
        out.push('\n');
    }

    // Return type
    let ret_ty = doc_type(&func.return_type, lang);
    out.push_str(&format!("**Returns:** `{ret_ty}`"));
    out.push('\n');
    out.push('\n');

    // Errors
    if let Some(err) = &func.error_type {
        let err_name = type_name(err, lang);
        out.push_str(&format!("**Errors:** Throws `{err_name}` on failure.\n\n"));
    }

    out
}

fn render_function_signature(func: &FunctionDef, lang: Language) -> String {
    match lang {
        Language::Python => render_python_fn_sig(func),
        Language::Node | Language::Wasm => render_typescript_fn_sig(func),
        Language::Go => render_go_fn_sig(func),
        Language::Java => render_java_fn_sig(func),
        Language::Ruby => render_ruby_fn_sig(func),
        Language::Ffi => render_c_fn_sig(func),
        Language::Php => render_php_fn_sig(func),
        Language::Elixir => render_elixir_fn_sig(func),
        Language::R => render_r_fn_sig(func),
        Language::Csharp => render_csharp_fn_sig(func),
    }
}

fn render_python_fn_sig(func: &FunctionDef) -> String {
    let name = func.name.to_snake_case();
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = p.name.to_snake_case();
            let pty = doc_type(&p.ty, Language::Python);
            if p.optional {
                format!("{pname}: {pty} = None")
            } else {
                format!("{pname}: {pty}")
            }
        })
        .collect();
    let ret = doc_type(&func.return_type, Language::Python);
    if func.is_async {
        format!("async def {}({}) -> {}", name, params.join(", "), ret)
    } else {
        format!("def {}({}) -> {}", name, params.join(", "), ret)
    }
}

fn render_typescript_fn_sig(func: &FunctionDef) -> String {
    let name = to_camel_case(&func.name);
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = to_camel_case(&p.name);
            let pty = doc_type(&p.ty, Language::Node);
            if p.optional {
                format!("{pname}?: {pty}")
            } else {
                format!("{pname}: {pty}")
            }
        })
        .collect();
    let ret = doc_type(&func.return_type, Language::Node);
    if func.is_async {
        format!("function {}({}): Promise<{}>", name, params.join(", "), ret)
    } else {
        format!("function {}({}): {}", name, params.join(", "), ret)
    }
}

fn render_go_fn_sig(func: &FunctionDef) -> String {
    let name = func.name.to_pascal_case();
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = to_camel_case(&p.name);
            let pty = doc_type(&p.ty, Language::Go);
            format!("{pname} {pty}")
        })
        .collect();
    let ret = doc_type(&func.return_type, Language::Go);
    if func.error_type.is_some() {
        format!("func {}({}) ({}, error)", name, params.join(", "), ret)
    } else {
        format!("func {}({}) {}", name, params.join(", "), ret)
    }
}

fn render_java_fn_sig(func: &FunctionDef) -> String {
    let name = to_camel_case(&func.name);
    let ret = doc_type(&func.return_type, Language::Java);
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = to_camel_case(&p.name);
            let pty = doc_type(&p.ty, Language::Java);
            format!("{pty} {pname}")
        })
        .collect();
    let throws = func
        .error_type
        .as_ref()
        .map(|e| format!(" throws {}", type_name(e, Language::Java)))
        .unwrap_or_default();
    format!("public static {} {}({}){}", ret, name, params.join(", "), throws)
}

fn render_ruby_fn_sig(func: &FunctionDef) -> String {
    let name = func.name.to_snake_case();
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = p.name.to_snake_case();
            if p.optional { format!("{pname}: nil") } else { pname }
        })
        .collect();
    format!("def self.{}({})", name, params.join(", "))
}

fn render_c_fn_sig(func: &FunctionDef) -> String {
    let prefix = "htm";
    let name = format!("{}_{}", prefix, func.name.to_snake_case());
    let ret = doc_type(&func.return_type, Language::Ffi);
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = p.name.to_snake_case();
            let pty = doc_type(&p.ty, Language::Ffi);
            format!("{pty} {pname}")
        })
        .collect();
    format!("{}* {}({});", ret, name, params.join(", "))
}

fn render_php_fn_sig(func: &FunctionDef) -> String {
    let name = to_camel_case(&func.name);
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = format!("${}", p.name.to_snake_case());
            let pty = doc_type(&p.ty, Language::Php);
            if p.optional {
                format!("?{pty} {pname} = null")
            } else {
                format!("{pty} {pname}")
            }
        })
        .collect();
    let ret = doc_type(&func.return_type, Language::Php);
    format!("public static function {}({}): {}", name, params.join(", "), ret)
}

fn render_elixir_fn_sig(func: &FunctionDef) -> String {
    let name = func.name.to_snake_case();
    let params: Vec<String> = func.params.iter().map(|p| p.name.to_snake_case()).collect();
    format!(
        "@spec {}({}) :: {{:ok, term()}} | {{:error, term()}}\ndef {}({})",
        name,
        params.join(", "),
        name,
        params.join(", ")
    )
}

fn render_r_fn_sig(func: &FunctionDef) -> String {
    let name = func.name.to_snake_case();
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = p.name.to_snake_case();
            if p.optional { format!("{pname} = NULL") } else { pname }
        })
        .collect();
    format!("{}({})", name, params.join(", "))
}

fn render_csharp_fn_sig(func: &FunctionDef) -> String {
    let name = func.name.to_pascal_case();
    let ret = doc_type(&func.return_type, Language::Csharp);
    let params: Vec<String> = func
        .params
        .iter()
        .map(|p| {
            let pname = to_camel_case(&p.name);
            let pty = doc_type(&p.ty, Language::Csharp);
            if p.optional {
                format!("{pty}? {pname} = null")
            } else {
                format!("{pty} {pname}")
            }
        })
        .collect();
    if func.is_async {
        format!("public static async Task<{}> {}Async({})", ret, name, params.join(", "))
    } else {
        format!("public static {} {}({})", ret, name, params.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Type rendering
// ---------------------------------------------------------------------------

fn render_type(ty: &TypeDef, lang: Language) -> String {
    let mut out = String::new();
    let tname = type_name(&ty.name, lang);

    out.push_str(&format!("### {tname}\n\n"));

    let doc = clean_doc(&ty.doc);
    if !doc.is_empty() {
        out.push_str(&doc);
        out.push('\n');
        out.push('\n');
    }

    // Fields table (only for non-opaque types or opaque types with documented fields)
    if !ty.is_opaque && !ty.fields.is_empty() {
        out.push_str("| Field | Type | Default | Description |\n");
        out.push_str("|-------|------|---------|-------------|\n");
        for field in &ty.fields {
            let fname = field_name(&field.name, lang);
            let fty = doc_type(&field.ty, lang);
            let fdefault = format_field_default(field, lang);
            let fdoc = clean_doc_inline(&field.doc);
            out.push_str(&format!("| `{fname}` | `{fty}` | {fdefault} | {fdoc} |\n"));
        }
        out.push('\n');
    }

    // Methods
    if !ty.methods.is_empty() {
        out.push_str("#### Methods\n\n");
        for method in &ty.methods {
            out.push_str(&render_method(method, &ty.name, lang));
        }
    }

    out
}

fn render_method(method: &MethodDef, _type_name_str: &str, lang: Language) -> String {
    let mut out = String::new();
    let mname = func_name(&method.name, lang);

    out.push_str(&format!("##### {mname}()\n\n"));

    let doc = clean_doc(&method.doc);
    if !doc.is_empty() {
        out.push_str(&doc);
        out.push('\n');
        out.push('\n');
    }

    let lang_code = lang_code_fence(lang);
    let sig = render_method_signature(method, lang);
    out.push_str("**Signature:**\n\n");
    out.push_str(&format!("```{lang_code}\n{sig}\n```\n\n"));

    out
}

fn render_method_signature(method: &MethodDef, lang: Language) -> String {
    let name = func_name(&method.name, lang);
    let params: Vec<String> = method
        .params
        .iter()
        .map(|p| {
            let pname = field_name(&p.name, lang);
            let pty = doc_type(&p.ty, lang);
            format!("{pname}: {pty}")
        })
        .collect();
    let ret = doc_type(&method.return_type, lang);

    match lang {
        Language::Python => {
            if method.is_static {
                format!("@staticmethod\ndef {}({}) -> {}", name, params.join(", "), ret)
            } else {
                let mut all_params = vec!["self".to_string()];
                all_params.extend(params);
                format!("def {}({}) -> {}", name, all_params.join(", "), ret)
            }
        }
        Language::Node | Language::Wasm => {
            if method.is_static {
                format!("static {}({}): {}", name, params.join(", "), ret)
            } else {
                format!("{}({}): {}", name, params.join(", "), ret)
            }
        }
        Language::Ruby => {
            if method.is_static {
                format!("def self.{}({})", name, params.join(", "))
            } else {
                format!("def {}({})", name, params.join(", "))
            }
        }
        Language::Go => format!("func ({}) {} {} {}", "o *Object", name, params.join(", "), ret),
        Language::Java => format!("public {} {}({})", ret, name, params.join(", ")),
        Language::Csharp => format!("public {} {}({})", ret, name, params.join(", ")),
        Language::Php => format!("public function {}({}): {}", name, params.join(", "), ret),
        Language::Elixir => format!("def {}({})", name, params.join(", ")),
        Language::R => format!("{}({})", name, params.join(", ")),
        Language::Ffi => format!("{} {}({});", ret, name, params.join(", ")),
    }
}

// ---------------------------------------------------------------------------
// Enum rendering
// ---------------------------------------------------------------------------

fn render_enum(en: &EnumDef, lang: Language) -> String {
    let mut out = String::new();
    let ename = type_name(&en.name, lang);

    out.push_str(&format!("### {ename}\n\n"));

    let doc = clean_doc(&en.doc);
    if !doc.is_empty() {
        out.push_str(&doc);
        out.push('\n');
        out.push('\n');
    }

    out.push_str("| Value | Description |\n");
    out.push_str("|-------|-------------|\n");
    for variant in &en.variants {
        let vname = enum_variant_name(&variant.name, lang);
        let vdoc = clean_doc_inline(&variant.doc);
        out.push_str(&format!("| `{vname}` | {vdoc} |\n"));
    }
    out.push('\n');

    out
}

// ---------------------------------------------------------------------------
// Error rendering
// ---------------------------------------------------------------------------

fn render_error(err: &ErrorDef, lang: Language) -> String {
    let mut out = String::new();
    let ename = type_name(&err.name, lang);

    out.push_str(&format!("### {ename}\n\n"));

    let doc = clean_doc(&err.doc);
    if !doc.is_empty() {
        out.push_str(&doc);
        out.push('\n');
        out.push('\n');
    }

    out.push_str("| Variant | Description |\n");
    out.push_str("|---------|-------------|\n");
    for variant in &err.variants {
        let vname = enum_variant_name(&variant.name, lang);
        let vdoc = if !variant.doc.is_empty() {
            clean_doc_inline(&variant.doc)
        } else if let Some(tmpl) = &variant.message_template {
            clean_doc_inline(tmpl)
        } else {
            String::new()
        };
        out.push_str(&format!("| `{vname}` | {vdoc} |\n"));
    }
    out.push('\n');

    out
}

// ---------------------------------------------------------------------------
// Configuration page
// ---------------------------------------------------------------------------

fn generate_configuration_doc(
    api: &ApiSurface,
    _config: &AlefConfig,
    output_dir: &str,
) -> anyhow::Result<GeneratedFile> {
    let mut out = String::new();

    out.push_str("---\ntitle: \"Configuration Reference\"\n---\n\n");
    out.push_str("# Configuration Reference\n\n");
    out.push_str("This page documents all configuration types and their defaults across all languages.\n\n");

    // Collect config-like types (ConversionOptions, PreprocessingOptions, etc.)
    let config_types: Vec<&TypeDef> = api
        .types
        .iter()
        .filter(|t| t.name.ends_with("Options") && !t.is_opaque && !is_update_type(&t.name))
        .collect();

    for ty in config_types {
        out.push_str(&format!("## {}\n\n", ty.name));
        let doc = clean_doc(&ty.doc);
        if !doc.is_empty() {
            out.push_str(&doc);
            out.push('\n');
            out.push('\n');
        }

        if !ty.fields.is_empty() {
            out.push_str("| Field | Type | Default | Description |\n");
            out.push_str("|-------|------|---------|-------------|\n");
            for field in &ty.fields {
                let fty = doc_type(&field.ty, Language::Python); // Use Python as canonical type display
                let fdefault = format_field_default(field, Language::Python);
                let fdoc = clean_doc_inline(&field.doc);
                out.push_str(&format!("| `{}` | `{}` | {} | {} |\n", field.name, fty, fdefault, fdoc));
            }
            out.push('\n');
        }

        out.push_str("---\n\n");
    }

    Ok(GeneratedFile {
        path: PathBuf::from(format!("{output_dir}/configuration.md")),
        content: out,
        generated_header: false,
    })
}

// ---------------------------------------------------------------------------
// Errors page
// ---------------------------------------------------------------------------

fn generate_errors_doc(api: &ApiSurface, output_dir: &str) -> anyhow::Result<GeneratedFile> {
    let mut out = String::new();

    out.push_str("---\ntitle: \"Error Reference\"\n---\n\n");
    out.push_str("# Error Reference\n\n");
    out.push_str("All error types thrown by the library across all languages.\n\n");

    for err in &api.errors {
        out.push_str(&format!("## {}\n\n", err.name));

        let doc = clean_doc(&err.doc);
        if !doc.is_empty() {
            out.push_str(&doc);
            out.push('\n');
            out.push('\n');
        }

        out.push_str("| Variant | Message | Description |\n");
        out.push_str("|---------|---------|-------------|\n");
        for variant in &err.variants {
            let tmpl = variant.message_template.as_deref().unwrap_or("").replace('|', "\\|");
            let vdoc = clean_doc_inline(&variant.doc);
            out.push_str(&format!("| `{}` | {} | {} |\n", variant.name, tmpl, vdoc));
        }
        out.push('\n');
        out.push_str("---\n\n");
    }

    Ok(GeneratedFile {
        path: PathBuf::from(format!("{output_dir}/errors.md")),
        content: out,
        generated_header: false,
    })
}

// ---------------------------------------------------------------------------
// Type mapping
// ---------------------------------------------------------------------------

/// Map an IR TypeRef to the idiomatic type string for a given language.
pub fn doc_type(ty: &TypeRef, lang: Language) -> String {
    match ty {
        TypeRef::String | TypeRef::Char => match lang {
            Language::Python => "str".to_string(),
            Language::Node | Language::Wasm => "string".to_string(),
            Language::Go => "string".to_string(),
            Language::Java => "String".to_string(),
            Language::Csharp => "string".to_string(),
            Language::Ruby => "String".to_string(),
            Language::Php => "string".to_string(),
            Language::Elixir => "String.t()".to_string(),
            Language::R => "character".to_string(),
            Language::Ffi => "const char*".to_string(),
        },
        TypeRef::Bytes => match lang {
            Language::Python => "bytes".to_string(),
            Language::Node | Language::Wasm => "Buffer".to_string(),
            Language::Go => "[]byte".to_string(),
            Language::Java => "byte[]".to_string(),
            Language::Csharp => "byte[]".to_string(),
            Language::Ruby => "String".to_string(),
            Language::Php => "string".to_string(),
            Language::Elixir => "binary()".to_string(),
            Language::R => "raw".to_string(),
            Language::Ffi => "const uint8_t*".to_string(),
        },
        TypeRef::Primitive(p) => doc_primitive(p, lang),
        TypeRef::Optional(inner) => {
            let inner_ty = doc_type(inner, lang);
            match lang {
                Language::Python => format!("{inner_ty} | None"),
                Language::Node | Language::Wasm => format!("{inner_ty} | null"),
                Language::Go => format!("*{inner_ty}"),
                Language::Java => format!("Optional<{inner_ty}>"),
                Language::Csharp => format!("{inner_ty}?"),
                Language::Ruby => format!("{inner_ty}?"),
                Language::Php => format!("?{inner_ty}"),
                Language::Elixir => format!("{inner_ty} | nil"),
                Language::R => format!("{inner_ty} or NULL"),
                Language::Ffi => format!("{inner_ty}*"),
            }
        }
        TypeRef::Vec(inner) => {
            let inner_ty = doc_type(inner, lang);
            match lang {
                Language::Python => format!("list[{inner_ty}]"),
                Language::Node | Language::Wasm => format!("Array<{inner_ty}>"),
                Language::Go => format!("[]{inner_ty}"),
                Language::Java => format!("List<{inner_ty}>"),
                Language::Csharp => format!("List<{inner_ty}>"),
                Language::Ruby => format!("Array<{inner_ty}>"),
                Language::Php => format!("array<{inner_ty}>"),
                Language::Elixir => format!("list({inner_ty})"),
                Language::R => "list".to_string(),
                Language::Ffi => format!("{inner_ty}*"),
            }
        }
        TypeRef::Map(k, v) => {
            let kty = doc_type(k, lang);
            let vty = doc_type(v, lang);
            match lang {
                Language::Python => format!("dict[{kty}, {vty}]"),
                Language::Node | Language::Wasm => format!("Record<{kty}, {vty}>"),
                Language::Go => format!("map[{kty}]{vty}"),
                Language::Java => format!("Map<{kty}, {vty}>"),
                Language::Csharp => format!("Dictionary<{kty}, {vty}>"),
                Language::Ruby => format!("Hash{{{kty}=>{vty}}}"),
                Language::Php => format!("array<{kty}, {vty}>"),
                Language::Elixir => "map()".to_string(),
                Language::R => "list".to_string(),
                Language::Ffi => "void*".to_string(),
            }
        }
        TypeRef::Named(name) => type_name(name, lang),
        TypeRef::Path => match lang {
            Language::Python => "str".to_string(),
            Language::Node | Language::Wasm => "string".to_string(),
            Language::Go => "string".to_string(),
            Language::Java => "String".to_string(),
            Language::Csharp => "string".to_string(),
            Language::Ruby => "String".to_string(),
            Language::Php => "string".to_string(),
            Language::Elixir => "String.t()".to_string(),
            Language::R => "character".to_string(),
            Language::Ffi => "const char*".to_string(),
        },
        TypeRef::Unit => match lang {
            Language::Python => "None".to_string(),
            Language::Node | Language::Wasm => "void".to_string(),
            Language::Go => "".to_string(),
            Language::Java => "void".to_string(),
            Language::Csharp => "void".to_string(),
            Language::Ruby => "nil".to_string(),
            Language::Php => "void".to_string(),
            Language::Elixir => ":ok".to_string(),
            Language::R => "NULL".to_string(),
            Language::Ffi => "void".to_string(),
        },
        TypeRef::Json => match lang {
            Language::Python => "Any".to_string(),
            Language::Node | Language::Wasm => "unknown".to_string(),
            Language::Go => "interface{}".to_string(),
            Language::Java => "Object".to_string(),
            Language::Csharp => "object".to_string(),
            Language::Ruby => "Object".to_string(),
            Language::Php => "mixed".to_string(),
            Language::Elixir => "term()".to_string(),
            Language::R => "list".to_string(),
            Language::Ffi => "void*".to_string(),
        },
        TypeRef::Duration => match lang {
            Language::Python => "float".to_string(),
            Language::Node | Language::Wasm => "number".to_string(),
            Language::Go => "time.Duration".to_string(),
            Language::Java => "Duration".to_string(),
            Language::Csharp => "TimeSpan".to_string(),
            Language::Ruby => "Float".to_string(),
            Language::Php => "float".to_string(),
            Language::Elixir => "integer()".to_string(),
            Language::R => "numeric".to_string(),
            Language::Ffi => "uint64_t".to_string(),
        },
    }
}

fn doc_primitive(p: &PrimitiveType, lang: Language) -> String {
    match lang {
        Language::Python => match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "float".to_string(),
            _ => "int".to_string(),
        },
        Language::Node | Language::Wasm => match p {
            PrimitiveType::Bool => "boolean".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "number".to_string(),
            PrimitiveType::U64 | PrimitiveType::I64 => "bigint".to_string(),
            _ => "number".to_string(),
        },
        Language::Go => match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::U8 => "uint8".to_string(),
            PrimitiveType::U16 => "uint16".to_string(),
            PrimitiveType::U32 => "uint32".to_string(),
            PrimitiveType::U64 => "uint64".to_string(),
            PrimitiveType::I8 => "int8".to_string(),
            PrimitiveType::I16 => "int16".to_string(),
            PrimitiveType::I32 => "int32".to_string(),
            PrimitiveType::I64 => "int64".to_string(),
            PrimitiveType::F32 => "float32".to_string(),
            PrimitiveType::F64 => "float64".to_string(),
            PrimitiveType::Usize | PrimitiveType::Isize => "int".to_string(),
        },
        Language::Java => match p {
            PrimitiveType::Bool => "boolean".to_string(),
            PrimitiveType::U8 | PrimitiveType::I8 => "byte".to_string(),
            PrimitiveType::U16 | PrimitiveType::I16 => "short".to_string(),
            PrimitiveType::U32 | PrimitiveType::I32 => "int".to_string(),
            PrimitiveType::U64 | PrimitiveType::I64 | PrimitiveType::Usize | PrimitiveType::Isize => "long".to_string(),
            PrimitiveType::F32 => "float".to_string(),
            PrimitiveType::F64 => "double".to_string(),
        },
        Language::Csharp => match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::U8 => "byte".to_string(),
            PrimitiveType::U16 => "ushort".to_string(),
            PrimitiveType::U32 => "uint".to_string(),
            PrimitiveType::U64 => "ulong".to_string(),
            PrimitiveType::I8 => "sbyte".to_string(),
            PrimitiveType::I16 => "short".to_string(),
            PrimitiveType::I32 => "int".to_string(),
            PrimitiveType::I64 => "long".to_string(),
            PrimitiveType::Usize => "nuint".to_string(),
            PrimitiveType::Isize => "nint".to_string(),
            PrimitiveType::F32 => "float".to_string(),
            PrimitiveType::F64 => "double".to_string(),
        },
        Language::Ruby => match p {
            PrimitiveType::Bool => "Boolean".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "Float".to_string(),
            _ => "Integer".to_string(),
        },
        Language::Php => match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "float".to_string(),
            _ => "int".to_string(),
        },
        Language::Elixir => match p {
            PrimitiveType::Bool => "boolean()".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "float()".to_string(),
            _ => "integer()".to_string(),
        },
        Language::R => match p {
            PrimitiveType::Bool => "logical".to_string(),
            PrimitiveType::F32 | PrimitiveType::F64 => "numeric".to_string(),
            _ => "integer".to_string(),
        },
        Language::Ffi => match p {
            PrimitiveType::Bool => "bool".to_string(),
            PrimitiveType::U8 => "uint8_t".to_string(),
            PrimitiveType::U16 => "uint16_t".to_string(),
            PrimitiveType::U32 => "uint32_t".to_string(),
            PrimitiveType::U64 => "uint64_t".to_string(),
            PrimitiveType::I8 => "int8_t".to_string(),
            PrimitiveType::I16 => "int16_t".to_string(),
            PrimitiveType::I32 => "int32_t".to_string(),
            PrimitiveType::I64 => "int64_t".to_string(),
            PrimitiveType::Usize => "uintptr_t".to_string(),
            PrimitiveType::Isize => "intptr_t".to_string(),
            PrimitiveType::F32 => "float".to_string(),
            PrimitiveType::F64 => "double".to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Naming conventions
// ---------------------------------------------------------------------------

/// Get the display name for a language.
fn lang_display_name(lang: Language) -> &'static str {
    match lang {
        Language::Python => "Python",
        Language::Node => "TypeScript",
        Language::Ruby => "Ruby",
        Language::Php => "PHP",
        Language::Elixir => "Elixir",
        Language::Go => "Go",
        Language::Java => "Java",
        Language::Csharp => "C#",
        Language::Ffi => "C",
        Language::Wasm => "WebAssembly",
        Language::R => "R",
    }
}

/// Get the slug used in file names (e.g. `typescript` for `Node`).
fn lang_slug(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::Node => "typescript",
        Language::Ruby => "ruby",
        Language::Php => "php",
        Language::Elixir => "elixir",
        Language::Go => "go",
        Language::Java => "java",
        Language::Csharp => "csharp",
        Language::Ffi => "c",
        Language::Wasm => "wasm",
        Language::R => "r",
    }
}

/// Get the code fence language identifier.
fn lang_code_fence(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::Node | Language::Wasm => "typescript",
        Language::Ruby => "ruby",
        Language::Php => "php",
        Language::Elixir => "elixir",
        Language::Go => "go",
        Language::Java => "java",
        Language::Csharp => "csharp",
        Language::Ffi => "c",
        Language::R => "r",
    }
}

/// Convert a Rust type name to the idiomatic name for the target language.
fn type_name(name: &str, lang: Language) -> String {
    // Strip module path prefix if present
    let short = name.rsplit("::").next().unwrap_or(name);
    match lang {
        Language::Python
        | Language::Node
        | Language::Wasm
        | Language::Ruby
        | Language::Go
        | Language::Java
        | Language::Csharp
        | Language::Php
        | Language::Elixir
        | Language::R => short.to_pascal_case(),
        Language::Ffi => {
            // C: prefix with HTM and PascalCase
            format!("HTM{}", short.to_pascal_case())
        }
    }
}

/// Convert a Rust function name to the idiomatic name for the target language.
fn func_name(name: &str, lang: Language) -> String {
    match lang {
        Language::Python | Language::Ruby | Language::Elixir | Language::R => name.to_snake_case(),
        Language::Node | Language::Wasm | Language::Java | Language::Php | Language::Csharp => to_camel_case(name),
        Language::Go => name.to_pascal_case(),
        Language::Ffi => format!("htm_{}", name.to_snake_case()),
    }
}

/// Convert a Rust field name to the idiomatic name for the target language.
fn field_name(name: &str, lang: Language) -> String {
    match lang {
        Language::Python | Language::Ruby | Language::Elixir | Language::Go | Language::R | Language::Ffi => {
            name.to_snake_case()
        }
        Language::Node | Language::Wasm | Language::Java | Language::Php | Language::Csharp => to_camel_case(name),
    }
}

/// Convert a Rust enum variant name to the idiomatic name for the target language.
fn enum_variant_name(name: &str, lang: Language) -> String {
    match lang {
        Language::Python => {
            // Python: UPPER_SNAKE_CASE
            name.to_snake_case().to_uppercase()
        }
        Language::Ruby | Language::Elixir => {
            // Ruby/Elixir: :snake_atom style
            name.to_snake_case()
        }
        Language::Go | Language::Node | Language::Wasm | Language::Java | Language::Csharp | Language::Php => {
            name.to_pascal_case()
        }
        Language::R => name.to_snake_case(),
        Language::Ffi => format!("HTM_{}", name.to_snake_case().to_uppercase()),
    }
}

/// Convert snake_case or PascalCase to camelCase.
fn to_camel_case(s: &str) -> String {
    let pascal = s.to_upper_camel_case();
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_lowercase().to_string() + chars.as_str(),
    }
}

// ---------------------------------------------------------------------------
// Default value formatting
// ---------------------------------------------------------------------------

fn format_field_default(field: &FieldDef, lang: Language) -> String {
    if let Some(typed) = &field.typed_default {
        return format_typed_default(typed, lang);
    }
    if let Some(raw) = &field.default {
        if !raw.is_empty() {
            return format!("`{raw}`");
        }
    }
    if field.optional {
        return match lang {
            Language::Python => "`None`".to_string(),
            Language::Node | Language::Wasm => "`null`".to_string(),
            Language::Go => "`nil`".to_string(),
            Language::Java => "`null`".to_string(),
            Language::Csharp => "`null`".to_string(),
            Language::Ruby => "`nil`".to_string(),
            Language::Php => "`null`".to_string(),
            Language::Elixir => "`nil`".to_string(),
            Language::R => "`NULL`".to_string(),
            Language::Ffi => "`NULL`".to_string(),
        };
    }
    String::new()
}

fn format_typed_default(val: &DefaultValue, lang: Language) -> String {
    match val {
        DefaultValue::BoolLiteral(b) => match lang {
            Language::Python => format!("`{}`", if *b { "True" } else { "False" }),
            _ => format!("`{b}`"),
        },
        DefaultValue::StringLiteral(s) => format!("`\"{s}\"`"),
        DefaultValue::IntLiteral(n) => format!("`{n}`"),
        DefaultValue::FloatLiteral(f) => format!("`{f}`"),
        DefaultValue::EnumVariant(v) => {
            // v is something like "HeadingStyle::Atx" or just "Atx"
            let parts: Vec<&str> = v.splitn(2, "::").collect();
            if parts.len() == 2 {
                let enum_type = type_name(parts[0], lang);
                let variant = enum_variant_name(parts[1], lang);
                match lang {
                    Language::Python => format!("`{enum_type}.{variant}`"),
                    Language::Node | Language::Wasm => format!("`{enum_type}.{variant}`"),
                    Language::Go => format!("`{enum_type}{variant}`"),
                    Language::Java => format!("`{enum_type}.{variant}`"),
                    Language::Csharp => format!("`{enum_type}.{variant}`"),
                    Language::Ruby => format!("`:{variant}`"),
                    Language::Php => format!("`{enum_type}::{variant}`"),
                    Language::Elixir => format!("`:{variant}`"),
                    Language::R => format!("`\"{variant}\"`"),
                    Language::Ffi => format!("`HTM_{}`", variant.to_uppercase()),
                }
            } else {
                format!("`{v}`")
            }
        }
        DefaultValue::Empty => match lang {
            Language::Python => "`[]`".to_string(),
            Language::Node | Language::Wasm => "`[]`".to_string(),
            Language::Go => "`nil`".to_string(),
            Language::Java => "`Collections.emptyList()`".to_string(),
            Language::Csharp => "`new List<>()`".to_string(),
            Language::Ruby => "`[]`".to_string(),
            Language::Php => "`[]`".to_string(),
            Language::Elixir => "`[]`".to_string(),
            Language::R => "`list()`".to_string(),
            Language::Ffi => "`NULL`".to_string(),
        },
        DefaultValue::None => match lang {
            Language::Python => "`None`".to_string(),
            Language::Node | Language::Wasm => "`null`".to_string(),
            Language::Go => "`nil`".to_string(),
            Language::Java => "`null`".to_string(),
            Language::Csharp => "`null`".to_string(),
            Language::Ruby => "`nil`".to_string(),
            Language::Php => "`null`".to_string(),
            Language::Elixir => "`nil`".to_string(),
            Language::R => "`NULL`".to_string(),
            Language::Ffi => "`NULL`".to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Doc string utilities
// ---------------------------------------------------------------------------

/// Clean up Rust doc strings for Markdown output.
///
/// Converts Rust-style links like `` [`field`](Self::field) `` to plain `` `field` ``.
/// Strips `# Examples` sections.
fn clean_doc(doc: &str) -> String {
    if doc.is_empty() {
        return String::new();
    }

    // Strip `# Examples` and everything after it
    let doc = strip_examples_section(doc);

    // Convert Rust-style links: [`text`](path) → `text`
    let doc = rust_links_to_plain(&doc);

    doc.trim().to_string()
}

/// Inline version that also strips newlines for use in table cells.
fn clean_doc_inline(doc: &str) -> String {
    if doc.is_empty() {
        return String::new();
    }
    let cleaned = clean_doc(doc);
    // Collapse to single line for table cells
    cleaned
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        // Escape pipe characters in table cells
        .replace('|', "\\|")
}

/// Strip `# Examples` heading and everything after it.
fn strip_examples_section(doc: &str) -> String {
    let mut out = String::new();
    let mut skip = false;
    for line in doc.lines() {
        if line.trim_start_matches('#').trim().eq_ignore_ascii_case("examples") && line.starts_with('#') {
            skip = true;
            continue;
        }
        if skip && line.starts_with('#') {
            // Another section — stop skipping
            skip = false;
        }
        if !skip {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Convert `` [`text`](path) `` patterns to `` `text` ``.
fn rust_links_to_plain(doc: &str) -> String {
    // Pattern: [`text`](anything) → `text`
    let mut result = String::with_capacity(doc.len());
    let chars: Vec<char> = doc.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Look for [`
        if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '`' {
            // Find closing `]`
            let start = i + 1; // position of opening `
            let mut j = start;
            while j < chars.len() && chars[j] != ']' {
                j += 1;
            }
            if j < chars.len() && j + 1 < chars.len() && chars[j] == ']' && chars[j + 1] == '(' {
                // Found `](`; now find closing `)`
                let text: String = chars[start..j].iter().collect();
                let mut k = j + 2;
                while k < chars.len() && chars[k] != ')' {
                    k += 1;
                }
                if k < chars.len() {
                    // Emit just the inner backtick text
                    result.push_str(&text);
                    i = k + 1;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    result
}

// ---------------------------------------------------------------------------
// Ordering helpers
// ---------------------------------------------------------------------------

fn type_sort_key(name: &str) -> (u8, &str) {
    match name {
        "ConversionOptions" => (0, name),
        "ConversionResult" => (1, name),
        _ => (2, name),
    }
}

fn is_update_type(name: &str) -> bool {
    name.ends_with("Update")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alef_core::ir::PrimitiveType;

    #[test]
    fn test_doc_type_string() {
        assert_eq!(doc_type(&TypeRef::String, Language::Python), "str");
        assert_eq!(doc_type(&TypeRef::String, Language::Node), "string");
        assert_eq!(doc_type(&TypeRef::String, Language::Java), "String");
        assert_eq!(doc_type(&TypeRef::String, Language::Ffi), "const char*");
    }

    #[test]
    fn test_doc_type_optional() {
        let ty = TypeRef::Optional(Box::new(TypeRef::String));
        assert_eq!(doc_type(&ty, Language::Python), "str | None");
        assert_eq!(doc_type(&ty, Language::Node), "string | null");
        assert_eq!(doc_type(&ty, Language::Go), "*string");
        assert_eq!(doc_type(&ty, Language::Csharp), "string?");
    }

    #[test]
    fn test_doc_type_vec() {
        let ty = TypeRef::Vec(Box::new(TypeRef::String));
        assert_eq!(doc_type(&ty, Language::Python), "list[str]");
        assert_eq!(doc_type(&ty, Language::Node), "Array<string>");
        assert_eq!(doc_type(&ty, Language::Go), "[]string");
        assert_eq!(doc_type(&ty, Language::Java), "List<String>");
    }

    #[test]
    fn test_doc_type_primitives() {
        assert_eq!(
            doc_type(&TypeRef::Primitive(PrimitiveType::Bool), Language::Python),
            "bool"
        );
        assert_eq!(
            doc_type(&TypeRef::Primitive(PrimitiveType::Bool), Language::Node),
            "boolean"
        );
        assert_eq!(
            doc_type(&TypeRef::Primitive(PrimitiveType::U64), Language::Node),
            "bigint"
        );
        assert_eq!(
            doc_type(&TypeRef::Primitive(PrimitiveType::F64), Language::Python),
            "float"
        );
        assert_eq!(
            doc_type(&TypeRef::Primitive(PrimitiveType::U32), Language::Ffi),
            "uint32_t"
        );
    }

    #[test]
    fn test_enum_variant_name_python() {
        assert_eq!(enum_variant_name("Atx", Language::Python), "ATX");
        assert_eq!(enum_variant_name("SnakeCase", Language::Python), "SNAKE_CASE");
    }

    #[test]
    fn test_enum_variant_name_java() {
        assert_eq!(enum_variant_name("Atx", Language::Java), "Atx");
    }

    #[test]
    fn test_enum_variant_name_ffi() {
        assert_eq!(enum_variant_name("Atx", Language::Ffi), "HTM_ATX");
    }

    #[test]
    fn test_clean_doc_strips_examples() {
        let doc = "Does something.\n\n# Examples\n\n```rust\nfoo();\n```\n";
        let cleaned = clean_doc(doc);
        assert!(!cleaned.contains("Examples"));
        assert!(!cleaned.contains("foo()"));
        assert!(cleaned.contains("Does something"));
    }

    #[test]
    fn test_clean_doc_rust_links() {
        let doc = "See [`field`](Self::field) for details.";
        let cleaned = clean_doc(doc);
        assert_eq!(cleaned, "See `field` for details.");
    }

    #[test]
    fn test_is_update_type() {
        assert!(is_update_type("ConversionOptionsUpdate"));
        assert!(!is_update_type("ConversionOptions"));
    }

    #[test]
    fn test_type_sort_key_ordering() {
        assert!(type_sort_key("ConversionOptions") < type_sort_key("ConversionResult"));
        assert!(type_sort_key("ConversionResult") < type_sort_key("SomeOtherType"));
    }

    #[test]
    fn test_func_name_conventions() {
        assert_eq!(func_name("convert", Language::Python), "convert");
        assert_eq!(func_name("convert_html", Language::Node), "convertHtml");
        assert_eq!(func_name("convert_html", Language::Go), "ConvertHtml");
        assert_eq!(func_name("convert", Language::Ffi), "htm_convert");
    }

    #[test]
    fn test_type_name_ffi_prefix() {
        assert_eq!(type_name("ConversionOptions", Language::Ffi), "HTMConversionOptions");
        assert_eq!(type_name("ConversionResult", Language::Ffi), "HTMConversionResult");
    }

    #[test]
    fn test_generate_docs_empty_api() {
        let api = ApiSurface {
            crate_name: "test".to_string(),
            version: "0.1.0".to_string(),
            types: vec![],
            functions: vec![],
            enums: vec![],
            errors: vec![],
        };
        use alef_core::config::*;
        let config = AlefConfig {
            crate_config: CrateConfig {
                name: "test".to_string(),
                sources: vec![],
                version_from: "Cargo.toml".to_string(),
                core_import: None,
                workspace_root: None,
                skip_core_import: false,
                features: vec![],
                path_mappings: std::collections::HashMap::new(),
            },
            languages: vec![Language::Python],
            exclude: ExcludeConfig::default(),
            include: IncludeConfig::default(),
            output: OutputConfig::default(),
            python: None,
            node: None,
            ruby: None,
            php: None,
            elixir: None,
            wasm: None,
            ffi: None,
            go: None,
            java: None,
            csharp: None,
            r: None,
            scaffold: None,
            readme: None,
            lint: None,
            test: None,
            custom_files: None,
            adapters: vec![],
            custom_modules: CustomModulesConfig::default(),
            custom_registrations: CustomRegistrationsConfig::default(),
            opaque_types: std::collections::HashMap::new(),
            generate: GenerateConfig::default(),
            generate_overrides: std::collections::HashMap::new(),
            dto: Default::default(),
            sync: None,
            e2e: None,
        };

        let files = generate_docs(&api, &config, &[Language::Python], "docs").unwrap();
        // 1 lang + configuration.md + errors.md
        assert_eq!(files.len(), 3);
        let lang_file = files
            .iter()
            .find(|f| f.path.to_str().unwrap().contains("api-python"))
            .unwrap();
        assert!(lang_file.content.contains("Python API Reference"));
        assert!(lang_file.content.contains("v0.1.0"));
    }
}
