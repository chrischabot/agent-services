//! Mock LLM returning a canned response (default: an empty additive payload).
//! Useful for a runnable default facade without API keys, and for tests.

use crate::config::LlmSettings;
use arcwell_memory_core::error::Result;
use arcwell_memory_core::traits::{GenerateOptions, Llm};
use arcwell_memory_core::types::Message;
use async_trait::async_trait;

/// A mock LLM that always returns a fixed response.
pub struct MockLlm {
    response: String,
}

impl MockLlm {
    /// Build from settings (`config.response`, else an empty additive payload).
    pub fn new(settings: &LlmSettings) -> Self {
        // `response` is not a typed field; default to empty additive output.
        let _ = settings;
        Self {
            response: "{\"memory\": []}".to_string(),
        }
    }
}

#[async_trait]
impl Llm for MockLlm {
    async fn generate(&self, _messages: &[Message], _options: &GenerateOptions) -> Result<String> {
        Ok(self.response.clone())
    }
}
