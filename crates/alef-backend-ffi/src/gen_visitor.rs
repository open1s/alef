/// Generate visitor/callback FFI bindings.
///
/// This module produces the `#[repr(C)]` callback struct, an opaque `Visitor`
/// handle that bridges C function pointers into the Rust `HtmlVisitor` trait,
/// and the three public FFI entry points:
///
/// - `{prefix}_visitor_create(callbacks: *const {Prefix}VisitorCallbacks) -> *mut {Prefix}Visitor`
/// - `{prefix}_visitor_free(visitor: *mut {Prefix}Visitor)`
/// - `{prefix}_convert_with_visitor(html, options, visitor) -> *mut ConversionResult`
///
/// # PoC scope
///
/// This initial implementation covers two callbacks (`visit_text` and
/// `visit_element_start`) as a proof-of-concept. The full 42-method suite
/// follows the same pattern but requires extending `VisitorCallbacks` and the
/// `FfiVisitor` trait implementation accordingly.
use heck::ToPascalCase;

/// The integer codes that map to `VisitResult` variants crossing the FFI boundary.
///
/// | Value | Meaning               |
/// |-------|-----------------------|
/// |   0   | `VisitResult::Continue`     |
/// |   1   | `VisitResult::Skip`         |
/// |   2   | `VisitResult::PreserveHtml` |
/// |   3   | `VisitResult::Custom(…)`    |
/// |   4   | `VisitResult::Error(…)`     |
#[allow(dead_code)]
pub const VISIT_RESULT_CONTINUE: i32 = 0;
pub const VISIT_RESULT_SKIP: i32 = 1;
pub const VISIT_RESULT_PRESERVE_HTML: i32 = 2;
pub const VISIT_RESULT_CUSTOM: i32 = 3;
pub const VISIT_RESULT_ERROR: i32 = 4;

