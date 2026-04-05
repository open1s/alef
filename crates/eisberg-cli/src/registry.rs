use eisberg_core::backend::Backend;
use eisberg_core::config::Language;

/// Get the backend for a given language.
pub fn get_backend(lang: Language) -> Box<dyn Backend> {
    match lang {
        Language::Python => Box::new(eisberg_backend_pyo3::Pyo3Backend),
        Language::Node => Box::new(eisberg_backend_napi::NapiBackend),
        Language::Ruby => Box::new(eisberg_backend_magnus::MagnusBackend),
        Language::Php => Box::new(eisberg_backend_php::PhpBackend),
        Language::Elixir => Box::new(eisberg_backend_rustler::RustlerBackend),
        Language::Wasm => Box::new(eisberg_backend_wasm::WasmBackend),
        Language::Ffi => Box::new(eisberg_backend_ffi::FfiBackend),
        Language::Go => Box::new(eisberg_backend_go::GoBackend),
        Language::Java => Box::new(eisberg_backend_java::JavaBackend),
        Language::Csharp => Box::new(eisberg_backend_csharp::CsharpBackend),
        Language::R => Box::new(eisberg_backend_extendr::ExtendrBackend),
    }
}
