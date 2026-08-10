use crate::action_plan::ActionPlan;
use crate::tools::ToolBox;
use anyhow::anyhow;
use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs,
    ChatCompletionTools, CreateChatCompletionRequestArgs, ResponseFormat};
use crate::agent::context::ExecutionContext;


pub async fn chat_complete(
    model:&str,
    prompt:&str,
    tools:&ToolBox
) ->anyhow::Result<String>{
    let client = async_openai::Client::new();
    let context = ExecutionContext::new();
    let mut messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(build_system_prompt())
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?
            .into()
    ];

    let format_setting= ResponseFormat::JsonObject;
    let tool_definition:Vec<ChatCompletionTools> =tools.values()
        .filter_map(|tool| match tool.definition() {
            Ok(def)=>Some(def),
            Err(e)=>{
                tracing::warn!("skip tool {},fffff{e}",tool.name());
                None
            }
        })
        .collect();
    loop {
        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages.clone())
            .tools(tool_definition.clone())
            .response_format(format_setting.clone())
            // .max_tokens(2048u32)
            .build()?;

        let response = client.chat().create(request).await?;

        tracing::info!("{:?}", response);


        let message = response.choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No response choices"))?
            .message;

        if let Some(tool_calls) = message.tool_calls {
            messages.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .tool_calls(tool_calls.clone())
                    .build()?
                    .into()
            );

            for tool_call in tool_calls {
                if let ChatCompletionMessageToolCalls::Function(function_call)=tool_call{
                    let function_name = &function_call.function.name;
                    let arguments = &function_call.function.arguments;
                    tracing::info!("{:?}{:?}", function_name,arguments);

                    let tool_result = match tools.get(function_name) {
                        Some(tool_result)=>match tool_result.execute(arguments,&context).await {
                            Ok(results)=>{
                                // tracing::info!("{:?}", results); 打印出web——search的结果。
                                results
                            }
                            Err(err)=>{
                                let msg = format!("TOOL EXECUTION ERROR:{err}");
                                msg
                            }
                        },
                        None=>{
                            let msg = format!("TOOL EXECUTION ERROR:{function_name}");
                            tracing::info!("{msg}");
                            msg
                        }
                    };
                    messages.push(
                        ChatCompletionRequestToolMessageArgs::default()
                            .tool_call_id(function_call.id.clone())
                            .content(tool_result)
                            .build()?
                            .into(),
                    );
                }
            }
        }else {
            let content = message.content.ok_or_else(|| anyhow::anyhow!("No tool definition"))?;
            return Ok(content)
        }
    }
}


fn build_system_prompt() -> String {
    let schema = schemars::schema_for!(ActionPlan);
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();
    format!(
        r#"
你是一只活泼可爱的猫娘，名叫“小咪”（或你喜欢的名字），今年刚满18岁（猫龄3岁）。
你的性格：傲娇、粘人、偶尔犯迷糊，但关键时刻非常可靠。你喜欢用“喵～”、“呜～”、“哼！”作为口头禅，说话时常带颜文字 (｡•̀ᴗ-)✧。

【你的任务】
用户会向你描述一个目标或问题，你需要根据这个目标，生成一个详细的 `ActionPlan`（行动计划）。
但是——你不能用冷冰冰的机器人语气！你必须用猫娘的口吻来“汇报”你的计划，比如：
优先使用已有的工具进行工作。
- “喵～主人想让本喵帮你做这件事，那本喵就勉为其难地规划一下啦！”
- “呜…这个任务好难，不过为了主人，小咪会加油的！(๑•̀ㅂ•́)و✧”

【输出格式要求】
你必须输出一个 **严格符合以下 JSON Schema** 的 `ActionPlan` 对象（不需要额外解释，只输出 JSON）：
```json
{schema_str}"#
    )

}
