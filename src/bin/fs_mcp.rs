use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{DateTime, Local};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::transport::stdio;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io};

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

const MAX_READ_CHARS: usize = 20_000;
const MAX_READ_IMAGE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_LIST_ENTRIES: usize = 200;
const MAX_SEARCH_ENTRIES: usize = 100;
const MAX_RECURSE_DEPTH: usize = 10;
const MAX_EXTRACT_BYTES: u64 = 512 * 1024 * 1024;

fn default_max_chars() -> usize {
    MAX_READ_CHARS
}
fn default_max_entries() -> usize {
    MAX_LIST_ENTRIES
}
fn default_max_search_entries() -> usize {
    MAX_SEARCH_ENTRIES
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn format_time(time: SystemTime) -> String {
    let dt: DateTime<Local> = time.into();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    kind: String,
    size: u64,
    size_human: String,
    modified: String,
}

fn collect_entries(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    entries: &mut Vec<FileEntry>,
    max_entries: usize,
) -> Result<(), String> {
    if entries.len() >= max_entries {
        return Ok(());
    }
    let read = fs::read_dir(dir).map_err(|e| format!("读取目录失败 {}: {e}", dir.display()))?;
    let mut children: Vec<_> = read.filter_map(|r| r.ok()).collect();
    children.sort_by_key(|c| c.file_name().to_string_lossy().to_lowercase());

    for child in children {
        if entries.len() >= max_entries {
            break;
        }
        let path = child.path();
        let meta = match child.metadata() {
            Ok(meta) => meta,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        entries.push(FileEntry {
            name: path.display().to_string(),
            kind: if is_dir { "dir" } else { "file" }.to_string(),
            size: meta.len(),
            size_human: format_size(meta.len()),
            modified: format_time(meta.modified().unwrap_or(UNIX_EPOCH)),
        });
        if is_dir && depth < max_depth {
            collect_entries(&path, depth + 1, max_depth, entries, max_entries)?;
        }
    }
    Ok(())
}

fn list_dir(path: &Path, recursive: bool, max_entries: usize) -> Result<Vec<FileEntry>, String> {
    if !path.exists() {
        return Err(format!("路径不存在: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("不是目录: {}", path.display()));
    }
    let max_depth = if recursive { MAX_RECURSE_DEPTH } else { 0 };
    let mut entries = Vec::new();
    collect_entries(path, 0, max_depth, &mut entries, max_entries)?;
    entries.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.name.cmp(&b.name)));
    Ok(entries)
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {e}"))?;
    }
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadFileArgs {
    /// 要读取的文件路径
    pub path: String,
    /// 最大读取字符数，超出部分截断，默认 20000
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListFilesArgs {
    /// 目录路径
    pub path: String,
    /// 是否递归列出子目录，默认 false
    #[serde(default)]
    pub recursive: bool,
    /// 最大返回条数，默认 200
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadImageArgs {
    /// 图片文件路径（支持 png/jpg/jpeg/gif/webp/bmp/svg）
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteFileArgs {
    /// 文件路径
    pub path: String,
    /// 文件内容
    pub content: String,
    /// 文件已存在时是否覆盖，默认 false
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateDirArgs {
    /// 目录路径（自动创建父目录）
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteFileArgs {
    /// 要删除的文件或目录路径
    pub path: String,
    /// 删除非空目录时必须为 true，默认 false
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveFileArgs {
    /// 源路径
    pub from: String,
    /// 目标路径
    pub to: String,
    /// 目标已存在时是否覆盖，默认 false
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CopyFileArgs {
    /// 源文件路径
    pub from: String,
    /// 目标文件路径
    pub to: String,
    /// 目标已存在时是否覆盖，默认 false
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchFilesArgs {
    /// 搜索起始目录
    pub root: String,
    /// 通配符模式，如 "*.rs"、"**/*.txt"
    pub pattern: String,
    /// 最大返回条数，默认 100
    #[serde(default = "default_max_search_entries")]
    pub max_entries: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileInfoArgs {
    /// 文件或目录路径
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UnzipFileArgs {
    pub zip_path: String,
    #[serde(default)]
    pub extract_to: Option<String>,
}

pub struct FileSystemServer;

impl Default for FileSystemServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl FileSystemServer {
    pub fn new() -> Self {
        FileSystemServer
    }

    #[tool(name = "read_file", description = "读取指定路径的文本文件内容")]
    async fn read_file(
        &self,
        Parameters(req): Parameters<ReadFileArgs>,
    ) -> Result<String, String> {
        let path = Path::new(&req.path);
        if !path.exists() {
            return Err(format!("文件不存在: {}", req.path));
        }
        if path.is_dir() {
            return Err(format!("这是一个目录，请使用 list_files: {}", req.path));
        }
        let file = fs::File::open(path).map_err(|e| format!("打开文件失败: {e}"))?;
        let read_limit = (req.max_chars as u64).saturating_mul(4).saturating_add(4);
        let mut bytes = Vec::new();
        use std::io::Read;
        file.take(read_limit)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("读取文件失败: {e}"))?;
        let mut text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(err) => format!(
                "[警告: 文件不是合法 UTF-8，已按 lossy 解码]\n{}",
                String::from_utf8_lossy(err.as_bytes())
            ),
        };
        let total_chars = text.chars().count();
        if total_chars > req.max_chars {
            let truncated: String = text.chars().take(req.max_chars).collect();
            text = format!(
                "{truncated}\n\n[已截断: 共 {total_chars} 字符，仅显示前 {} 字符]",
                req.max_chars
            );
        }
        Ok(text)
    }

    #[tool(name = "list_files", description = "列出目录内容，返回 JSON 数组")]
    async fn list_files(
        &self,
        Parameters(req): Parameters<ListFilesArgs>,
    ) -> Result<String, String> {
        let entries = list_dir(Path::new(&req.path), req.recursive, req.max_entries)?;
        if entries.is_empty() {
            return Ok("[]".to_string());
        }
        serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())
    }

    #[tool(name = "read_image", description = "读取图片文件，返回 base64 data URI，供视觉模型使用")]
    async fn read_image(
        &self,
        Parameters(req): Parameters<ReadImageArgs>,
    ) -> Result<String, String> {
        let path = Path::new(&req.path);
        if !path.exists() {
            return Err(format!("文件不存在: {}", req.path));
        }
        if path.is_dir() {
            return Err(format!("这是一个目录，图片无法读取: {}", req.path));
        }
        let meta = fs::metadata(path).map_err(|e| format!("读取元数据失败: {e}"))?;
        if meta.len() > MAX_READ_IMAGE_BYTES {
            return Err(format!(
                "图片过大: {}（上限 {}）",
                format_size(meta.len()),
                format_size(MAX_READ_IMAGE_BYTES)
            ));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let mime = match ext.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "svg" => "image/svg+xml",
            _ => return Err(format!("不支持的图片格式: .{ext}")),
        };
        let bytes = fs::read(path).map_err(|e| format!("读取图片失败: {e}"))?;
        let encoded = STANDARD.encode(&bytes);
        Ok(format!("data:{mime};base64,{encoded}"))
    }

    #[tool(name = "write_file", description = "将内容写入文件，自动创建父目录")]
    async fn write_file(
        &self,
        Parameters(req): Parameters<WriteFileArgs>,
    ) -> Result<String, String> {
        let path = Path::new(&req.path);
        if path.exists() && !req.overwrite {
            return Err(format!(
                "文件已存在（如需覆盖请设置 overwrite=true）: {}",
                req.path
            ));
        }
        ensure_parent(path)?;
        fs::write(path, &req.content).map_err(|e| format!("写入文件失败: {e}"))?;
        Ok(format!(
            "已写入 {} 字符到 {}",
            req.content.chars().count(),
            req.path
        ))
    }

    #[tool(name = "create_dir", description = "创建目录（自动创建父目录）")]
    async fn create_dir(&self, Parameters(req): Parameters<CreateDirArgs>) -> Result<String, String> {
        fs::create_dir_all(&req.path).map_err(|e| format!("创建目录失败: {e}"))?;
        Ok(format!("已创建目录: {}", req.path))
    }

    #[tool(name = "delete_file", description = "删除文件或目录（删除非空目录需设置 recursive=true）")]
    async fn delete_file(
        &self,
        Parameters(req): Parameters<DeleteFileArgs>,
    ) -> Result<String, String> {
        let path = Path::new(&req.path);
        if !path.exists() {
            return Err(format!("路径不存在: {}", req.path));
        }
        if path.is_dir() {
            let is_empty = fs::read_dir(path)
                .map(|mut it| it.next().is_none())
                .unwrap_or(false);
            if is_empty {
                fs::remove_dir(path).map_err(|e| format!("删除目录失败: {e}"))?;
            } else if req.recursive {
                fs::remove_dir_all(path).map_err(|e| format!("删除目录失败: {e}"))?;
            } else {
                return Err(format!(
                    "目录非空（如需递归删除请设置 recursive=true）: {}",
                    req.path
                ));
            }
        } else {
            fs::remove_file(path).map_err(|e| format!("删除文件失败: {e}"))?;
        }
        Ok(format!("已删除: {}", req.path))
    }

    #[tool(name = "move_file", description = "移动或重命名文件/目录")]
    async fn move_file(
        &self,
        Parameters(req): Parameters<MoveFileArgs>,
    ) -> Result<String, String> {
        let from = Path::new(&req.from);
        let to = Path::new(&req.to);
        if !from.exists() {
            return Err(format!("源路径不存在: {}", req.from));
        }
        if to.exists() {
            if !req.overwrite {
                return Err(format!(
                    "目标已存在（如需覆盖请设置 overwrite=true）: {}",
                    req.to
                ));
            }
            if to.is_dir() {
                fs::remove_dir_all(to).map_err(|e| format!("清理目标失败: {e}"))?;
            } else {
                fs::remove_file(to).map_err(|e| format!("清理目标失败: {e}"))?;
            }
        }
        ensure_parent(to)?;
        match fs::rename(from, to) {
            Ok(()) => {}
            Err(_) if from.is_file() => {
                tracing::warn!(
                    "rename 失败（可能跨卷），回退为复制+删除: {} -> {}",
                    req.from,
                    req.to
                );
                fs::copy(from, to).map_err(|e| format!("复制失败: {e}"))?;
                fs::remove_file(from).map_err(|e| format!("删除源文件失败: {e}"))?;
            }
            Err(e) => return Err(format!("移动失败: {e}")),
        }
        Ok(format!("已移动: {} -> {}", req.from, req.to))
    }

    #[tool(name = "copy_file", description = "复制文件（不支持目录）")]
    async fn copy_file(
        &self,
        Parameters(req): Parameters<CopyFileArgs>,
    ) -> Result<String, String> {
        let from = Path::new(&req.from);
        let to = Path::new(&req.to);
        if !from.exists() {
            return Err(format!("源文件不存在: {}", req.from));
        }
        if from.is_dir() {
            return Err(format!("源路径是目录，请使用 move_file: {}", req.from));
        }
        if to.exists() && !req.overwrite {
            return Err(format!(
                "目标已存在（如需覆盖请设置 overwrite=true）: {}",
                req.to
            ));
        }
        ensure_parent(to)?;
        fs::copy(from, to).map_err(|e| format!("复制失败: {e}"))?;
        Ok(format!("已复制: {} -> {}", req.from, req.to))
    }

    #[tool(name = "search_files", description = "按通配符模式搜索文件，如 *.rs、**/*.txt，返回 JSON 数组")]
    async fn search_files(
        &self,
        Parameters(req): Parameters<SearchFilesArgs>,
    ) -> Result<String, String> {
        let root = Path::new(&req.root);
        if !root.is_dir() {
            return Err(format!("根目录不存在或不是目录: {}", req.root));
        }
        let pattern = req.pattern.replace('\\', "/");
        let pattern_path = root.join(&pattern);
        let pattern_str = pattern_path
            .to_str()
            .ok_or_else(|| "路径包含非法字符".to_string())?;

        let paths: Vec<PathBuf> = glob::glob(pattern_str)
            .map_err(|e| format!("模式错误: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        let mut entries = Vec::new();
        for path in paths {
            if entries.len() >= req.max_entries {
                break;
            }
            let meta = match fs::metadata(&path) {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            let is_dir = meta.is_dir();
            entries.push(FileEntry {
                name: path.display().to_string(),
                kind: if is_dir { "dir" } else { "file" }.to_string(),
                size: meta.len(),
                size_human: format_size(meta.len()),
                modified: format_time(meta.modified().unwrap_or(UNIX_EPOCH)),
            });
        }
        if entries.is_empty() {
            return Ok(format!(
                "在 {} 下未找到匹配 \"{}\" 的文件",
                req.root, req.pattern
            ));
        }
        serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())
    }

    #[tool(name = "file_info", description = "查看文件或目录的元数据信息")]
    async fn file_info(&self, Parameters(req): Parameters<FileInfoArgs>) -> Result<String, String> {
        let path = Path::new(&req.path);
        let meta = fs::metadata(path).map_err(|e| format!("获取元数据失败: {e}"))?;
        let info = serde_json::json!({
            "path": req.path,
            "kind": if meta.is_dir() { "dir" } else { "file" },
            "size": meta.len(),
            "size_human": format_size(meta.len()),
            "created": meta.created().ok().map(format_time),
            "modified": meta.modified().ok().map(format_time),
            "readonly": meta.permissions().readonly(),
        });
        serde_json::to_string_pretty(&info).map_err(|e| e.to_string())
    }

    #[tool(name = "unzip_file", description = "解压 zip 文件")]
    async fn unzip_file(
        &self,
        Parameters(req): Parameters<UnzipFileArgs>,
    ) -> Result<String, String> {
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
        let mut total_size = 0u64;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;

            let Some(enclosed) = entry.enclosed_name() else {
                continue;
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
                let written = io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
                total_size += written;
                if total_size > MAX_EXTRACT_BYTES {
                    return Err(format!(
                        "解压内容超过大小上限 {}（疑似 zip 炸弹），已中止",
                        format_size(MAX_EXTRACT_BYTES)
                    ));
                }
            }
        }

        let mut summary = format!(
            "Extracted {} files ({} in total) to {}\n\nContents:\n",
            names.len(),
            format_size(total_size),
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
}

#[tool_handler]
impl ServerHandler for FileSystemServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
            .with_server_info(
                Implementation::new("fs-mcp-server", "1.0.0")
                    .with_title("Filesystem MCP Server")
                    .with_description(
                        "文件系统操作工具集：读取/写入/删除/移动/复制文件、目录管理、图片读取、通配符搜索、zip 解压",
                    ),
            )
            .with_instructions("请使用 tools 列表查看可用工具。")
    }
}
