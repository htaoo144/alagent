use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug,Serialize,Deserialize)]
#[serde(tag = "type")]
pub enum ContentItem{
    #[serde(rename = "message")]
    Message{
        role:String,
        content:String,
    },
    #[serde(rename = "tool_call")]
    ToolCall{
        tool_call_id:String,
        name:String,
        arguments:Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult{
        tool_call_id:String,
        name:String,
        status:ToolResultStatus,
        content:String,
    },
}
#[derive(Debug,Serialize,Eq,Clone,Copy,PartialEq,Deserialize)]
pub enum ToolResultStatus{
    Success,
    Error,

}
#[derive(Debug,Serialize,Deserialize)]
pub struct Event{
    pub id:String,
    pub execution_id:String,
    pub timestamp: i64,
    pub author:String,
    pub content:Vec<ContentItem>,
}

impl Event{
    pub fn new(
        execution_id:String,
        author:String,
        content:Vec<ContentItem>,
    )->Self{
        Self{
            id :Uuid::new_v4().to_string(),
            execution_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
            author,
            content,
        }
    }
}






