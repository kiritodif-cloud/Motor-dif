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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneTreeNode {
    pub id: String,
    pub mesh: String,
    pub parent: Option<String>,
    pub components: Vec<String>,
    pub children: Vec<SceneTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneTreeResponse {
    pub selected_entity: Option<String>,
    pub root_nodes: Vec<SceneTreeNode>,
    pub total_entities: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStatsResponse {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub entities: usize,
    pub draw_calls: usize,
    pub active_particles: usize,
    pub physics_bodies: usize,
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
    GetConsoleLogs {
        filter: Option<String>,
        limit: Option<usize>,
        respond_to: oneshot::Sender<Vec<String>>,
    },
    GetSceneTree {
        respond_to: oneshot::Sender<SceneTreeResponse>,
    },
    GetPerformanceStats {
        respond_to: oneshot::Sender<PerformanceStatsResponse>,
    },
    CaptureViewport {
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    DeleteEntity {
        id: String,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    SetCamera {
        mode: Option<String>,
        position: Option<[f32; 3]>,
        target: Option<[f32; 3]>,
        ortho_size: Option<f32>,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    SetPlayMode {
        mode: String,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    StepSimulation {
        dt: f32,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    RunLuaSnippet {
        code: String,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    SaveScene {
        path: Option<String>,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    SpawnPrefab {
        name: String,
        position: [f32; 3],
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    SelectEntity {
        id: String,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    ForgeCreateModel {
        recipe: crate::modeler::ModelRecipe,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeBakeGlb {
        recipe: crate::modeler::ModelRecipe,
        output_name: String,
        insert_in_scene: bool,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeCaptureTurnaround {
        angle: String,
        respond_to: oneshot::Sender<Result<String, String>>,
    },
    ForgeCreateTexture {
        style: String,
        filename: String,
        size: u32,
        seed: u32,
        color_tint: [f32; 4],
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeCreateMusic {
        preset: Option<String>,
        recipe: Option<crate::music_forge::MusicRecipe>,
        bake_path: Option<String>,
        play_now: bool,
        intensity: Option<f32>,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    SetMusicIntensity {
        intensity: f32,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeGenerateTerrain {
        recipe: crate::level_forge::TerrainRecipe,
        output_name: String,
        insert_in_scene: bool,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeGenerateDungeon {
        recipe: crate::level_forge::DungeonRecipe,
        insert_in_scene: bool,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeScatterProps {
        recipe: crate::level_forge::ScatterRecipe,
        terrain_recipe: Option<crate::level_forge::TerrainRecipe>,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeCreateRiggedCharacter {
        preset: String,
        id_prefix: String,
        position: [f32; 3],
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
    },
    ForgeSpawnVfx {
        preset: Option<String>,
        recipe: Option<crate::vfx_forge::VfxRecipe>,
        position: Option<[f32; 3]>,
        attach_to_entity: Option<String>,
        respond_to: oneshot::Sender<Result<serde_json::Value, String>>,
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

pub const ENGINE_DOCS: &str = include_str!("../AGENT_GUIDE.md");

pub fn extract_doc_topic(full_docs: &str, topic: &str) -> String {
    let t = topic.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("all") {
        return full_docs.to_string();
    }
    let header_lower = format!("## {}", t.to_ascii_lowercase());

    let mut found_start = None;
    for (byte_offset, _) in full_docs.match_indices("## ") {
        let line = &full_docs[byte_offset..];
        let line_end = line.find('\n').unwrap_or(line.len());
        let current_header = line[..line_end].trim();
        if current_header.to_ascii_lowercase() == header_lower {
            found_start = Some(byte_offset);
            break;
        }
    }

    if let Some(start) = found_start {
        let remainder = &full_docs[start..];
        let first_nl = remainder.find('\n').unwrap_or(remainder.len());
        let body = &remainder[first_nl..];
        if let Some(next_header) = body.find("\n## ") {
            let section_end = first_nl + next_header;
            remainder[..section_end].trim().to_string()
        } else {
            remainder.trim().to_string()
        }
    } else {
        format!(
            "Error: Topic '{}' not found in engine documentation. Available topics: quickstart, scene_json, lua_core, lua_ui, lua_vfx_audio, lua_ai_rig, mcp_workflow, all",
            t
        )
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
                            "name": "get_engine_docs",
                            "description": "CALL THIS FIRST before writing Lua scripts or scene JSONs. Returns compact, token-efficient API signatures and examples so you don't need to inspect engine source files.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "topic": {
                                        "type": "string",
                                        "enum": ["all", "quickstart", "scene_json", "lua_core", "lua_ui", "lua_vfx_audio", "lua_ai_rig", "mcp_workflow"],
                                        "description": "Topic section to retrieve ('all', 'quickstart', 'scene_json', 'lua_core', 'lua_ui', 'lua_vfx_audio', 'lua_ai_rig', 'mcp_workflow'). Default: 'all'"
                                    }
                                }
                            }
                        },
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
                        },
                        {
                            "name": "export_game",
                            "description": "Exporta el juego completo a un paquete independiente ejecutable en dist/",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "game_name": { "type": "string", "description": "Nombre del juego y del ejecutable generado" },
                                    "entry_scene": { "type": "string", "description": "Ruta de la escena inicial (ej. 'assets/scene.json')" },
                                    "output_dir": { "type": "string", "description": "Directorio de destino (opcional, por defecto 'dist/<game_name>')" }
                                },
                                "required": ["game_name", "entry_scene"]
                            }
                        },
                        {
                            "name": "get_console_logs",
                            "description": "Retorna los ultimos logs filtrados de la consola del motor",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "filter": { "type": "string", "enum": ["all", "error", "lua", "mcp"], "description": "Filtro de logs ('all', 'error', 'lua', 'mcp')" },
                                    "limit": { "type": "integer", "description": "Numero maximo de logs a retornar (por defecto 50)" }
                                }
                            }
                        },
                        {
                            "name": "get_scene_tree",
                            "description": "Retorna la jerarquia completa de la escena en formato arbol JSON y entidad seleccionada",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "get_performance_stats",
                            "description": "Retorna metricas en tiempo real: fps, frame_time_ms, entities, draw_calls, active_particles, physics_bodies",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "delete_entity",
                            "description": "Elimina una entidad del mundo ECS y sus fisicas en tiempo real",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string", "description": "ID de la entidad a eliminar" }
                                },
                                "required": ["id"]
                            }
                        },
                        {
                            "name": "set_camera",
                            "description": "Controla la camara del motor (modo 2D/3D, posicion, objetivo y tamano ortografico)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "mode": { "type": "string", "enum": ["3d", "2d"], "description": "Modo de camara ('3d' o '2d')" },
                                    "position": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 3,
                                        "maxItems": 3,
                                        "description": "Posicion [x, y, z]"
                                    },
                                    "target": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 3,
                                        "maxItems": 3,
                                        "description": "Punto objetivo [x, y, z]"
                                    },
                                    "ortho_size": { "type": "number", "description": "Tamano ortografico (modo 2D)" }
                                }
                            }
                        },
                        {
                            "name": "set_play_mode",
                            "description": "Inicia ('play'), pausa ('pause') o resetea/detiene ('stop') la simulacion",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "mode": { "type": "string", "enum": ["play", "pause", "stop"], "description": "Modo deseado" }
                                },
                                "required": ["mode"]
                            }
                        },
                        {
                            "name": "step_simulation",
                            "description": "Ejecuta un paso determinista del motor por el delta time especificado",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "dt": { "type": "number", "description": "Paso de tiempo en segundos (por defecto 0.01666)" }
                                }
                            }
                        },
                        {
                            "name": "run_lua_snippet",
                            "description": "Ejecuta un fragmento de codigo Lua en caliente sobre el motor y devuelve el resultado",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "code": { "type": "string", "description": "Codigo Lua a evaluar" }
                                },
                                "required": ["code"]
                            }
                        },
                        {
                            "name": "save_scene",
                            "description": "Persiste la escena actual a un archivo JSON en disco",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Ruta del archivo (por defecto 'assets/scene.json')" }
                                }
                            }
                        },
                        {
                            "name": "spawn_prefab",
                            "description": "Instancia un prefab desde assets/prefabs/<name>.json en la posicion indicada",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string", "description": "Nombre del prefab (ej. 'bouncing_orb')" },
                                    "x": { "type": "number", "description": "Posicion X" },
                                    "y": { "type": "number", "description": "Posicion Y" },
                                    "z": { "type": "number", "description": "Posicion Z" }
                                },
                                "required": ["name"]
                            }
                        },
                        {
                            "name": "select_entity",
                            "description": "Selecciona una entidad por ID en el editor activo para inspeccionar y mostrar sus gizmos",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string", "description": "ID de la entidad a seleccionar" }
                                },
                                "required": ["id"]
                            }
                        },
                        {
                            "name": "new_scene_from_template",
                            "description": "Genera una nueva escena a partir de una plantilla lista para usar ('empty_3d', 'top_down_2d', 'platformer_3d', 'space_arena') y la guarda en disco",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "template_name": {
                                        "type": "string",
                                        "enum": ["empty_3d", "top_down_2d", "platformer_3d", "space_arena"],
                                        "description": "Nombre de la plantilla ('empty_3d', 'top_down_2d', 'platformer_3d', 'space_arena')"
                                    },
                                    "save_path": {
                                        "type": "string",
                                        "description": "Ruta de guardado (por defecto 'assets/scene.json')"
                                    }
                                },
                                "required": ["template_name"]
                            }
                        },
                        {
                            "name": "forge_create_model",
                            "description": "Crea o actualiza un modelo procedural 3D en el Asset Forge a partir de una receta parametrica y/o SDF",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "recipe": {
                                        "type": "object",
                                        "description": "Receta con nombre, partes, nodos SDF y modificadores globales"
                                    }
                                },
                                "required": ["recipe"]
                            }
                        },
                        {
                            "name": "forge_bake_glb",
                            "description": "Hornea una receta de Asset Forge a un archivo binario glTF 2.0 (.glb) con Vertex Colors e inserta opcionalmente en la escena",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "recipe": {
                                        "type": "object",
                                        "description": "Receta del modelo (opcional si se usa la receta activa)"
                                    },
                                    "output_name": {
                                        "type": "string",
                                        "description": "Nombre del archivo a generar en assets/models/<output_name>.glb"
                                    },
                                    "insert_in_scene": {
                                        "type": "boolean",
                                        "description": "Si es true, instancia el modelo horneado como entidad en la escena activa"
                                    }
                                },
                                "required": ["output_name"]
                            }
                        },
                        {
                            "name": "forge_capture_turnaround",
                            "description": "Posiciona la camara en angulos ortogonales/isometricos ('front', 'side', 'iso', 'top') y captura el viewport en PNG Base64",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "angle": {
                                        "type": "string",
                                        "enum": ["front", "side", "iso", "top"],
                                        "description": "Angulo de camara ('front', 'side', 'iso', 'top')"
                                    }
                                },
                                "required": ["angle"]
                            }
                        },
                        {
                            "name": "forge_create_texture",
                            "description": "Genera una textura procedural PNG en assets/textures/ ('planet', 'scifi_panel', 'plasma', 'grid_neon', 'rock') y la carga en el motor",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "style": {
                                        "type": "string",
                                        "enum": ["planet", "scifi_panel", "plasma", "grid_neon", "rock"],
                                        "description": "Estilo procedural de la textura ('planet', 'scifi_panel', 'plasma', 'grid_neon', 'rock')"
                                    },
                                    "filename": {
                                        "type": "string",
                                        "description": "Nombre del archivo a guardar (ej. 'lava_world' o 'plasma_glow')"
                                    },
                                    "size": {
                                        "type": "integer",
                                        "description": "Tamano de la textura en pixeles (cuadrado, ej. 256 o 512, por defecto 256)"
                                    },
                                    "seed": {
                                        "type": "integer",
                                        "description": "Semilla determinista del generador de ruido (por defecto 42)"
                                    },
                                    "color_rgba": {
                                        "type": "array",
                                        "items": { "type": "number" },
                                        "minItems": 4,
                                        "maxItems": 4,
                                        "description": "Tinte de color base [r, g, b, a] normalizado 0..1"
                                    }
                                },
                                "required": ["style", "filename"]
                            }
                        },
                        {
                            "name": "forge_create_music",
                            "description": "Compone musica procedural con Music Forge a partir de un preset o receta JSON, con opcion de hornear a .wav o reproducir en vivo con musica adaptativa",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "preset": { "type": "string", "description": "Nombre de preset: 'space_ambient', 'boss_battle', 'cyber_action', 'menu_chill'" },
                                    "recipe": { "type": "object", "description": "Objeto MusicRecipe con bpm, scale, chord_progression y tracks" },
                                    "bake_path": { "type": "string", "description": "Ruta opcional para guardar .wav, ej. 'assets/music/battle.wav'" },
                                    "play_now": { "type": "boolean", "description": "Si true, reproduce la musica adaptativa en el bus de musica de inmediato" },
                                    "intensity": { "type": "number", "description": "Nivel de intensidad inicial (0.0 a 1.0)" }
                                }
                            }
                        },
                        {
                            "name": "set_music_intensity",
                            "description": "Ajusta la intensidad adaptativa de la musica procedural en reproduccion (0.0 ambiente tranquilo a 1.0 batalla completa)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "intensity": { "type": "number", "description": "Nivel de intensidad de 0.0 a 1.0" }
                                },
                                "required": ["intensity"]
                            }
                        },
                        {
                            "name": "forge_generate_terrain",
                            "description": "Genera terreno procedural 3D con altura por FBM, colores por altura/pendiente y hornea a GLB, opcionalmente instanciandolo en escena con collider trimesh estatico",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "recipe": { "type": "object", "description": "TerrainRecipe con width, depth, resolution, max_height, noise_scale, octaves, seed, island_falloff, flat_shading, palette" },
                                    "output_name": { "type": "string", "description": "Nombre de archivo GLB destino (ej. 'terrain_demo')" },
                                    "insert_in_scene": { "type": "boolean", "description": "Si true, instancia el terreno en la escena actual con collider trimesh estatico" }
                                },
                                "required": ["recipe", "output_name"]
                            }
                        },
                        {
                            "name": "forge_generate_dungeon",
                            "description": "Genera mazmorra procedural con salas y pasillos ortogonales conectados, puntos de spawn, boss y loot, e inserta suelo y muros con colliders estaticos",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "recipe": { "type": "object", "description": "DungeonRecipe con room_count, min_room_size, max_room_size, dungeon_size, wall_height, seed" },
                                    "insert_in_scene": { "type": "boolean", "description": "Si true, instancia suelo y muros de la mazmorra en la escena actual" }
                                },
                                "required": ["recipe"]
                            }
                        },
                        {
                            "name": "forge_scatter_props",
                            "description": "Calcula distribucion procedural de props en terreno por Poisson-disc sampling respetando pendientes y limites de altura, e inserta las entidades en escena",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "recipe": { "type": "object", "description": "ScatterRecipe con prop_mesh, count, min_spacing, min_height, max_height, max_slope_deg, scale_min, scale_max, random_yaw, seed" },
                                    "terrain_recipe": { "type": "object", "description": "TerrainRecipe opcional para muestreo de alturas y pendientes (si se omite usa el terreno activo)" }
                                },
                                "required": ["recipe"]
                            }
                        },
                        {
                            "name": "forge_create_rigged_character",
                            "description": "Construye e inserta un personaje o mecha modular articulado con Procedural Rig (presets: mech_biped, sci_fi_trooper, spider_drone, defense_turret)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "preset": { "type": "string", "enum": ["mech_biped", "sci_fi_trooper", "spider_drone", "defense_turret"], "description": "Tipo de personaje modular" },
                                    "id_prefix": { "type": "string", "description": "Prefijo para los IDs de las entidades hijas generadas" },
                                    "position": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "Posicion [x, y, z] del personaje en el mundo" }
                                },
                                "required": ["preset"]
                            }
                        },
                        {
                            "name": "forge_spawn_vfx",
                            "description": "Dispara un efecto visual procedural o adjunta un emisor continuo (VFX) a una entidad (presets: explosion, fire, magic_portal, hit_spark, level_up)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "preset": { "type": "string", "enum": ["explosion", "fire", "magic_portal", "hit_spark", "level_up"], "description": "Nombre de preset VFX" },
                                    "recipe": { "type": "object", "description": "Receta VFX procedural personalizada (VfxRecipe)" },
                                    "position": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3, "description": "Posicion [x, y, z] de disparo en el mundo" },
                                    "attach_to_entity": { "type": "string", "description": "ID de la entidad a la que adjuntar el emisor persistente" }
                                }
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
                "get_engine_docs" => {
                    let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("all");
                    let text = extract_doc_topic(ENGINE_DOCS, topic);
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
                "export_game" => {
                    let game_name = args.get("game_name").and_then(|v| v.as_str()).unwrap_or("MotorGame");
                    let entry_scene = args.get("entry_scene").and_then(|v| v.as_str()).unwrap_or("assets/scene.json");
                    let output_dir = args.get("output_dir").and_then(|v| v.as_str());

                    match crate::exporter::export_game(game_name, entry_scene, output_dir) {
                        Ok(out_path) => {
                            Some(JsonRpcResponse::success(
                                id,
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": format!("Juego exportado exitosamente en '{}'", out_path.display())
                                    }],
                                    "dist_path": out_path.to_string_lossy(),
                                    "status": "success"
                                }),
                            ))
                        }
                        Err(e) => Some(JsonRpcResponse::error(id, -32000, format!("Error al exportar: {}", e))),
                    }
                }
                "get_console_logs" => {
                    let filter = args.get("filter").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let limit = args.get("limit").and_then(|v| v.as_u64()).map(|u| u as usize);
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::GetConsoleLogs { filter, limit, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(logs) => {
                            let text = serde_json::to_string_pretty(&logs).unwrap_or_default();
                            Some(JsonRpcResponse::success(
                                id,
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": text
                                    }],
                                    "logs": logs
                                }),
                            ))
                        }
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Fallo al obtener logs")),
                    }
                }
                "get_scene_tree" => {
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::GetSceneTree { respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(tree) => {
                            let text = serde_json::to_string_pretty(&tree).unwrap_or_default();
                            Some(JsonRpcResponse::success(
                                id,
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": text
                                    }],
                                    "scene_tree": tree
                                }),
                            ))
                        }
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Fallo al obtener arbol de escena")),
                    }
                }
                "get_performance_stats" => {
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::GetPerformanceStats { respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(stats) => {
                            let text = serde_json::to_string_pretty(&stats).unwrap_or_default();
                            Some(JsonRpcResponse::success(
                                id,
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": text
                                    }],
                                    "performance_stats": stats
                                }),
                            ))
                        }
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Fallo al obtener estadisticas de rendimiento")),
                    }
                }
                "delete_entity" => {
                    let entity_id = match args.get("id").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'id'")),
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::DeleteEntity { id: entity_id, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(msg)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": msg }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al eliminar entidad")),
                    }
                }
                "set_camera" => {
                    let mode = args.get("mode").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let position = args.get("position").and_then(|v| v.as_array()).and_then(|arr| {
                        if arr.len() == 3 {
                            Some([
                                arr[0].as_f64()? as f32,
                                arr[1].as_f64()? as f32,
                                arr[2].as_f64()? as f32,
                            ])
                        } else {
                            None
                        }
                    });
                    let target = args.get("target").and_then(|v| v.as_array()).and_then(|arr| {
                        if arr.len() == 3 {
                            Some([
                                arr[0].as_f64()? as f32,
                                arr[1].as_f64()? as f32,
                                arr[2].as_f64()? as f32,
                            ])
                        } else {
                            None
                        }
                    });
                    let ortho_size = args.get("ortho_size").and_then(|v| v.as_f64()).map(|s| s as f32);

                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::SetCamera { mode, position, target, ortho_size, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(msg)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": msg }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al configurar camara")),
                    }
                }
                "set_play_mode" => {
                    let mode = match args.get("mode").and_then(|v| v.as_str()) {
                        Some(m) => m.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'mode'")),
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::SetPlayMode { mode, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(msg)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": msg }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al cambiar modo de simulacion")),
                    }
                }
                "step_simulation" => {
                    let dt = args.get("dt").and_then(|v| v.as_f64()).unwrap_or(1.0 / 60.0) as f32;
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::StepSimulation { dt, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(msg)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": msg }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al avanzar simulacion")),
                    }
                }
                "run_lua_snippet" => {
                    let code = match args.get("code").and_then(|v| v.as_str()) {
                        Some(c) => c.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'code'")),
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::RunLuaSnippet { code, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(output)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": output }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al ejecutar fragmento Lua")),
                    }
                }
                "save_scene" => {
                    let path = args.get("path").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::SaveScene { path, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(msg)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": msg }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al guardar escena")),
                    }
                }
                "spawn_prefab" => {
                    let name = match args.get("name").and_then(|v| v.as_str()) {
                        Some(n) => n.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'name'")),
                    };
                    let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let z = args.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::SpawnPrefab { name, position: [x, y, z], respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(msg)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": msg }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al instanciar prefab")),
                    }
                }
                "select_entity" => {
                    let id_str = match args.get("id").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'id'")),
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::SelectEntity { id: id_str, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(msg)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": msg }] }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al seleccionar entidad")),
                    }
                }
                "new_scene_from_template" => {
                    let template_name = match args.get("template_name").and_then(|v| v.as_str()) {
                        Some(t) => t,
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'template_name'")),
                    };
                    let save_path = args.get("save_path").and_then(|v| v.as_str()).unwrap_or("assets/scene.json");
                    match crate::templates::generate_and_save_template(template_name, save_path) {
                        Ok(scene) => Some(JsonRpcResponse::success(
                            id,
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("Plantilla '{}' guardada exitosamente en '{}' con {} entidades", template_name, save_path, scene.entities.len())
                                }],
                                "template": template_name,
                                "save_path": save_path,
                                "entities_count": scene.entities.len()
                            }),
                        )),
                        Err(e) => Some(JsonRpcResponse::error(id, -32000, format!("Error generando plantilla: {}", e))),
                    }
                }
                "forge_create_model" => {
                    let recipe_val = match args.get("recipe") {
                        Some(v) => v.clone(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'recipe'")),
                    };
                    let recipe: crate::modeler::ModelRecipe = match serde_json::from_value(recipe_val) {
                        Ok(r) => r,
                        Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Formato de receta invalido: {}", e))),
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeCreateModel { recipe, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al crear modelo en Forge")),
                    }
                }
                "forge_bake_glb" => {
                    let output_name = match args.get("output_name").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'output_name'")),
                    };
                    let recipe = if let Some(rv) = args.get("recipe") {
                        match serde_json::from_value::<crate::modeler::ModelRecipe>(rv.clone()) {
                            Ok(r) => r,
                            Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Formato de receta invalido: {}", e))),
                        }
                    } else {
                        crate::modeler::preset_spaceship()
                    };
                    let insert_in_scene = args.get("insert_in_scene").and_then(|v| v.as_bool()).unwrap_or(false);
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeBakeGlb { recipe, output_name, insert_in_scene, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al hornear modelo GLB")),
                    }
                }
                "forge_capture_turnaround" => {
                    let angle = args.get("angle").and_then(|v| v.as_str()).unwrap_or("iso").to_string();
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeCaptureTurnaround { angle, respond_to: tx }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(base64_png)) => {
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
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, format!("Error capturando turnaround: {}", err))),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al capturar turnaround")),
                    }
                }
                "forge_create_texture" => {
                    let style = match args.get("style").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'style'")),
                    };
                    let filename = match args.get("filename").and_then(|v| v.as_str()) {
                        Some(f) => f.to_string(),
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'filename'")),
                    };
                    let size = args.get("size").and_then(|v| v.as_u64()).unwrap_or(256) as u32;
                    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42) as u32;
                    let color_tint = if let Some(arr) = args.get("color_rgba").and_then(|v| v.as_array()) {
                        if arr.len() == 4 {
                            [
                                arr[0].as_f64().unwrap_or(0.3) as f32,
                                arr[1].as_f64().unwrap_or(0.7) as f32,
                                arr[2].as_f64().unwrap_or(0.9) as f32,
                                arr[3].as_f64().unwrap_or(1.0) as f32,
                            ]
                        } else {
                            [0.3, 0.7, 0.9, 1.0]
                        }
                    } else {
                        [0.3, 0.7, 0.9, 1.0]
                    };

                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeCreateTexture {
                        style,
                        filename,
                        size,
                        seed,
                        color_tint,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al crear textura en Forge")),
                    }
                }
                "forge_create_music" => {
                    let preset = args.get("preset").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let recipe = if let Some(rv) = args.get("recipe") {
                        match serde_json::from_value::<crate::music_forge::MusicRecipe>(rv.clone()) {
                            Ok(r) => Some(r),
                            Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Error en recipe: {}", e))),
                        }
                    } else {
                        None
                    };
                    let bake_path = args.get("bake_path").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let play_now = args.get("play_now").and_then(|v| v.as_bool()).unwrap_or(false);
                    let intensity = args.get("intensity").and_then(|v| v.as_f64()).map(|f| f as f32);

                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeCreateMusic {
                        preset,
                        recipe,
                        bake_path,
                        play_now,
                        intensity,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al crear musica en Music Forge")),
                    }
                }
                "set_music_intensity" => {
                    let intensity = match args.get("intensity").and_then(|v| v.as_f64()) {
                        Some(f) => f as f32,
                        None => return Some(JsonRpcResponse::error(id, -32602, "Falta el parametro requerido 'intensity' (numero)")),
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::SetMusicIntensity {
                        intensity,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al ajustar intensidad de musica")),
                    }
                }
                "forge_generate_terrain" => {
                    let recipe = match args.get("recipe") {
                        Some(v) => match serde_json::from_value::<crate::level_forge::TerrainRecipe>(v.clone()) {
                            Ok(r) => r,
                            Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Error en recipe: {}", e))),
                        },
                        None => crate::level_forge::TerrainRecipe::default(),
                    };
                    let output_name = match args.get("output_name").and_then(|v| v.as_str()) {
                        Some(s) => s.to_string(),
                        None => "terrain_demo".to_string(),
                    };
                    let insert_in_scene = args.get("insert_in_scene").and_then(|v| v.as_bool()).unwrap_or(true);
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeGenerateTerrain {
                        recipe,
                        output_name,
                        insert_in_scene,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al generar terreno en Level Forge")),
                    }
                }
                "forge_generate_dungeon" => {
                    let recipe = match args.get("recipe") {
                        Some(v) => match serde_json::from_value::<crate::level_forge::DungeonRecipe>(v.clone()) {
                            Ok(r) => r,
                            Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Error en recipe: {}", e))),
                        },
                        None => crate::level_forge::DungeonRecipe::default(),
                    };
                    let insert_in_scene = args.get("insert_in_scene").and_then(|v| v.as_bool()).unwrap_or(true);
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeGenerateDungeon {
                        recipe,
                        insert_in_scene,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al generar mazmorra en Level Forge")),
                    }
                }
                "forge_scatter_props" => {
                    let recipe = match args.get("recipe") {
                        Some(v) => match serde_json::from_value::<crate::level_forge::ScatterRecipe>(v.clone()) {
                            Ok(r) => r,
                            Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Error en recipe: {}", e))),
                        },
                        None => crate::level_forge::ScatterRecipe::default(),
                    };
                    let terrain_recipe = if let Some(tv) = args.get("terrain_recipe") {
                        match serde_json::from_value::<crate::level_forge::TerrainRecipe>(tv.clone()) {
                            Ok(tr) => Some(tr),
                            Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Error en terrain_recipe: {}", e))),
                        }
                    } else {
                        None
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeScatterProps {
                        recipe,
                        terrain_recipe,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al dispersar props en Level Forge")),
                    }
                }
                "forge_create_rigged_character" => {
                    let preset = args.get("preset").and_then(|v| v.as_str()).unwrap_or("mech_biped").to_string();
                    let id_prefix = args.get("id_prefix").and_then(|v| v.as_str()).unwrap_or("char").to_string();
                    let position = if let Some(arr) = args.get("position").and_then(|v| v.as_array()) {
                        [
                            arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        ]
                    } else {
                        [0.0, 0.0, 0.0]
                    };
                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeCreateRiggedCharacter {
                        preset,
                        id_prefix,
                        position,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al crear personaje riggeado")),
                    }
                }
                "forge_spawn_vfx" => {
                    let preset = args.get("preset").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let recipe = if let Some(rv) = args.get("recipe") {
                        match serde_json::from_value::<crate::vfx_forge::VfxRecipe>(rv.clone()) {
                            Ok(r) => Some(r),
                            Err(e) => return Some(JsonRpcResponse::error(id, -32602, format!("Error en recipe: {}", e))),
                        }
                    } else {
                        None
                    };
                    let position = args.get("position").and_then(|v| v.as_array()).map(|arr| [
                        arr.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    ]);
                    let attach_to_entity = args.get("attach_to_entity").and_then(|v| v.as_str()).map(|s| s.to_string());

                    let (tx, rx) = oneshot::channel();
                    if cmd_tx.send(McpCommand::ForgeSpawnVfx {
                        preset,
                        recipe,
                        position,
                        attach_to_entity,
                        respond_to: tx,
                    }).await.is_err() {
                        return Some(JsonRpcResponse::error(id, -32000, "Motor no disponible"));
                    }
                    match rx.await {
                        Ok(Ok(val)) => Some(JsonRpcResponse::success(id, json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_default() }], "data": val }))),
                        Ok(Err(err)) => Some(JsonRpcResponse::error(id, -32000, err)),
                        Err(_) => Some(JsonRpcResponse::error(id, -32000, "Timeout al disparar VFX")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_v3_structs_serialization() {
        let stats = PerformanceStatsResponse {
            fps: 60.5,
            frame_time_ms: 16.52,
            entities: 42,
            draw_calls: 3,
            active_particles: 150,
            physics_bodies: 12,
        };
        let json_str = serde_json::to_string(&stats).unwrap();
        assert!(json_str.contains("\"fps\":60.5"));
        assert!(json_str.contains("\"draw_calls\":3"));

        let child = SceneTreeNode {
            id: "child_sword".to_string(),
            mesh: "cube".to_string(),
            parent: Some("player".to_string()),
            components: vec!["Transform".to_string(), "MeshRender".to_string()],
            children: Vec::new(),
        };

        let root = SceneTreeNode {
            id: "player".to_string(),
            mesh: "sphere".to_string(),
            parent: None,
            components: vec!["Transform".to_string(), "MeshRender".to_string(), "ScriptComponent".to_string()],
            children: vec![child],
        };

        let tree = SceneTreeResponse {
            selected_entity: Some("player".to_string()),
            root_nodes: vec![root],
            total_entities: 2,
        };

        let tree_json = serde_json::to_string_pretty(&tree).unwrap();
        assert!(tree_json.contains("\"selected_entity\": \"player\""));
        assert!(tree_json.contains("\"child_sword\""));
    }

    #[test]
    fn test_mcp_forge_tools_schema_and_commands() {
        let recipe = crate::modeler::preset_spaceship();
        let recipe_json = serde_json::to_value(&recipe).unwrap();
        assert_eq!(recipe.name, "spaceship");
        assert!(recipe_json.get("parts").is_some());

        // Test tools/list contains forge tools
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/list".to_string(),
            params: None,
        };
        let (cmd_tx, _cmd_rx) = mpsc::channel(1);
        let resp = tokio::runtime::Runtime::new().unwrap().block_on(async {
            handle_jsonrpc(&cmd_tx, req).await
        }).unwrap();
        let resp_json = serde_json::to_value(&resp).unwrap();
        let resp_str = resp_json.to_string();
        assert!(resp_str.contains("forge_create_model"));
        assert!(resp_str.contains("forge_bake_glb"));
        assert!(resp_str.contains("forge_capture_turnaround"));
        assert!(resp_str.contains("forge_create_music"));
        assert!(resp_str.contains("set_music_intensity"));
        assert!(resp_str.contains("forge_generate_terrain"));
        assert!(resp_str.contains("forge_generate_dungeon"));
        assert!(resp_str.contains("forge_scatter_props"));
        assert!(resp_str.contains("forge_create_rigged_character"));
        assert!(resp_str.contains("forge_spawn_vfx"));
    }
}
