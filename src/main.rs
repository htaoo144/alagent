use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use alagent::complete::chat_complete;
use alagent::constant::AGENT_MODEL;
use alagent::tools::build_toolbox;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let tools = build_toolbox().await?;
    let plan=chat_complete(AGENT_MODEL, r"读取一下C:\Users\32729\Desktop\111.txt的内容，并一字不差的在你的reponse中打印出来",&tools).await?;
    println!("Response:{plan}");
    Ok(())
}
