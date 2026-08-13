use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    messages: Vec<MemoryMessage>,
    max_turns: usize,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            max_turns: 20,
        }
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    pub fn add(&mut self, role: impl Into<String>, content: impl Into<String>) {
        self.messages.push(MemoryMessage {
            role: role.into(),
            content: content.into(),
        });
        self.trim();
    }

    pub fn add_user(&mut self, content: impl Into<String>) {
        self.add("user", content);
    }

    pub fn add_assistant(&mut self, content: impl Into<String>) {
        self.add("assistant", content);
    }

    pub fn messages(&self) -> &[MemoryMessage] {
        &self.messages
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    fn trim(&mut self) {
        let max_messages = self.max_turns.saturating_mul(2);
        if self.messages.len() > max_messages {
            let remove = self.messages.len() - max_messages;
            self.messages.drain(..remove);
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let memory = serde_json::from_str(&json)?;
        Ok(memory)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
