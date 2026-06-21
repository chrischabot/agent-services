//! LLM providers for the Arcwell Memory.
//!
//! Each provider implements [`arcwell_memory_core::traits::Llm`] over `reqwest`
//! (rustls-TLS). Construct providers by name via [`factory::build_llm`].

pub mod anthropic;
pub mod azure;
pub mod config;
pub mod factory;
pub mod gemini;
pub mod mock;
pub mod ollama;
pub mod openai;

pub use config::LlmSettings;
pub use factory::build_llm;

use arcwell_memory_core::error::Mem0Error;
use arcwell_memory_core::types::Message;
use serde_json::{Value, json};

/// Map a transport-level reqwest error into a `Mem0Error::Llm`.
pub(crate) fn http_error(context: &str, err: reqwest::Error) -> Mem0Error {
    Mem0Error::llm(format!("{context}: {err}"))
}

/// Convert messages into `[{role, content}]` wire form (drops `name` for
/// providers that reject it).
pub(crate) fn to_wire_messages(messages: &[Message]) -> Vec<Value> {
    messages
        .iter()
        .map(|m| json!({ "role": m.role, "content": m.content }))
        .collect()
}