/// Generate the visitor FFI bindings block for `lib.rs`.
///
/// # Parameters
///
/// - `prefix`: the FFI function prefix (e.g. `"htm"`).
/// - `core_import`: the Rust `use` path for the core crate (e.g. `"html_to_markdown_rs"`).
pub fn gen_visitor_bindings(prefix: &str, core_import: &str) -> String {
    let pascal_prefix = prefix.to_pascal_case();

    format!(
        r#"// ---------------------------------------------------------------------------
// Visitor / callback FFI  (PoC — visit_text + visit_element_start)
// ---------------------------------------------------------------------------

/// Visit-result code: continue with default conversion.
pub const HTM_VISIT_CONTINUE: i32 = 0;
/// Visit-result code: skip this element entirely (no output).
pub const HTM_VISIT_SKIP: i32 = 1;
/// Visit-result code: preserve the original HTML verbatim.
pub const HTM_VISIT_PRESERVE_HTML: i32 = 2;
/// Visit-result code: use `out_custom` / `out_len` as custom Markdown output.
pub const HTM_VISIT_CUSTOM: i32 = 3;
/// Visit-result code: abort conversion; `out_custom` contains the error message.
pub const HTM_VISIT_ERROR: i32 = 4;

/// Opaque context passed to every C callback.
///
/// Fields reflect `NodeContext` from the Rust core. All string pointers are
/// valid only for the duration of the callback invocation.
#[repr(C)]
pub struct {pascal_prefix}NodeContext {{
    /// Coarse-grained node type tag (matches `NodeType` discriminant).
    pub node_type: i32,
    /// Null-terminated tag name (e.g. `"div"`). Never null.
    pub tag_name: *const std::ffi::c_char,
    /// Depth in the DOM tree (0 = root).
    pub depth: usize,
    /// Index among siblings (0-based).
    pub index_in_parent: usize,
    /// Null-terminated parent tag name, or null if root.
    pub parent_tag: *const std::ffi::c_char,
    /// Non-zero if this element is treated as inline.
    pub is_inline: i32,
}}

/// C-facing callback struct for the visitor pattern.
///
/// Populate the function-pointer fields you care about; leave the rest null.
/// The `user_data` pointer is forwarded unchanged to every callback — use it
/// to thread your own context through the conversion.
///
/// # Callback return protocol
///
/// Callbacks return an `i32` visit-result code.  When the code is
/// `HTM_VISIT_CUSTOM` (3) or `HTM_VISIT_ERROR` (4), the callback must also
/// write a heap-allocated, null-terminated string into `*out_custom` and set
/// `*out_len` to its byte length (excluding the null terminator).  The Rust
/// side will read the string and then call `free()` on the pointer.
///
/// For all other codes `out_custom` and `out_len` are not written.
#[repr(C)]
pub struct {pascal_prefix}VisitorCallbacks {{
    /// Arbitrary caller context forwarded to every callback.
    pub user_data: *mut std::ffi::c_void,

    /// Called before entering any element.
    ///
    /// Signature: `fn(ctx, user_data, out_custom, out_len) -> i32`
    pub visit_element_start: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit text nodes.
    ///
    /// Signature: `fn(ctx, text, user_data, out_custom, out_len) -> i32`
    pub visit_text: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            text: *const std::ffi::c_char,
            user_data: *mut std::ffi::c_void,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,
}}

// SAFETY: The `user_data` pointer is the caller's responsibility. We require
// callers to uphold thread-safety themselves (i.e. not share a visitor across
// threads without synchronisation). The callbacks themselves are `extern "C"`
// and therefore inherently `Send`.
unsafe impl Send for {pascal_prefix}VisitorCallbacks {{}}

/// Opaque handle wrapping a `{pascal_prefix}VisitorCallbacks` and implementing
/// the Rust `HtmlVisitor` trait.
///
/// Allocate with `{prefix}_visitor_create` and release with `{prefix}_visitor_free`.
/// The handle must NOT outlive the `{pascal_prefix}VisitorCallbacks` it was created from.
pub struct {pascal_prefix}Visitor {{
    callbacks: {pascal_prefix}VisitorCallbacks,
    /// CString storage for tag names / parent tags that we pass back to C.
    _tag_scratch: std::cell::RefCell<Vec<std::ffi::CString>>,
}}

impl std::fmt::Debug for {pascal_prefix}Visitor {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        f.debug_struct("{pascal_prefix}Visitor").finish_non_exhaustive()
    }}
}}

/// Map a `VisitResult` integer code + optional custom string pointer back to
/// the Rust `VisitResult` enum.
///
/// # Safety
///
/// `custom_ptr` must be either null or a pointer to a heap-allocated
/// null-terminated string that this function will take ownership of (freeing
/// it after reading).
unsafe fn decode_visit_result(
    code: i32,
    custom_ptr: *mut std::ffi::c_char,
) -> {core_import}::visitor::VisitResult {{
    use {core_import}::visitor::VisitResult;
    match code {{
        {VISIT_RESULT_SKIP} => VisitResult::Skip,
        {VISIT_RESULT_PRESERVE_HTML} => VisitResult::PreserveHtml,
        {VISIT_RESULT_CUSTOM} | {VISIT_RESULT_ERROR} => {{
            let msg = if custom_ptr.is_null() {{
                String::new()
            }} else {{
                // SAFETY: caller guarantees this is a valid heap CString.
                let cstr = unsafe {{ std::ffi::CString::from_raw(custom_ptr) }};
                cstr.to_string_lossy().into_owned()
            }};
            if code == {VISIT_RESULT_CUSTOM} {{
                VisitResult::Custom(msg)
            }} else {{
                VisitResult::Error(msg)
            }}
        }}
        _ => VisitResult::Continue,
    }}
}}

