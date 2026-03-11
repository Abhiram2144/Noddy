// AI module for LLM orchestration and tool execution.
pub mod chat_service;
pub mod context_builder;
pub mod intent_router;
pub mod llm_client;
pub mod orchestrator;
pub mod planner;
pub mod prompt_templates;
pub mod schedule_parser;
pub mod tool_executor;

pub use chat_service::handle_chat;
