use skif_core::ir::{FieldDef, MethodDef, ParamDef, TypeRef};

/// Check if a free function can be auto-delegated to the core crate.
/// Functions with sanitized params/return types cannot be delegated — the generated
/// signature differs from the core crate's actual signature.
pub fn can_auto_delegate_function(func: &skif_core::ir::FunctionDef) -> bool {
    !func.sanitized
        && func.params.iter().all(|p| !p.sanitized && is_delegatable_type(&p.ty))
        && is_delegatable_type(&func.return_type)
}

/// Check if all params and return type are delegatable.
/// Delegatable = any type that can be passed through with .into() or direct copy.
/// Non-delegatable = Json (no standard conversion), sanitized types.
/// Methods with sanitized signatures cannot be delegated.
pub fn can_auto_delegate(method: &MethodDef) -> bool {
    !method.sanitized
        && method.params.iter().all(|p| !p.sanitized && is_delegatable_type(&p.ty))
        && is_delegatable_type(&method.return_type)
}

/// A type is delegatable if it can cross the binding boundary.
/// Named types use .into() conversion. Json types are not delegatable.
pub fn is_delegatable_type(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::Primitive(_) | TypeRef::String | TypeRef::Bytes | TypeRef::Path | TypeRef::Unit => true,
        TypeRef::Named(_) => true, // Uses .into() conversion
        TypeRef::Optional(inner) | TypeRef::Vec(inner) => is_delegatable_type(inner),
        TypeRef::Map(k, v) => is_delegatable_type(k) && is_delegatable_type(v),
        TypeRef::Json => false,
    }
}

/// Check if a type is "simple" — can be passed without any conversion.
pub fn is_simple_type(ty: &TypeRef) -> bool {
    match ty {
        TypeRef::Primitive(_) | TypeRef::String | TypeRef::Bytes | TypeRef::Path | TypeRef::Unit => true,
        TypeRef::Optional(inner) | TypeRef::Vec(inner) => is_simple_type(inner),
        TypeRef::Map(k, v) => is_simple_type(k) && is_simple_type(v),
        TypeRef::Named(_) | TypeRef::Json => false,
    }
}

/// Partition methods into (instance, static).
pub fn partition_methods(methods: &[MethodDef]) -> (Vec<&MethodDef>, Vec<&MethodDef>) {
    let instance: Vec<_> = methods.iter().filter(|m| m.receiver.is_some()).collect();
    let statics: Vec<_> = methods.iter().filter(|m| m.receiver.is_none()).collect();
    (instance, statics)
}

/// Build a constructor parameter list string.
/// Returns (param_list, signature_with_defaults, field_assignments).
/// If param_list exceeds 100 chars, uses multiline format with trailing commas.
pub fn constructor_parts(fields: &[FieldDef], type_mapper: &dyn Fn(&TypeRef) -> String) -> (String, String, String) {
    // Sort fields: required first, then optional.
    // Many FFI frameworks (PyO3, NAPI) require required params before optional ones.
    let mut sorted_fields: Vec<&FieldDef> = fields.iter().collect();
    sorted_fields.sort_by_key(|f| f.optional as u8);

    let params: Vec<String> = sorted_fields
        .iter()
        .map(|f| {
            let ty = if f.optional {
                format!("Option<{}>", type_mapper(&f.ty))
            } else {
                type_mapper(&f.ty)
            };
            format!("{}: {}", f.name, ty)
        })
        .collect();

    let defaults: Vec<String> = sorted_fields
        .iter()
        .map(|f| {
            if f.optional {
                format!("{}=None", f.name)
            } else {
                f.name.clone()
            }
        })
        .collect();

    // Assignments keep original field order (for struct literal)
    let assignments: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();

    // Format param_list with line wrapping if needed
    let single_line = params.join(", ");
    let param_list = if single_line.len() > 100 {
        format!("\n        {},\n    ", params.join(",\n        "))
    } else {
        single_line
    };

    (param_list, defaults.join(", "), assignments.join(", "))
}

/// Build a function parameter list.
pub fn function_params(params: &[ParamDef], type_mapper: &dyn Fn(&TypeRef) -> String) -> String {
    params
        .iter()
        .map(|p| {
            let ty = if p.optional {
                format!("Option<{}>", type_mapper(&p.ty))
            } else {
                type_mapper(&p.ty)
            };
            format!("{}: {}", p.name, ty)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Build a function signature defaults string (for pyo3 signature etc.).
pub fn function_sig_defaults(params: &[ParamDef]) -> String {
    params
        .iter()
        .map(|p| {
            if p.optional {
                format!("{}=None", p.name)
            } else {
                p.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}
