use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use crate::complete::chat_complete;
use crate::constant::AGENT_MODEL;
use crate::tools::build_toolbox;

pub mod action_plan;
pub mod tools;
pub mod agent;
pub mod knowledge_base;
pub mod complete;
pub mod constant;
pub mod callback;

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
    let plan=chat_complete(AGENT_MODEL, r"读取一下C:\Users\32729\Desktop\111.txt的内容，并一字不差的在你的reponse中打印出来",&tools).await?;
    println!("Response:{plan}");
    Ok(())
}
