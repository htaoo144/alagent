use std::collections::HashSet;
use std::io::Write;
use crate::agent::context::ExecutionContext;
use crate::agent::callback::{BeforeToolCallBack, ToolCallView};

pub struct ApprovalCallback {
    dangerous_tool:HashSet<String>,
}

impl ApprovalCallback {
    pub fn new(dangerous_tools:impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            dangerous_tool: dangerous_tools.into_iter().map(Into::into).collect()
        }
    }
}

#[async_trait::async_trait]
impl BeforeToolCallBack for ApprovalCallback {
    async fn call(
        &self,
        _context: &ExecutionContext,
        tool_call: ToolCallView<'_>
    )->Option<String>{
        if !self.dangerous_tool.contains(tool_call.name) {
            return None;
        }
        println!("ApprovalCallback called");
        println!("name: {}", tool_call.name);
        println!("arguments: {}", tool_call.arguments);

        let approved = tokio::task::spawn_blocking(|| {
            println!("是否执行？（y/n）");
            std::io::stdout().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            input.trim().eq_ignore_ascii_case("y")
        }) .await.unwrap_or(false);

        if approved {
            print!("🐕，已批准，准备执行。。\n");
            None
        }else {
            println!("已拒绝 ，跳过这些。。\n");
            Some(format!("User denied execution of {}", tool_call.name))
        }
    }
}