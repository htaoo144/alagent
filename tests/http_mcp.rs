use std::sync::{Arc, Mutex};
use std::time::Duration;

use alagent::tools::mcp::client::McpClient;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Implementation, ServerCapabilities, ServerInfo};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, JsonSchema)]
struct EchoArgs {
    message: String,
}

#[derive(Clone)]
struct EchoServer;

#[tool_router]
impl EchoServer {
    #[tool(name = "echo", description = "返回传入的消息")]
    async fn echo(&self, Parameters(req): Parameters<EchoArgs>) -> Result<String, String> {
        Ok(format!("echo: {}", req.message))
    }
}

#[tool_handler]
impl ServerHandler for EchoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("echo-server", "1.0.0"))
    }
}

async fn capture_auth(
    State(seen): State<Arc<Mutex<Option<String>>>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    *seen.lock().unwrap() = header;
    next.run(request).await
}

struct TestServer {
    url: String,
    seen: Arc<Mutex<Option<String>>>,
    shutdown: tokio::sync::watch::Sender<bool>,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn stop(self) {
        self.shutdown.send(true).ok();
        let _ = tokio::time::timeout(Duration::from_secs(5), self.handle).await;
    }
}

async fn spawn_echo_server() -> TestServer {
    let seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let service: StreamableHttpService<EchoServer, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(EchoServer),
            Default::default(),
            StreamableHttpServerConfig::default().with_sse_keep_alive(None),
        );
    let router = Router::new()
        .nest_service("/mcp", service)
        .route_layer(middleware::from_fn_with_state(
            seen.clone(),
            capture_auth,
        ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let address = listener.local_addr().expect("listener should have address");

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let mut rx = shutdown_rx;
                rx.changed().await.ok();
            })
            .await;
    });

    TestServer {
        url: format!("http://{address}/mcp"),
        seen,
        shutdown: shutdown_tx,
        handle,
    }
}

#[tokio::test]
async fn connect_http_with_api_key() {
    let server = spawn_echo_server().await;

    let result = {
        let client = McpClient::connect_http(&server.url, Some("test-api-key-123")).await.unwrap();

        let tools = client.list_tools().await.unwrap();
        assert!(
            tools.iter().any(|t| t.name.as_ref() == "echo"),
            "tools: {tools:?}"
        );

        client
            .call_tool("echo", json!({"message": "hello"}))
            .await
            .unwrap()
    };
    assert_eq!(result, "echo: hello");

    let seen = server.seen.lock().unwrap().clone();
    assert_eq!(seen.as_deref(), Some("Bearer test-api-key-123"));

    server.stop().await;
}

#[tokio::test]
async fn connect_http_without_api_key() {
    let server = spawn_echo_server().await;

    {
        let client = McpClient::connect_http(&server.url, None).await.unwrap();
        let tools = client.list_tools().await.unwrap();
        assert!(tools.iter().any(|t| t.name.as_ref() == "echo"));

        let result = client
            .call_tool("echo", json!({"message": "world"}))
            .await
            .unwrap();
        assert_eq!(result, "echo: world");
    }

    let seen = server.seen.lock().unwrap().clone();
    assert_eq!(seen, None);

    server.stop().await;
}
