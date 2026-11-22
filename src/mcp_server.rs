use std::net::SocketAddr;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::assets::{CameraDef, EntityDef};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInspection {
    pub camera: CameraDef,
    pub entities: Vec<EntityDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineLogsResponse {
    pub fps: f32,
    pub logs: Vec<String>,
    pub lua_errors: Option<String>,
}

pub enum McpCommand {
    InspectWorld {
        respond_to: oneshot::Sender<WorldInspection>,
    },
    UpsertEntity {
        entity: EntityDef,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    GetEngineLogs {
        respond_to: oneshot::Sender<EngineLogsResponse>,
    },
    CaptureViewport {
        respond_to: oneshot::Sender<Result<String, String>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

pub async fn handle_jsonrpc(
    cmd_tx: &mpsc::Sender<McpCommand>,
    req: JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    let id = req.id.clone();

    match req.method.as_str() {
        "initialize" => {
            Some(JsonRpcResponse::success(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "motor-dif-mcp",
                        "version": "0.1.0"
                    }
                }),
            ))
        }
        "notifications/initialized" => None,
        "ping" => Some(JsonRpcResponse::success(id, json!({}))),
        "tools/list" => {
            Some(JsonRpcResponse::success(
                id,
                json!({
                    "tools": [
                        {
                            "name": "inspect_world",
                            "description": "Devuelve la camara y todas las entidades actuales del mundo en formato JSON",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "upsert_entity",
                            "description": "Crea o modifica una entidad 2D/3D en tiempo real en la escena activa",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string", "description": "ID unico de la entidad" },
                                    "mesh": { "type": "string", "description": "Tipo de malla ('cube', 'quad' o ruta a archivo glb)" },
                                    "position": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 3,
                                        "maxItems": 3,
                                        "description": "Coordenadas [x, y, z]"
                                    },
                                    "rotation": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 3,
                                        "maxItems": 3,
                                        "description": "Angulos de Euler en radianes [rx, ry, rz]"
                                    },
                                    "scale": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 3,
                                        "maxItems": 3,
                                        "description": "Escala [sx, sy, sz]"
                                    },
                                    "color": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 4,
                                        "maxItems": 4,
                                        "description": "Color RGBA normalizado [r, g, b, a]"
                                    }
                                },
                                "required": ["id", "mesh", "position", "color"]
                            }
                        },
                        {
                            "name": "get_engine_logs",
                            "description": "Devuelve los ultimos logs de ejecucion, posibles errores de scripts y FPS del motor",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "capture_viewport",
                            "description": "Captura un fotograma del viewport actual del motor y devuelve la imagen en formato nativo PNG Base64",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        }
                    ]
                }),
            ))
        }
        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            match tool_name {
                "inspect_world" => {
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::InspectWorld { respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(inspection) => {
                            let text = serde_json::to_string_pretty(&inspection).unwrap_or_default();
                            Some(JsonRpcResponse::success(
                                id,
                                json!({
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": text
                                        }
                                    ]
                                }),
                            ))
                        }
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Fallo al inspeccionar mundo")),
                    }
                }
                "upsert_entity" => {
                    let entity_def: Result<EntityDef, _> = serde_json::from_value(args);
                    match entity_def {
                        Ok(entity) => {
                            let (tx, rx) = oneshot::channel();
                            if cmd_tx.send(McpCommand::UpsertEntity { entity, respond_to: tx }).await.is_err() {
                                return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                            }
                            match rx.await {
                                Ok(Ok(msg)) => Some(JsonRpcResponse::success(
                                    id,
                                    json!({
                                        "content": [
                                            {
                                                "type": "text",
                                                "text": msg
                                            }
                                        ]
                                    }),
                                )),
                                Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                                Err(_) => Some(JsonRpcResponse::error(id, -32000, "Fallo al procesar entidad")),
                            }
                        }
                        Err(err) => Some(JsonRpcResponse::error(id, -32602, format!("Argumentos invalidos: {}", err))),
                    }
                }
                "get_engine_logs" => {
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::GetEngineLogs { respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(logs_resp) => {
                            let text = serde_json::to_string_pretty(&logs_resp).unwrap_or_default();
                            Some(JsonRpcResponse::success(
                                id,
                                json!({
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": text
                                        }
                                    ]
                                }),
                            ))
                        }
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Fallo al obtener logs")),
                    }
                }
                "capture_viewport" => {
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::CaptureViewport { respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(base64_png)) => {
                            // NATIVE MCP IMAGE FORMAT: type: "image", data: base64, mimeType: "image/png"
                            Some(JsonRpcResponse::success(
                                id,
                                json!({
                                    "content": [
                                        {
                                            "type": "image",
                                            "data": base64_png,
                                            "mimeType": "image/png"
                                        }
                                    ]
                                }),
                            ))
                        }
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, format!("Error capturando viewport: {}", err))),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout o canal cerrado al capturar viewport")),
                    }
                }
                _ => Some(JsonRpcResponse::error(id, -32601, format!("Herramienta desconocida: {}", tool_name))),
            }
        }
        _ => Some(JsonRpcResponse::error(id, -32601, format!("Metodo desconocido: {}", req.method))),
    }
}