/// Build a temporary `{pascal_prefix}NodeContext` from a Rust `NodeContext`, invoke
/// the provided callback, and decode the `VisitResult`.
///
/// The `NodeContext` passed to the C callback is only valid for the duration
/// of this function call.
unsafe fn call_with_ctx<F>(
    ctx: &{core_import}::visitor::NodeContext,
    callback: F,
) -> {core_import}::visitor::VisitResult
where
    F: FnOnce(
        *const {pascal_prefix}NodeContext,
        *mut *mut std::ffi::c_char,
        *mut usize,
    ) -> i32,
{{
    // Build temporary CStrings for the string fields.
    let tag_cstring = std::ffi::CString::new(ctx.tag_name.as_str()).unwrap_or_default();
    let parent_cstring: Option<std::ffi::CString> = ctx
        .parent_tag
        .as_deref()
        .and_then(|s| std::ffi::CString::new(s).ok());

    let c_ctx = {pascal_prefix}NodeContext {{
        node_type: ctx.node_type as i32,
        tag_name: tag_cstring.as_ptr(),
        depth: ctx.depth,
        index_in_parent: ctx.index_in_parent,
        parent_tag: parent_cstring.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()),
        is_inline: ctx.is_inline as i32,
    }};

    let mut out_custom: *mut std::ffi::c_char = std::ptr::null_mut();
    let mut out_len: usize = 0;

    let code = callback(&c_ctx, &mut out_custom, &mut out_len);

    // SAFETY: decode_visit_result takes ownership of out_custom when non-null.
    unsafe {{ decode_visit_result(code, out_custom) }}
}}

impl {core_import}::visitor::HtmlVisitor for {pascal_prefix}Visitor {{
    fn visit_element_start(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_element_start else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        // SAFETY: cb is a valid function pointer; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, out_custom, out_len)
            }})
        }}
    }}

    fn visit_text(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_text else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cstring = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is a valid function pointer; text_cstring lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, text_cstring.as_ptr(), user_data, out_custom, out_len)
            }})
        }}
    }}
}}

/// Create a new visitor handle from a callbacks struct.
///
/// The returned handle must be freed with `{prefix}_visitor_free`.
/// The `{pascal_prefix}VisitorCallbacks` struct is **copied** into the handle;
/// the caller may free it after this call returns.
///
/// Returns null on allocation failure.
///
/// # Safety
///
/// `callbacks` must point to a valid, fully initialised `{pascal_prefix}VisitorCallbacks`.
/// `user_data` (embedded in the struct) must remain valid and accessible from
/// any thread that calls `{prefix}_convert_with_visitor` until after
/// `{prefix}_visitor_free` is called.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn {prefix}_visitor_create(
    callbacks: *const {pascal_prefix}VisitorCallbacks,
) -> *mut {pascal_prefix}Visitor {{
    if callbacks.is_null() {{
        return std::ptr::null_mut();
    }}
    // SAFETY: caller guarantees the pointer is valid.
    let cbs = unsafe {{ callbacks.read() }};
    let visitor = {pascal_prefix}Visitor {{
        callbacks: cbs,
        _tag_scratch: std::cell::RefCell::new(Vec::new()),
    }};
    Box::into_raw(Box::new(visitor))
}}

/// Free a visitor handle previously returned by `{prefix}_visitor_create`.
///
/// After this call the pointer is invalid and must not be used.
///
/// # Safety
///
/// `visitor` must have been returned by `{prefix}_visitor_create`, or be null.
/// Passing a null pointer is safe and has no effect.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn {prefix}_visitor_free(visitor: *mut {pascal_prefix}Visitor) {{
    if !visitor.is_null() {{
        // SAFETY: visitor was created with Box::into_raw.
        unsafe {{ drop(Box::from_raw(visitor)); }}
    }}
}}

