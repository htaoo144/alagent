use serde_json::Value;
use crate::agent::context::ExecutionContext;
use crate::agent::event::ToolResultStatus;

#[derive(Debug,Clone,Copy)]
pub struct ToolCallView<'a>{
    pub tool_call_id :& 'a str,
    pub name : & 'a str,
    pub arguments: & 'a Value
}

#[async_trait::async_trait]
pub trait BeforeToolCallBack : Sync + Send {
    async fn call(&self, context:&ExecutionContext, tool_call:ToolCallView<'_>) ->Option<String>;
}

#[async_trait::async_trait]
pub trait AfterToolCallBack : Sync + Send {
    async fn call(
        &self,
        context:&ExecutionContext,
        tool_call_id:&str,
        tool_name: &str,
        status:ToolResultStatus,
        content_item: &str
    )->Option<(ToolResultStatus,String)>;
}