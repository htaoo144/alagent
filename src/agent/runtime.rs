use std::sync::Arc;
use crate::agent::context::ExecutionContext;
use crate::tools::ToolBox;
use async_openai::types::assistants::FunctionCall;
use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionTools,
    CreateChatCompletionRequestArgs
};
use serde_json::Value;
use crate::agent::callback::{ AfterToolCallBack, BeforeToolCallBack, ToolCallView};
use crate::agent::event::{ContentItem, Event, ToolResultStatus};

#[derive(Debug)]
pub struct AgentResult{
    pub output:String,
    pub context:ExecutionContext
}

pub struct Agent<'a>{
    model:&'a str,
    instructions:Option<&'a str>,
    toolbox:&'a ToolBox,
    max_steps:u32,
    before_tool_callback:Vec<Arc<dyn BeforeToolCallBack>>,
    after_tool_callback:Vec<Arc<dyn AfterToolCallBack>>,
}

impl<'a> Agent<'a>{
    pub fn new(model:&'a str,instructions:Option<&'a str>,toolbox:&'a ToolBox)->Self{
        Self{
            model,
            instructions,
            toolbox,
            max_steps:10,
            before_tool_callback:Vec::new(),
            after_tool_callback:Vec::new(),
        }
    }

    pub fn with_max_steps(mut self,max_steps:u32)->Self{
        self.max_steps=max_steps;
        self
    }
    pub fn with_before_tool_callback(mut self,callback:Arc<dyn BeforeToolCallBack>)->Self{
        self.before_tool_callback.push(callback);
        self
    }
    pub fn with_after_tool_callback(mut self,callback:Arc<dyn AfterToolCallBack>)->Self{
        self.after_tool_callback.push(callback);
        self
    }
    pub async fn run(&self,user_input:&str)->anyhow::Result<AgentResult>{
        let mut context = ExecutionContext::new();
        context.add_event(Event::new(
            context.execution_id.clone(),
            "user".to_string(),
            vec![ContentItem::Message {
                role:"user".to_string(),
                content:user_input.to_string(),
            }],
        ));

        let client = async_openai::Client::new();

        let tool_definitions:Vec<ChatCompletionTools>=self
            .toolbox
            .values()
            .filter_map(|tool| match tool.definition() {
                Ok(def)=>Some(def),
                Err(e)=>{
                    tracing::warn!("Skip tool:{},failed to get its definition: {}",tool.name(),e);
                    None
                }
            }).collect();
        loop {
            if context.current_step >= self.max_steps{
                anyhow::bail!("Max step exceeded");
            }

            let messages = self.build_messages(&context)?;

            let request = CreateChatCompletionRequestArgs::default()
                .model(self.model)
                .messages(messages)
                .tools(tool_definitions.clone())
                .max_tokens(2048u32)
                .build()?;

            let response = client.chat().create(request).await?;

            let message = response.choices
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("No message"))?
                .message;

            if let Some(tool_call) = message.tool_calls{
                self.record_tool_calls(&mut context,&tool_call);
                self.execute_tool_calls(&mut context, &tool_call).await;
            }else {
                let content = message.content
                    .ok_or_else(|| anyhow::anyhow!("No message"))?;

                context.add_event(Event::new(
                    context.execution_id.clone(),
                    "agent".to_string(),
                    vec![ContentItem::Message {
                        role:"assistant".to_string(),
                        content:content.to_string(),
                    }],
                ));
                context.final_result= Some(content.clone());
                return Ok(AgentResult{
                    output:content,
                    context
                });
            }
            context.increment_step();
        }
    }

    fn record_tool_calls(
        &self,
        content:&mut ExecutionContext,
        tool_calls:&[ChatCompletionMessageToolCalls]
    ){
        let mut call_item = Vec::new();
        for call in tool_calls{
            if let ChatCompletionMessageToolCalls::Function(function_call) = call {
                let arguments:serde_json::Value= serde_json::from_str(&function_call.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                call_item.push(ContentItem::ToolCall {
                    tool_call_id:function_call.id.clone(),
                    name:function_call.function.name.clone(),
                    arguments,
                });
            }
        }
        content.add_event(Event::new(
            content.execution_id.clone(),
            "agent".to_string(),
            call_item,
        ))
    }

    async fn execute_tool_calls(
        &self,
        context:&mut ExecutionContext,
        tool_calls:&[ChatCompletionMessageToolCalls]
    ){
        let mut result_item = Vec::new();

        for call in tool_calls{
            let ChatCompletionMessageToolCalls::Function(function_call) = call else { continue };
            let function_name = &function_call.function.name;
            let arguments = &function_call.function.arguments;
            tracing::info!("tool call {},{}", function_name, arguments);

            let arguments_value:Value=serde_json::from_str(arguments).unwrap_or(Value::Null);
            let view = ToolCallView{
                tool_call_id: &function_call.id,
                name: function_name,
                arguments: &arguments_value,
            };

            let mut short_circuited =None;
            for callback in &self.before_tool_callback{
                if let Some(result) = callback.call(context, view).await{
                    short_circuited=Some(result);
                    break;
                }
            }

            let (mut status,mut content) = match short_circuited {
                Some(result) => (ToolResultStatus::Success,result),
                None =>match self.toolbox.get(function_name) {
                    Some(tool)=>match tool.execute(arguments,context).await {
                        Ok(result)=>{
                            // tracing::info!("tool result:{result}");
                            (ToolResultStatus::Success,result)
                        }
                        Err(err)=>{
                            let msg= format!("TOOL execute error{err}");
                            tracing::error!("{msg}");
                            (ToolResultStatus::Error,msg.to_string())
                        }
                    },
                    None =>{
                        let msg= format!("TOOL execute error:unknow tool{function_name}");
                        tracing::error!("{msg}");
                        (ToolResultStatus::Error,msg.to_string())

                    }
                },
            };
            for callback in &self.after_tool_callback{
                if let Some((new_status,new_content)) =callback
                    .call(
                        context,
                        &function_call.id,
                        function_name,
                        status,
                        &content
                    ).await
                {
                    status = new_status;
                    content = new_content;
                    break;
                }
            }

            result_item.push(
                ContentItem::ToolResult {
                    tool_call_id: function_call.id.clone(),
                    name:function_name.clone(),
                    status,
                    content,
                });
        }
        context.add_event(Event::new(
            context.execution_id.clone(),
            "tool".to_string(),
            result_item,
        ));
    }


    fn build_messages(&self,content:&ExecutionContext) -> anyhow::Result<Vec<ChatCompletionRequestMessage>>{
        let mut messages = Vec::new();

        if let Some(system) = self.instructions{
            messages.push(ChatCompletionRequestSystemMessageArgs::default()
                .content(system)
                .build()?
                .into()
            );
        }

        for event in &content.event{
            for item in &event.content{
                match item {
                    ContentItem::Message {role,content}=>{
                        let message:ChatCompletionRequestMessage = if role =="user"{
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(content.clone())
                                .build()?
                                .into()
                        } else {
                            ChatCompletionRequestAssistantMessageArgs::default()
                                .content(content.clone())
                                .build()?
                                .into()
                        };
                        messages.push(message);
                    }
                    ContentItem::ToolCall {
                        tool_call_id,
                        name,
                        arguments,
                    }=>{
                        let tool_call = ChatCompletionMessageToolCalls::Function(
                            ChatCompletionMessageToolCall{
                                id:tool_call_id.clone(),
                                function:FunctionCall{
                                    name:name.clone(),
                                    arguments:arguments.to_string()
                                },
                            },
                        );
                        if let Some(ChatCompletionRequestMessage::Assistant(last))=
                            messages.last_mut(){
                            last.tool_calls.get_or_insert_with(Vec::new).push(tool_call);
                        } else {
                            messages.push(ChatCompletionRequestAssistantMessageArgs::default()
                                .tool_calls(vec![tool_call])
                                .build()?
                                .into()
                            );
                        }
                    }
                    ContentItem::ToolResult {
                        tool_call_id,
                        content,
                        ..
                    } =>{
                        messages.push(
                            ChatCompletionRequestToolMessageArgs::default()
                                .tool_call_id(tool_call_id.clone())
                                .content(content.clone())
                                .build()?
                                .into(),
                        );
                    }
                }
            }
        }
        Ok(messages)
    }

}