/// Convert HTML to Markdown using a custom visitor.
///
/// Equivalent to `{prefix}_convert` but threads the provided visitor through
/// the conversion pipeline so that every `visit_*` callback is invoked during
/// processing.
///
/// Returns a heap-allocated null-terminated Markdown string on success, or
/// null on failure (check `{prefix}_last_error_code` / `{prefix}_last_error_context`).
/// The returned pointer must be freed with `{prefix}_free_string`.
///
/// # Arguments
///
/// - `html`: null-terminated, UTF-8 HTML input. Must not be null.
/// - `options`: optional conversion options; pass null for defaults.
/// - `visitor`: optional visitor handle from `{prefix}_visitor_create`; pass
///   null for default conversion (equivalent to `{prefix}_convert`).
///
/// # Safety
///
/// All pointer arguments must be valid or null as described above.
/// The `visitor` pointer (and its embedded `user_data`) must remain valid for
/// the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn {prefix}_convert_with_visitor(
    html: *const std::ffi::c_char,
    options: *const {core_import}::ConversionOptions,
    visitor: *mut {pascal_prefix}Visitor,
) -> *mut std::ffi::c_char {{
    clear_last_error();

    if html.is_null() {{
        set_last_error(1, "Null pointer passed for html");
        return std::ptr::null_mut();
    }}

    let html_str = match unsafe {{ std::ffi::CStr::from_ptr(html) }}.to_str() {{
        Ok(s) => s.to_string(),
        Err(_) => {{
            set_last_error(1, "Invalid UTF-8 in html parameter");
            return std::ptr::null_mut();
        }}
    }};

    let options_rs: Option<{core_import}::ConversionOptions> = if options.is_null() {{
        None
    }} else {{
        Some(unsafe {{ &*(options as *const {core_import}::ConversionOptions) }}.clone())
    }};

    // Build the visitor handle if provided.
    let visitor_handle: Option<{core_import}::visitor::VisitorHandle> = if visitor.is_null() {{
        None
    }} else {{
        // SAFETY: visitor is a valid pointer for the duration of this call.
        let ffi_visitor = unsafe {{ &mut *visitor }};
        // Wrap in Rc<RefCell<dyn HtmlVisitor>> as required by convert_with_visitor.
        // We use a raw-pointer wrapper to avoid cloning — the {pascal_prefix}Visitor is
        // pinned in place by the caller-owned Box.
        struct VisitorRef(*mut {pascal_prefix}Visitor);
        impl std::fmt::Debug for VisitorRef {{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
                f.debug_struct("VisitorRef").finish_non_exhaustive()
            }}
        }}
        impl {core_import}::visitor::HtmlVisitor for VisitorRef {{
            fn visit_element_start(
                &mut self,
                ctx: &{core_import}::visitor::NodeContext,
            ) -> {core_import}::visitor::VisitResult {{
                // SAFETY: pointer is valid for the duration of the convert call.
                unsafe {{ (*self.0).visit_element_start(ctx) }}
            }}
            fn visit_text(
                &mut self,
                ctx: &{core_import}::visitor::NodeContext,
                text: &str,
            ) -> {core_import}::visitor::VisitResult {{
                // SAFETY: pointer is valid for the duration of the convert call.
                unsafe {{ (*self.0).visit_text(ctx, text) }}
            }}
        }}
        let _ = ffi_visitor; // suppress unused warning
        Some(std::rc::Rc::new(std::cell::RefCell::new(VisitorRef(visitor))))
    }};

    match {core_import}::convert_with_visitor(&html_str, options_rs, visitor_handle) {{
        Ok(markdown) => match std::ffi::CString::new(markdown) {{
            Ok(s) => s.into_raw(),
            Err(_) => {{
                set_last_error(3, "Conversion output contained null bytes");
                std::ptr::null_mut()
            }}
        }},
        Err(e) => {{
            set_last_error(2, &e.to_string());
            std::ptr::null_mut()
        }}
    }}
}}"#,
        VISIT_RESULT_SKIP = VISIT_RESULT_SKIP,
        VISIT_RESULT_PRESERVE_HTML = VISIT_RESULT_PRESERVE_HTML,
        VISIT_RESULT_CUSTOM = VISIT_RESULT_CUSTOM,
        VISIT_RESULT_ERROR = VISIT_RESULT_ERROR,
        prefix = prefix,
        pascal_prefix = pascal_prefix,
        core_import = core_import,
    )
}