pub struct McpServerHandle {
    pub bound_addr: SocketAddr,
    pub _rt: Arc<tokio::runtime::Runtime>,
}

pub fn start_mcp_server(cmd_tx: mpsc::Sender<McpCommand>, port: u16) -> McpServerHandle {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let std_listener = match std::net::TcpListener::bind(addr) {
        Ok(l) => l,
        Err(_) => std::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .expect("Failed to bind TCP port for MCP server"),
    };
    std_listener
        .set_nonblocking(true)
        .expect("Failed to set nonblocking on MCP TCP listener");
    let bound_addr = std_listener.local_addr().expect("Failed to get local addr");

    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime for MCP server"),
    );

    let _guard = rt.enter();

    // Spawn async tasks on Tokio runtime
    let cmd_tx_http = cmd_tx.clone();
    rt.spawn(async move {
        let listener = match TcpListener::from_std(std_listener) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to convert std TcpListener to tokio: {}", e);
                return;
            }
        };
        let listener_arc = Arc::new(listener);

        loop {
            if let Ok((mut socket, _)) = listener_arc.accept().await {
                let cmd_tx_conn = cmd_tx_http.clone();
                tokio::spawn(async move {
                    let mut buf = Vec::new();
                    let mut temp = [0u8; 1024];
                    let mut header_end = None;

                    loop {
                        let n = match socket.read(&mut temp).await {
                            Ok(n) if n > 0 => n,
                            _ => break,
                        };
                        buf.extend_from_slice(&temp[..n]);
                        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_end = Some(pos);
                            break;
                        }
                    }

                    let header_pos = match header_end {
                        Some(pos) => pos,
                        None => return,
                    };

                    let headers_str = String::from_utf8_lossy(&buf[..header_pos]).to_string();

                    // Handle CORS pre-flight
                    if headers_str.starts_with("OPTIONS") {
                        let resp = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n";
                        let _ = socket.write_all(resp.as_bytes()).await;
                        return;
                    }

                    let mut content_length: usize = 0;
                    for line in headers_str.lines() {
                        if line.to_ascii_lowercase().starts_with("content-length:") {
                            if let Some(val) = line.split(':').nth(1) {
                                content_length = val.trim().parse::<usize>().unwrap_or(0);
                            }
                        }
                    }

                    let body_start = header_pos + 4;
                    let mut body_bytes = buf[body_start..].to_vec();
                    while body_bytes.len() < content_length {
                        let n = match socket.read(&mut temp).await {
                            Ok(n) if n > 0 => n,
                            _ => break,
                        };
                        body_bytes.extend_from_slice(&temp[..n]);
                    }

                    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

                    if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(body_str.trim()) {
                        let maybe_resp = handle_jsonrpc(&cmd_tx_conn, req).await;
                        if let Some(resp) = maybe_resp {
                            let resp_json = serde_json::to_string(&resp).unwrap_or_default();
                            let http_resp = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
                                resp_json.len(),
                                resp_json
                            );
                            let _ = socket.write_all(http_resp.as_bytes()).await;
                        }
                    } else {
                        let err_resp = json!({
                            "jsonrpc": "2.0",
                            "error": { "code": -32700, "message": "Parse error: expected valid JSON-RPC 2.0" }
                        });
                        let resp_json = err_resp.to_string();
                        let http_resp = format!(
                            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
                            resp_json.len(),
                            resp_json
                        );
                        let _ = socket.write_all(http_resp.as_bytes()).await;
                    }
                });
            }
        }
    });

    // Also spawn stdio listener in background
    let cmd_tx_stdio = cmd_tx.clone();
    rt.spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('{') {
                continue;
            }
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                if let Some(resp) = handle_jsonrpc(&cmd_tx_stdio, req).await {
                    if let Ok(resp_json) = serde_json::to_string(&resp) {
                        let mut stdout = tokio::io::stdout();
                        let _ = stdout.write_all(resp_json.as_bytes()).await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                    }
                }
            }
        }
    });

    McpServerHandle {
        bound_addr,
        _rt: rt,
    }
}
