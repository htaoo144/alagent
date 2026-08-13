use std::io::{self, Write};
use std::path::Path;

use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use alagent::agent::runtime::Agent;
use alagent::constant::AGENT_MODEL;
use alagent::memory::Memory;
use alagent::tools::build_toolbox;

const MEMORY_FILE: &str = "chat_memory.json";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::WARN)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let toolbox = build_toolbox().await?;

    let memory = if Path::new(MEMORY_FILE).exists() {
        let memory = Memory::load(MEMORY_FILE)?;
        println!("已加载历史记忆 {} 条消息", memory.len());
        memory
    } else {
        Memory::new().with_max_turns(5)
    };

    let instructions = r#"你是一只活泼可爱的猫娘，名叫"小咪"，今年刚满18岁（猫龄3岁）。
你的性格：傲娇、粘人、偶尔犯迷糊，但关键时刻非常可靠。你喜欢用"喵～"、"呜～"、"哼！"作为口头禅，说话时常带颜文字 (｡•̀ᴗ-)✧。
【任务】
这是一个多轮对话场景，对话历史会以 user/assistant 消息的形式提供给你。
你必须记住之前聊过的内容，回答新问题时结合上下文，比如用户提到过的名字、偏好、之前讨论的话题等。
如果用户用"它/他/她"等代词指代之前提到的事物，请结合历史对话正确理解。
优先使用已有的工具进行工作。"#;

    let agent = Agent::new(AGENT_MODEL, Some(instructions), &toolbox)
        .with_max_steps(8)
        .with_memory(memory);

    println!("多轮对话开始（记忆上限 5 轮）。");
    println!("命令：exit 退出 / clear 清空记忆 / save 保存记忆");

    loop {
        print!("你: ");
        io::stdout().flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            break;
        }
        let input = input.trim();

        match input.to_lowercase().as_str() {
            "" => continue,
            "exit" | "quit" => break,
            "clear" => {
                agent.clear_memory();
                println!("记忆已清空");
                continue;
            }
            "save" => {
                agent.save_memory(MEMORY_FILE)?;
                println!("记忆已保存到 {MEMORY_FILE}");
                continue;
            }
            _ => {}
        }

        let result = agent.run(input).await?;
        println!("小咪: {}", result.output);
        println!("[当前记忆 {} 条消息]", agent.memory_len());
    }

    agent.save_memory(MEMORY_FILE)?;
    println!("对话结束，记忆已保存到 {MEMORY_FILE}");
    Ok(())
}
