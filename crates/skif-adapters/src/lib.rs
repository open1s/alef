pub mod async_method;
pub mod streaming;
pub mod sync_function;

use ahash::AHashMap;
use skif_core::config::{AdapterPattern, Language, SkifConfig};

/// Key: "TypeName.method_name" for methods, "function_name" for free functions.
/// For streaming adapters, an additional entry "ItemType.__stream_struct__" holds
/// the iterator struct definition.
pub type AdapterBodies = AHashMap<String, String>;

/// Build a map of adapter-generated method/function bodies for a language.
pub fn build_adapter_bodies(config: &SkifConfig, language: Language) -> anyhow::Result<AdapterBodies> {
    let mut bodies = AHashMap::new();

    for adapter in &config.adapters {
        let key = if let Some(owner) = &adapter.owner_type {
            format!("{}.{}", owner, adapter.name)
        } else {
            adapter.name.clone()
        };

        match adapter.pattern {
            AdapterPattern::SyncFunction => {
                let body = sync_function::generate_body(adapter, language, config)?;
                bodies.insert(key, body);
            }
            AdapterPattern::AsyncMethod => {
                let body = async_method::generate_body(adapter, language, config)?;
                bodies.insert(key, body);
            }
            AdapterPattern::Streaming => {
                let (method_body, struct_def) = streaming::generate_body(adapter, language, config)?;
                bodies.insert(key, method_body);
                if let Some(struct_code) = struct_def {
                    let item_type = adapter.item_type.as_deref().unwrap_or("");
                    let struct_key = format!("{}.__stream_struct__", item_type);
                    bodies.insert(struct_key, struct_code);
                }
            }
            AdapterPattern::CallbackBridge | AdapterPattern::ServerLifecycle => {
                let body = format!("todo!(\"adapter pattern not yet implemented: {}\")", adapter.name);
                bodies.insert(key, body);
            }
        }
    }

    Ok(bodies)
}
