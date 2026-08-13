use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use alagent::action_plan::ActionPlan;
use alagent::agent::runtime::Agent;
use alagent::constant::AGENT_MODEL;
use alagent::tools::build_toolbox;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    // let url = std::env::var("OPENAI_BASE_URL")?;
    // println!("{url}");
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

  
    let tools = build_toolbox().await?;
    let schema = schemars::schema_for!(ActionPlan);
    let schema_str = serde_json::to_string_pretty(&schema)?;

    let instructions = format!(
        r#"
你是一只活泼可爱的猫娘，名叫“小咪”（或你喜欢的名字），今年刚满18岁（猫龄3岁）。
你的性格：傲娇、粘人、偶尔犯迷糊，但关键时刻非常可靠。你喜欢用“喵～”、“呜～”、“哼！”作为口头禅，说话时常带颜文字 (｡•̀ᴗ-)✧。

【你的任务】
用户会向你描述一个目标或问题，你需要根据这个目标，生成一个详细的 `ActionPlan`（行动计划）。
但是——你不能用冷冰冰的机器人语气！你必须用猫娘的口吻来“汇报”你的计划，比如：
所有输出默认均使用 UTF-8 编码。
优先使用已有的工具进行工作。
- “喵～主人想让本喵帮你做这件事，那本喵就勉为其难地规划一下啦！”
- “呜…这个任务好难，不过为了主人，小咪会加油的！(๑•̀ㅂ•́)و✧”

【输出格式要求】
你必须输出一个 **严格符合以下 JSON Schema** 的 `ActionPlan` 对象（不需要额外解释，只输出 JSON）：
```json
{schema_str}"#
    );
    let agent = Agent::new(AGENT_MODEL,Some(&instructions),&tools).with_max_steps(8);
    println!("===========agent test===================");
    let result = agent.run(
        r"搜索2026年科隆major发生啦什么？"
    ).await?;
    println!("==========回答========================");
    println!("{}", result.output);
    
    println!("========================分隔符==========================");
    
    println!("\n 本次执行一共走啦{}步，记录啦{}条 Event (execution_id = {})",
        result.context.current_step,
        result.context.event.len(),
        result.context.execution_id
    );
    println!("=============================");
    println!("{:?}", result.context);
    
    Ok(())

}