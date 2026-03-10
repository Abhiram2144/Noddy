// AI module for LLM orchestration and tool execution.
pub mod chat_service;
pub mod intent_router;
pub mod llm_client;
pub mod orchestrator;
pub mod prompt_templates;
pub mod tool_executor;

pub use chat_service::handle_chat;
