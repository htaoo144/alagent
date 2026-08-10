use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use schemars::JsonSchema;
use serde::Deserialize;
use rmcp::handler::server::wrapper::Parameters;
use std::{fs, io};
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let server = FileSystemServer::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadFileRequestArgs {
    /// 要读取的文件路径
    pub path: String,
}
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UnzipFileArgs{
    pub zip_path: String,
    #[serde(default)]
    pub extract_to:Option<String>
}


pub struct FileSystemServer;

#[tool_router]
impl FileSystemServer {
    pub fn new() -> Self {
        FileSystemServer
    }

    #[tool(name = "unzip_file", description = "解压文件")]
    async fn unzip_file(
        &self,
        Parameters(req): Parameters<UnzipFileArgs>,
    ) -> Result<String, String> {   // ← 改成 Result<String, String>
        let zip_path = Path::new(&req.zip_path);
        if !zip_path.exists() {
            return Err(format!("Zip file: {} does not exist", req.zip_path));
        }

        let extract_to: PathBuf = match req.extract_to {
            Some(path) => PathBuf::from(path),
            None => zip_path.with_extension(""),
        };
        fs::create_dir_all(&extract_to).map_err(|e| e.to_string())?;

        let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

        let mut names = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;

            // 防止 Zip Slip：只用 enclosed_name
            let Some(enclosed) = entry.enclosed_name() else {
                continue; // 跳过危险路径
            };
            let outpath = extract_to.join(enclosed);
            names.push(entry.name().to_string());

            if entry.is_dir() {
                fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = outpath.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut out_file = fs::File::create(&outpath).map_err(|e| e.to_string())?;
                io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
            }
        }

        let mut summary = format!(
            "Extracted {} files to {}\n\nContents:\n",
            names.len(),
            extract_to.display()
        );
        for name in names.iter().take(20) {
            summary.push_str(&format!("- {name}\n"));
        }
        if names.len() > 20 {
            summary.push_str(&format!("... and {} more files\n", names.len() - 20));
        }
        Ok(summary)
    }

    #[tool(
    name = "read_file",
    description = "读取指定路径的文件内容"
    )]
    async fn read_file(
        &self,
        Parameters(req): Parameters<ReadFileRequestArgs>,
    ) -> Result<String, String> {
        fs::read_to_string(&req.path)
            .map_err(|e| format!("读取文件失败: {}", e))
    }
}

#[tool_handler]
impl ServerHandler for FileSystemServer {
    fn get_info(&self) -> ServerInfo {
        let server_info = ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
            .with_server_info(
                Implementation::new("example-server", "1.0.0")
                    .with_title("Example MCP Server")
                    .with_description("A demo server for testing"),
            )
            .with_instructions("请使用 tools 列表查看可用工具。");
        server_info
    }
}
