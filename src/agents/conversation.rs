//! Conversation management for OpenAI agents
//!
//! This module handles interactive conversations with AI agents about repositories.

use crate::error::Result;
use super::{ConversationContext, OpenAIAgent};
use std::collections::HashMap;

/// Conversation manager for interactive repository analysis
pub struct ConversationManager {
    agent: OpenAIAgent,
    active_conversations: HashMap<String, ConversationContext>,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new(agent: OpenAIAgent) -> Self {
        Self {
            agent,
            active_conversations: HashMap::new(),
        }
    }

    /// Start a new conversation about a repository
    pub async fn start_conversation(&mut self, repository: String) -> Result<String> {
        let context = self.agent.start_conversation(repository).await?;
        let conversation_id = context.id.clone();
        self.active_conversations.insert(conversation_id.clone(), context);
        Ok(conversation_id)
    }

    /// Send a message in an existing conversation
    pub async fn send_message(&mut self, conversation_id: &str, message: String) -> Result<String> {
        if let Some(context) = self.active_conversations.get_mut(conversation_id) {
            self.agent.continue_conversation(context, message).await
        } else {
            Err(crate::error::ProcessorError::new("Conversation not found"))
        }
    }

    /// Get conversation history
    pub fn get_conversation(&self, conversation_id: &str) -> Option<&ConversationContext> {
        self.active_conversations.get(conversation_id)
    }

    /// End a conversation
    pub fn end_conversation(&mut self, conversation_id: &str) -> bool {
        self.active_conversations.remove(conversation_id).is_some()
    }

    /// List all active conversations
    pub fn list_conversations(&self) -> Vec<String> {
        self.active_conversations.keys().cloned().collect()
    }
} 