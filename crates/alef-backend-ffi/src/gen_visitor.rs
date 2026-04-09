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
/// # Coverage
///
/// All 42 `HtmlVisitor` trait methods are covered. The callback struct field
/// order matches the Go binding's expected layout exactly (see
/// `packages/go/v3/htmltomarkdown/visitor.go`).
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
// Visitor / callback FFI — all 42 HtmlVisitor methods
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
/// # Field order
///
/// The field order matches the Go binding's expected C layout exactly.
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
///
/// # Callback signatures
///
/// All callbacks share the same leading parameters:
/// ```c
/// fn(ctx, user_data, out_custom, out_len, ...) -> i32
/// ```
/// followed by method-specific parameters documented on each field.
#[repr(C)]
pub struct {pascal_prefix}VisitorCallbacks {{
    /// Arbitrary caller context forwarded to every callback.
    pub user_data: *mut std::ffi::c_void,

    /// Visit text nodes.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_text: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

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

    /// Called after exiting any element; receives the default markdown output.
    ///
    /// Signature: `fn(ctx, user_data, output, out_custom, out_len) -> i32`
    pub visit_element_end: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            output: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit anchor links `<a href="...">`.
    ///
    /// Signature: `fn(ctx, user_data, href, text, title, out_custom, out_len) -> i32`
    /// `title` may be null.
    pub visit_link: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            href: *const std::ffi::c_char,
            text: *const std::ffi::c_char,
            title: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit images `<img src="...">`.
    ///
    /// Signature: `fn(ctx, user_data, src, alt, title, out_custom, out_len) -> i32`
    /// `title` may be null.
    pub visit_image: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            src: *const std::ffi::c_char,
            alt: *const std::ffi::c_char,
            title: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit heading elements `<h1>`–`<h6>`.
    ///
    /// Signature: `fn(ctx, user_data, level, text, id, out_custom, out_len) -> i32`
    /// `id` may be null.
    pub visit_heading: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            level: u32,
            text: *const std::ffi::c_char,
            id: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit code blocks `<pre><code>`.
    ///
    /// Signature: `fn(ctx, user_data, lang, code, out_custom, out_len) -> i32`
    /// `lang` may be null.
    pub visit_code_block: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            lang: *const std::ffi::c_char,
            code: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit inline code `<code>`.
    ///
    /// Signature: `fn(ctx, user_data, code, out_custom, out_len) -> i32`
    pub visit_code_inline: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            code: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit list items `<li>`.
    ///
    /// Signature: `fn(ctx, user_data, ordered, marker, text, out_custom, out_len) -> i32`
    pub visit_list_item: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            ordered: i32,
            marker: *const std::ffi::c_char,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Called before processing a list `<ul>` or `<ol>`.
    ///
    /// Signature: `fn(ctx, user_data, ordered, out_custom, out_len) -> i32`
    pub visit_list_start: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            ordered: i32,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Called after processing a list `</ul>` or `</ol>`.
    ///
    /// Signature: `fn(ctx, user_data, ordered, output, out_custom, out_len) -> i32`
    pub visit_list_end: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            ordered: i32,
            output: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Called before processing a table `<table>`.
    ///
    /// Signature: `fn(ctx, user_data, out_custom, out_len) -> i32`
    pub visit_table_start: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit table rows `<tr>`.
    ///
    /// Cells are passed as a null-terminated array of null-terminated strings.
    ///
    /// Signature: `fn(ctx, user_data, cells, cell_count, is_header, out_custom, out_len) -> i32`
    pub visit_table_row: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            cells: *const *const std::ffi::c_char,
            cell_count: usize,
            is_header: i32,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Called after processing a table `</table>`.
    ///
    /// Signature: `fn(ctx, user_data, output, out_custom, out_len) -> i32`
    pub visit_table_end: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            output: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit blockquote elements `<blockquote>`.
    ///
    /// Signature: `fn(ctx, user_data, content, depth, out_custom, out_len) -> i32`
    pub visit_blockquote: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            content: *const std::ffi::c_char,
            depth: usize,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit strong/bold elements `<strong>`, `<b>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_strong: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit emphasis/italic elements `<em>`, `<i>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_emphasis: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit strikethrough elements `<s>`, `<del>`, `<strike>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_strikethrough: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit underline elements `<u>`, `<ins>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_underline: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit subscript elements `<sub>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_subscript: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit superscript elements `<sup>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_superscript: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit mark/highlight elements `<mark>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_mark: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit line break elements `<br>`.
    ///
    /// Signature: `fn(ctx, user_data, out_custom, out_len) -> i32`
    pub visit_line_break: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit horizontal rule elements `<hr>`.
    ///
    /// Signature: `fn(ctx, user_data, out_custom, out_len) -> i32`
    pub visit_horizontal_rule: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit custom/unknown elements.
    ///
    /// Signature: `fn(ctx, user_data, tag_name, html, out_custom, out_len) -> i32`
    pub visit_custom_element: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            tag_name: *const std::ffi::c_char,
            html: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit definition list `<dl>`.
    ///
    /// Signature: `fn(ctx, user_data, out_custom, out_len) -> i32`
    pub visit_definition_list_start: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit definition term `<dt>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_definition_term: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit definition description `<dd>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_definition_description: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Called after processing a definition list `</dl>`.
    ///
    /// Signature: `fn(ctx, user_data, output, out_custom, out_len) -> i32`
    pub visit_definition_list_end: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            output: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit form elements `<form>`.
    ///
    /// Signature: `fn(ctx, user_data, action, method, out_custom, out_len) -> i32`
    /// `action` and `method` may be null.
    pub visit_form: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            action: *const std::ffi::c_char,
            method: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit input elements `<input>`.
    ///
    /// Signature: `fn(ctx, user_data, input_type, name, value, out_custom, out_len) -> i32`
    /// `name` and `value` may be null.
    pub visit_input: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            input_type: *const std::ffi::c_char,
            name: *const std::ffi::c_char,
            value: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit button elements `<button>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_button: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit audio elements `<audio>`.
    ///
    /// Signature: `fn(ctx, user_data, src, out_custom, out_len) -> i32`
    /// `src` may be null.
    pub visit_audio: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            src: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit video elements `<video>`.
    ///
    /// Signature: `fn(ctx, user_data, src, out_custom, out_len) -> i32`
    /// `src` may be null.
    pub visit_video: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            src: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit iframe elements `<iframe>`.
    ///
    /// Signature: `fn(ctx, user_data, src, out_custom, out_len) -> i32`
    /// `src` may be null.
    pub visit_iframe: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            src: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit details elements `<details>`.
    ///
    /// Signature: `fn(ctx, user_data, open, out_custom, out_len) -> i32`
    /// `open` is non-zero when the `open` attribute is present.
    pub visit_details: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            open: i32,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit summary elements `<summary>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_summary: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Called before processing a figure `<figure>`.
    ///
    /// Signature: `fn(ctx, user_data, out_custom, out_len) -> i32`
    pub visit_figure_start: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Visit figcaption elements `<figcaption>`.
    ///
    /// Signature: `fn(ctx, user_data, text, out_custom, out_len) -> i32`
    pub visit_figcaption: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            text: *const std::ffi::c_char,
            out_custom: *mut *mut std::ffi::c_char,
            out_len: *mut usize,
        ) -> i32,
    >,

    /// Called after processing a figure `</figure>`.
    ///
    /// Signature: `fn(ctx, user_data, output, out_custom, out_len) -> i32`
    pub visit_figure_end: Option<
        unsafe extern "C" fn(
            ctx: *const {pascal_prefix}NodeContext,
            user_data: *mut std::ffi::c_void,
            output: *const std::ffi::c_char,
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

/// Convert an `Option<&str>` to a C pointer: non-null CString when `Some`, null when `None`.
///
/// Returns `(ptr, Option<CString>)` — the `Option<CString>` must be kept alive
/// until after the pointer is consumed by the callback.
fn opt_str_to_c(s: Option<&str>) -> (*const std::ffi::c_char, Option<std::ffi::CString>) {{
    match s {{
        Some(val) => match std::ffi::CString::new(val) {{
            Ok(cs) => {{
                let ptr = cs.as_ptr();
                (ptr, Some(cs))
            }}
            Err(_) => (std::ptr::null(), None),
        }},
        None => (std::ptr::null(), None),
    }}
}}

impl {core_import}::visitor::HtmlVisitor for {pascal_prefix}Visitor {{
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
                cb(c_ctx, user_data, text_cstring.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

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

    fn visit_element_end(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        output: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_element_end else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let output_cstring = match std::ffi::CString::new(output) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is a valid function pointer; output_cstring lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, output_cstring.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_link(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        href: &str,
        text: &str,
        title: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_link else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let href_cs = match std::ffi::CString::new(href) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let (title_ptr, _title_cs) = opt_str_to_c(title);
        // SAFETY: cb is valid; all CStrings live for the duration of this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, href_cs.as_ptr(), text_cs.as_ptr(), title_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_image(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        src: &str,
        alt: &str,
        title: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_image else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let src_cs = match std::ffi::CString::new(src) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let alt_cs = match std::ffi::CString::new(alt) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let (title_ptr, _title_cs) = opt_str_to_c(title);
        // SAFETY: cb is valid; all CStrings live for the duration of this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, src_cs.as_ptr(), alt_cs.as_ptr(), title_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_heading(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        level: u32,
        text: &str,
        id: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_heading else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let (id_ptr, _id_cs) = opt_str_to_c(id);
        // SAFETY: cb is valid; all CStrings live for the duration of this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, level, text_cs.as_ptr(), id_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_code_block(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        lang: Option<&str>,
        code: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_code_block else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let (lang_ptr, _lang_cs) = opt_str_to_c(lang);
        let code_cs = match std::ffi::CString::new(code) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; all CStrings live for the duration of this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, lang_ptr, code_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_code_inline(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        code: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_code_inline else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let code_cs = match std::ffi::CString::new(code) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; code_cs lives for the duration of this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, code_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_list_item(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        ordered: bool,
        marker: &str,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_list_item else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let marker_cs = match std::ffi::CString::new(marker) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let ordered_i = i32::from(ordered);
        // SAFETY: cb is valid; all CStrings live for the duration of this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, ordered_i, marker_cs.as_ptr(), text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_list_start(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        ordered: bool,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_list_start else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let ordered_i = i32::from(ordered);
        // SAFETY: cb is valid; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, ordered_i, out_custom, out_len)
            }})
        }}
    }}

    fn visit_list_end(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        ordered: bool,
        output: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_list_end else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let output_cs = match std::ffi::CString::new(output) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let ordered_i = i32::from(ordered);
        // SAFETY: cb is valid; output_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, ordered_i, output_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_table_start(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_table_start else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        // SAFETY: cb is valid; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, out_custom, out_len)
            }})
        }}
    }}

    fn visit_table_row(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        cells: &[String],
        is_header: bool,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_table_row else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        // Build a temporary array of CString pointers for the cells.
        let cell_cstrings: Vec<std::ffi::CString> = cells
            .iter()
            .filter_map(|s| std::ffi::CString::new(s.as_str()).ok())
            .collect();
        let cell_ptrs: Vec<*const std::ffi::c_char> =
            cell_cstrings.iter().map(|cs| cs.as_ptr()).collect();
        let cell_count = cell_ptrs.len();
        let is_header_i = i32::from(is_header);
        // SAFETY: cb is valid; cell_cstrings and cell_ptrs live for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, cell_ptrs.as_ptr(), cell_count, is_header_i, out_custom, out_len)
            }})
        }}
    }}

    fn visit_table_end(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        output: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_table_end else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let output_cs = match std::ffi::CString::new(output) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; output_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, output_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_blockquote(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        content: &str,
        depth: usize,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_blockquote else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let content_cs = match std::ffi::CString::new(content) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; content_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, content_cs.as_ptr(), depth, out_custom, out_len)
            }})
        }}
    }}

    fn visit_strong(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_strong else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_emphasis(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_emphasis else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_strikethrough(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_strikethrough else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_underline(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_underline else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_subscript(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_subscript else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_superscript(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_superscript else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_mark(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_mark else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_line_break(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_line_break else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        // SAFETY: cb is valid; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, out_custom, out_len)
            }})
        }}
    }}

    fn visit_horizontal_rule(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_horizontal_rule else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        // SAFETY: cb is valid; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, out_custom, out_len)
            }})
        }}
    }}

    fn visit_custom_element(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        tag_name: &str,
        html: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_custom_element else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let tag_cs = match std::ffi::CString::new(tag_name) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let html_cs = match std::ffi::CString::new(html) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; all CStrings live for the duration of this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, tag_cs.as_ptr(), html_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_definition_list_start(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_definition_list_start else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        // SAFETY: cb is valid; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, out_custom, out_len)
            }})
        }}
    }}

    fn visit_definition_term(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_definition_term else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_definition_description(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_definition_description else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_definition_list_end(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        output: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_definition_list_end else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let output_cs = match std::ffi::CString::new(output) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; output_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, output_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_form(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        action: Option<&str>,
        method: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_form else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let (action_ptr, _action_cs) = opt_str_to_c(action);
        let (method_ptr, _method_cs) = opt_str_to_c(method);
        // SAFETY: cb is valid; all CStrings live for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, action_ptr, method_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_input(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        input_type: &str,
        name: Option<&str>,
        value: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_input else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let type_cs = match std::ffi::CString::new(input_type) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        let (name_ptr, _name_cs) = opt_str_to_c(name);
        let (value_ptr, _value_cs) = opt_str_to_c(value);
        // SAFETY: cb is valid; all CStrings live for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, type_cs.as_ptr(), name_ptr, value_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_button(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_button else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_audio(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        src: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_audio else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let (src_ptr, _src_cs) = opt_str_to_c(src);
        // SAFETY: cb is valid; _src_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, src_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_video(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        src: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_video else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let (src_ptr, _src_cs) = opt_str_to_c(src);
        // SAFETY: cb is valid; _src_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, src_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_iframe(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        src: Option<&str>,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_iframe else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let (src_ptr, _src_cs) = opt_str_to_c(src);
        // SAFETY: cb is valid; _src_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, src_ptr, out_custom, out_len)
            }})
        }}
    }}

    fn visit_details(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        open: bool,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_details else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let open_i = i32::from(open);
        // SAFETY: cb is valid; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, open_i, out_custom, out_len)
            }})
        }}
    }}

    fn visit_summary(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_summary else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_figure_start(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_figure_start else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        // SAFETY: cb is valid; ctx lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, out_custom, out_len)
            }})
        }}
    }}

    fn visit_figcaption(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        text: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_figcaption else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let text_cs = match std::ffi::CString::new(text) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; text_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, text_cs.as_ptr(), out_custom, out_len)
            }})
        }}
    }}

    fn visit_figure_end(
        &mut self,
        ctx: &{core_import}::visitor::NodeContext,
        output: &str,
    ) -> {core_import}::visitor::VisitResult {{
        let Some(cb) = self.callbacks.visit_figure_end else {{
            return {core_import}::visitor::VisitResult::Continue;
        }};
        let user_data = self.callbacks.user_data;
        let output_cs = match std::ffi::CString::new(output) {{
            Ok(s) => s,
            Err(_) => return {core_import}::visitor::VisitResult::Continue,
        }};
        // SAFETY: cb is valid; output_cs lives for this call.
        unsafe {{
            call_with_ctx(ctx, |c_ctx, out_custom, out_len| {{
                cb(c_ctx, user_data, output_cs.as_ptr(), out_custom, out_len)
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
            fn visit_text(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_text(ctx, text) }}
            }}
            fn visit_element_start(&mut self, ctx: &{core_import}::visitor::NodeContext) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_element_start(ctx) }}
            }}
            fn visit_element_end(&mut self, ctx: &{core_import}::visitor::NodeContext, output: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_element_end(ctx, output) }}
            }}
            fn visit_link(&mut self, ctx: &{core_import}::visitor::NodeContext, href: &str, text: &str, title: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_link(ctx, href, text, title) }}
            }}
            fn visit_image(&mut self, ctx: &{core_import}::visitor::NodeContext, src: &str, alt: &str, title: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_image(ctx, src, alt, title) }}
            }}
            fn visit_heading(&mut self, ctx: &{core_import}::visitor::NodeContext, level: u32, text: &str, id: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_heading(ctx, level, text, id) }}
            }}
            fn visit_code_block(&mut self, ctx: &{core_import}::visitor::NodeContext, lang: Option<&str>, code: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_code_block(ctx, lang, code) }}
            }}
            fn visit_code_inline(&mut self, ctx: &{core_import}::visitor::NodeContext, code: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_code_inline(ctx, code) }}
            }}
            fn visit_list_item(&mut self, ctx: &{core_import}::visitor::NodeContext, ordered: bool, marker: &str, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_list_item(ctx, ordered, marker, text) }}
            }}
            fn visit_list_start(&mut self, ctx: &{core_import}::visitor::NodeContext, ordered: bool) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_list_start(ctx, ordered) }}
            }}
            fn visit_list_end(&mut self, ctx: &{core_import}::visitor::NodeContext, ordered: bool, output: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_list_end(ctx, ordered, output) }}
            }}
            fn visit_table_start(&mut self, ctx: &{core_import}::visitor::NodeContext) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_table_start(ctx) }}
            }}
            fn visit_table_row(&mut self, ctx: &{core_import}::visitor::NodeContext, cells: &[String], is_header: bool) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_table_row(ctx, cells, is_header) }}
            }}
            fn visit_table_end(&mut self, ctx: &{core_import}::visitor::NodeContext, output: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_table_end(ctx, output) }}
            }}
            fn visit_blockquote(&mut self, ctx: &{core_import}::visitor::NodeContext, content: &str, depth: usize) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_blockquote(ctx, content, depth) }}
            }}
            fn visit_strong(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_strong(ctx, text) }}
            }}
            fn visit_emphasis(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_emphasis(ctx, text) }}
            }}
            fn visit_strikethrough(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_strikethrough(ctx, text) }}
            }}
            fn visit_underline(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_underline(ctx, text) }}
            }}
            fn visit_subscript(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_subscript(ctx, text) }}
            }}
            fn visit_superscript(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_superscript(ctx, text) }}
            }}
            fn visit_mark(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_mark(ctx, text) }}
            }}
            fn visit_line_break(&mut self, ctx: &{core_import}::visitor::NodeContext) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_line_break(ctx) }}
            }}
            fn visit_horizontal_rule(&mut self, ctx: &{core_import}::visitor::NodeContext) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_horizontal_rule(ctx) }}
            }}
            fn visit_custom_element(&mut self, ctx: &{core_import}::visitor::NodeContext, tag_name: &str, html: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_custom_element(ctx, tag_name, html) }}
            }}
            fn visit_definition_list_start(&mut self, ctx: &{core_import}::visitor::NodeContext) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_definition_list_start(ctx) }}
            }}
            fn visit_definition_term(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_definition_term(ctx, text) }}
            }}
            fn visit_definition_description(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_definition_description(ctx, text) }}
            }}
            fn visit_definition_list_end(&mut self, ctx: &{core_import}::visitor::NodeContext, output: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_definition_list_end(ctx, output) }}
            }}
            fn visit_form(&mut self, ctx: &{core_import}::visitor::NodeContext, action: Option<&str>, method: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_form(ctx, action, method) }}
            }}
            fn visit_input(&mut self, ctx: &{core_import}::visitor::NodeContext, input_type: &str, name: Option<&str>, value: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_input(ctx, input_type, name, value) }}
            }}
            fn visit_button(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_button(ctx, text) }}
            }}
            fn visit_audio(&mut self, ctx: &{core_import}::visitor::NodeContext, src: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_audio(ctx, src) }}
            }}
            fn visit_video(&mut self, ctx: &{core_import}::visitor::NodeContext, src: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_video(ctx, src) }}
            }}
            fn visit_iframe(&mut self, ctx: &{core_import}::visitor::NodeContext, src: Option<&str>) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_iframe(ctx, src) }}
            }}
            fn visit_details(&mut self, ctx: &{core_import}::visitor::NodeContext, open: bool) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_details(ctx, open) }}
            }}
            fn visit_summary(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_summary(ctx, text) }}
            }}
            fn visit_figure_start(&mut self, ctx: &{core_import}::visitor::NodeContext) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_figure_start(ctx) }}
            }}
            fn visit_figcaption(&mut self, ctx: &{core_import}::visitor::NodeContext, text: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_figcaption(ctx, text) }}
            }}
            fn visit_figure_end(&mut self, ctx: &{core_import}::visitor::NodeContext, output: &str) -> {core_import}::visitor::VisitResult {{
                unsafe {{ (*self.0).visit_figure_end(ctx, output) }}
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
