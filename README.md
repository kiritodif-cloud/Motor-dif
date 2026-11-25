# Motor-dif v0.3.0

**Motor-dif** es un motor de videojuegos 2D/3D ligero y **nativo para IA** escrito en **Rust**, diseñado para crear, iterar y exportar juegos completos conversando en lenguaje natural con agentes de IA a través de **MCP (Model Context Protocol)**, sin sacrificar rendimiento en gráficas integradas.

[![Rust](https://img.shields.io/badge/Rust-Stable-orange?logo=rust)](https://www.rust-lang.org/)
[![wgpu](https://img.shields.io/badge/Graphics-wgpu-blue)](https://wgpu.rs/)
[![MCP](https://img.shields.io/badge/AI--Native-MCP%20Server-8A2BE2)](https://modelcontextprotocol.io/)
[![Performance](https://img.shields.io/badge/Release-~75%20FPS%20(iGPU)-brightgreen)]()
[![CI/CD](https://img.shields.io/badge/CI%2FCD-GitHub%20Actions-blue?logo=github)](https://github.com)

---

## ¿Por qué Motor-dif?

La mayoría de los motores tradicionales fueron diseñados para flujos manuales complejos, mientras que los prototipos web generados por IA (HTML/Three.js) chocan rápido con límites de físicas, guardado de escenas, editor visual y empaquetado.

**Motor-dif** combina lo mejor de ambos mundos:
- **Núcleo nativo en Rust (`wgpu` + `Rapier3D` + `mlua` + `egui`):** Compila a un único ejecutable ligero y corre fluido (> 60-75 FPS en modo *Release* con sombras `2048x2048`, PBR, 16 luces y HDR en gráficas integradas).
- **Servidor MCP integrado (`http://127.0.0.1:3030/mcp`):** Permite que cualquier agente (Antigravity, Claude Desktop, Cursor, OpenCode, etc.) inspeccione la documentación del motor, construya mundos, capture el visor en vivo, lea errores de consola y pruebe el juego en tiempo real con autenticación opcional por token (`--mcp-token`).
- **Forges nativas en Rust:** Generadores procedurales integrados para modelos, texturas, mapas de altura, mundos voxel, audio adaptativo, sistemas de partículas y controladores de jugador.

---

## Novedades Principales en Motor-dif v0.3.0

### 1. Render-To-Texture (RTT) y Cámaras Secundarias
- **Arquitectura de Pases RTT:** Registro de texturas de renderizado (`RenderTargetRegistry`) y planificador round-robin con presupuesto por frame (`RttScheduler`) para mantener > 60 FPS en tarjetas integradas (iGPU).
- **Proyección 3D y 2D:**
  - Cámaras de seguridad CCTV, retrovisores y pantallas in-game sobre mallas 3D sin E/S de disco (`set_material_rtt`).
  - Elementos de UI nativos (`UiElement::RttView` / `ui_draw_rtt`) para minimapas, visores tácticos y picture-in-picture en HUD.
- **Herramienta MCP `configure_rtt`:** Configuración remota de objetivos de textura y asignación a entidades de cámara y pantalla.

### 2. Netcode UDP Nativo con Renet y 3 Canales
- **Protocolo de Transporte UDP:** Handshake y enrutamiento con Renet sobre 3 canales configurados:
  - Canal 0 (`Unreliable`): Sincronización de alta frecuencia de transformaciones y snapshots de mundo.
  - Canal 1 (`ReliableOrdered`): Spawns, eventos críticos de gameplay y chat.
  - Canal 2 (`ReliableUnordered`): Confirmaciones discretas y pings.
- **Replicación Automática ECS:** Componentes `NetworkId` y `NetworkSync` para empaquetado de snapshots binarios y sincronización configurable en hercios (`rate_hz`).
- **Interpolación Suave en Cliente:** Amortiguación LERP para traslación y SLERP para rotación de cuaterniones, eliminando jitter en la simulación.
- **API Completa en Lua:** `net_host(port)`, `net_connect(ip_port)`, `net_disconnect()`, `net_is_server()`, `net_is_client()`, `net_client_id()`, `net_send(ch, msg)`, `net_broadcast(ch, msg)` y callbacks `on_net_connect`, `on_net_disconnect`, `on_net_message`.

### 3. Modo Servidor Headless y CI-Friendly
- **Servidor Dedicado en 1 Comando:** Ejecución sin GPU ni ventana mediante CLI:
  ```bash
  # Servidor dedicado en puerto 5000
  motor_dif --server 5000
  # Cliente auto-conectándose al servidor
  motor_dif --connect 127.0.0.1:5000
  ```
- **Compatibilidad 100% Headless:** Todos los tests unitarios desacoplados de dispositivos de audio o GPU física, garantizando ejecución limpia en CI/CD de GitHub Actions.

### 4. Herramientas MCP de Red, Profiler F3 y Fallback de Assets
- **Herramienta MCP `net_status`:** Retorna el rol activo, cantidad de clientes/peers conectados, latencia RTT promedio (ms), rendimiento TX/RX en bytes/s y total de entidades replicadas.
- **Métricas F3 en Tiempo Real:** Filas dedicadas en el overlay del Profiler para pases RTT y telemetría de Red (`Net: [Role] | Peers: [N] | RTT: [ms] | TX/RX: [KB/s]`).
- **Resolución Inteligente de Assets:** Búsqueda recursiva con fallback automático a `../../assets` cuando el ejecutable se ejecuta desde subcarpetas (ej. `dist/`).

### 5. Pipeline PBR Completo, Mipmaps y Hot-Reload GLB
- **Materiales PBR & `uv_scale`:** Serialización completa de `pbr_material`, `uv_scale` y `spot_light` en escenas/prefabs, Inspector, Lua (`set_uv_scale`, `set_spot_light`) y MCP (`upsert_entity`).
- **Mipmaps CPU & Muestreo Anisotrópico:** Generación automática de cadena de mipmaps en CPU y filtrado anisotrópico `16x` para texturas PBR, además de extracción/desduplicación de texturas embebidas en `.glb`/`.gltf` y hot-reload de modelos 3D.

### 6. Fase 42: IBL Procedural, Frustum Culling, Físicas Avanzadas, Skinning y REPL
- **Renderizado:** Mapas `normal` y `metallic_roughness` en formato lineal `Rgba8Unorm`, IBL procedural de cielo para reflejos metálicos y **Frustum Culling en CPU**.
- **Físicas y Animación:** Colliders `heightfield` y `trimesh`, `Joints` (`fixed`, `revolute`, `spherical` + `create_joint` en Lua), **Skinning esquelético** (`compute_skinned_vertices`) y `AnimStateMachine`.
- **Editor y Red:** Gizmos visuales para `PointLight`/`SpotLight`, **Lua REPL** integrado en Consola, Drag & Drop de assets al Viewport, `GamepadBackend` seguro para CI y serialización de RPCs en Renet.

---

## Novedades de Producción en Motor-dif v0.2.0

### 1. Acabado Visual y Gráficos Modernos (`wgpu`)
- **Pipeline PBR Completo:** Modelo de microfacetas Cook-Torrance GGX con mapas de normales (*tangent-space*), rugosidad, metalicidad y emisión.
- **Sombras Direccionales PCF 3x3:** Sombra suave gaussianizada de 2048x2048 con *slope-scaled bias* y profundidad centrada en cámara.
- **16 Luces Dinámicas:** Iluminación por puntos y focos (*point & spot lights*) con cálculo de atenuación cuadrática en GPU.
- **Niebla Atmosférica:** Modos lineal y exponencial integrados armoniosamente con el ciclo día/noche y el color de horizonte.
- **Post-Procesado HDR con ACES Saturado:** Mapeo tonal cinematográfico con bloom, viñeta y aberración cromática configurable en caliente.
- **View-Model en 1ª Persona:** Renderizado de armas y herramientas en primer plano sin *clipping* contra paredes, con animaciones procedurales (`swing`, `recoil`, `bob`).

### 2. Animación Esquelética y Audio Espacial 3D
- **Skeletal Animation Player:** Reproducción de clips de animación glTF con interpolación esférica de cuaterniones (slerp) y mezcla por desvanecimiento cruzado (*crossfade blending*).
- **Audio Espacial 3D:** Atenuación paramétrica con curvas lineal, exponencial y smoothstep, más paneo estéreo posicional relativo a la orientación de la cámara.
- **Buses de Audio:** Control independiente de volúmenes para `Master`, `SFX` y `Music`.

### 3. Navegación, IA y Pathfinding 3D
- **A\* 3D Consciente de Terreno y Vóxeles:** Búsqueda heurística euclidiana que escala pendientes y sortea obstáculos con presupuesto de pasos por fotograma (*step budget*).
- **Steering Behaviors:** Comportamiento `steer_towards` con suavizado de giro en yaw y llegada automática.
- **Líneas de Depuración 3D:** Renderizado inmediato de rutas (`draw_path`), líneas (`draw_line_3d`) y anillos (`draw_ring_3d`) en el visor.

### 4. Partículas GPU y Game Feel
- **Emisores de Partículas Instanciadas:** Simulación ultraligera en CPU/GPU con presets de producción (`explosion`, `fire`, `smoke`, `sparks`, `rain`, `magic_portal`).
- **Game Feel Instantáneo:** Escala de tiempo dinámico (`set_time_scale`), micro-congelamiento de impacto (`hitstop`) y sacudida de cámara con frecuencia y armónicos (`camera_shake`).

### 5. Físicas de Producción con Rapier3D
- **Kinematic Character Controller:** Desplazamiento cinemático robusto con subida de escalones (0.4m), límite de pendiente (45°), fijación al suelo (0.2m) y tiempo coyote (0.15s).
- **Capas y Máscaras de Colisión (`CollisionLayer`):** Filtrado por máscaras de bits (`DEFAULT`, `PLAYER`, `ENEMY`, `WORLD`, `PROJECTILE`, `TRIGGER`).
- **Consultas Espaciales:** Detección de impacto por raycast (`raycast`) y comprobación de volumen esférico (`overlap_sphere`).
- **Callbacks de Eventos:** Detección en scripts de `on_collision(self, other_id, nx, ny, nz)`, `on_trigger_enter` y `on_trigger_exit`.

### 6. Mapeo Profesional de Input y UI Responsiva
- **Sistema de Input Basado en Acciones:** Soporte para teclado, ratón y mandos/gamepad con zona muerta configurable, triggers analógicos y persistencia en `assets/input_config.json`.
- **UI Adaptable con 9 Anclajes:** Posicionamiento responsivo de paneles, barras de vida, etiquetas, botones, imágenes y ranuras de inventario de 720p a 4K.
- **Motor de Tweens en Lua:** Interpolaciones visuales fluidas (`tween_ui_property`) con curvas de easing (`linear`, `ease_in_quad`, `ease_out_quad`, `ease_in_out_cubic`, `bounce_out`).

### 7. Herramientas de Producción en Editor y Profiler
- **Sistema de Prefabs (`.prefab.json`):** Jerarquías completas de entidades con hijos y componentes, instanciables desde el Editor, Lua (`spawn_prefab`) y MCP (`spawn_prefab`).
- **Historial Deshacer/Rehacer (`Ctrl+Z` / `Ctrl+Y`):** Pila de estados al transformar entidades con gizmos en el visor.
- **Snapping en Cuadrícula (`🧲 Snap`):** Ajuste magnético de posición (0.25m - 2.0m) y rotación (15° - 90°).
- **Profiler en Tiempo Real (`F3`):** Gráfica sparkline de 120 frames con líneas de referencia de 60 y 30 FPS, desglose de tiempos CPU/GPU (Simulación, Render, UI), recuento de entidades, draw calls, partículas y cálculo de memoria VRAM estimada.
- **Hot-Reload Total sin Reinicios:** Detección en tiempo real con `notify` de scripts Lua, texturas `.png`, prefabs `.prefab.json` y shaders `.wgsl`.

### 8. Distribución y CI/CD Multiplataforma
- **Exportación en 1 Clic (`File -> Export Game...`):** Cosechado inteligente de dependencias para crear un paquete limpio y ejecutable independiente con `game_config.json`.
- **GitHub Actions (`.github/workflows/release.yml`):** Compilación automática, ejecución de tests y empaquetado de ejecutables para Windows (zip) y Linux (tar.gz).

---

## Inicio Rápido

### Requisitos
- [Rust](https://www.rust-lang.org/tools/install) (toolchain estable)
- GPU compatible con Vulkan, DirectX 12 o Metal (totalmente compatible con gráficas integradas Intel/AMD)

### Compilar y Ejecutar
```bash
# Clonar el repositorio
git clone https://github.com/tu-usuario/Motor-dif.git
cd Motor-dif

# Ejecutar pruebas unitarias (155 tests pasando al 100%)
cargo test --bin motor_dif

# Compilar en modo Release optimizado
cargo build --release

# Iniciar el editor
./target/release/motor_dif
```

---

## Conectar con un Agente de IA (MCP)

1. Abre **Motor-dif**. El servidor MCP se iniciará en `http://127.0.0.1:3030/mcp`.
2. Haz clic en **`Connect MCP`** en la barra superior del editor para copiar la configuración a tu cliente de IA (Antigravity, Claude Desktop, Cursor, etc.):

```json
{
  "mcpServers": {
    "motor-dif": {
      "url": "http://127.0.0.1:3030/mcp"
    }
  }
}
```

Si ejecutas el motor con token de seguridad:
```bash
./target/release/motor_dif --mcp-token MI_TOKEN_SECRETO
```
Agrega la cabecera correspondiente:
```json
{
  "mcpServers": {
    "motor-dif": {
      "url": "http://127.0.0.1:3030/mcp",
      "headers": {
        "Authorization": "Bearer MI_TOKEN_SECRETO"
      }
    }
  }
}
```

---

## Atajos de Teclado del Editor

| Atajo | Acción |
| :--- | :--- |
| `F3` | Abrir/Cerrar Profiler en tiempo real (Sparkline, desglose de tiempos y memoria) |
| `F11` | Alternar Pantalla Completa |
| `W` / `E` / `R` | Gizmo de Traslación / Rotación / Escala |
| `Ctrl+Z` | Deshacer última transformación |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Rehacer transformación |
| `Ctrl+S` | Guardar escena actual |
| `Ctrl+D` | Duplicar entidad seleccionada |
| `Supr` | Eliminar entidad seleccionada |
| `Esc` | Liberar Pointer Lock en Modo Play |

---

## Estructura del Proyecto

```text
Motor-dif/
├── .github/workflows/
│   └── release.yml        # CI/CD multiplataforma (Windows & Linux) y empaquetado de releases
├── src/
│   ├── main.rs            # Bucle principal, editor egui, F3 Profiler, Hot-Reload y Standalone
│   ├── renderer.rs        # Pipeline wgpu, PBR GGX, normal maps, soft shadows PCF 3x3, 16 luces y ACES
│   ├── scripting.rs       # Runtime Lua (mlua), bindings de ECS, UI, físicas, audio, AI y tweens
│   ├── mcp_server.rs      # Servidor HTTP MCP, esquemas auto-correctivos y autenticación Bearer
│   ├── physics.rs         # Rapier3D: KinematicCharacterController, capas de colisión y spatial queries
│   ├── animation.rs       # Reproductor glTF con blending y presets procedurales
│   ├── audio.rs           # Motor de audio espacial 3D con curvas de atenuación y buses Master/SFX/Music
│   ├── nav.rs             # A* 3D de terreno y vóxeles con step budget y comportamientos de dirección
│   ├── particles.rs       # Sistema de partículas GPU instanciadas con presets de producción
│   ├── input_map.rs       # Mapeo de acciones, soporte para mandos/gamepad y persistencia
│   ├── game_ui.rs         # Renderizador declarativo con 9 anclajes responsivos y atlas de sprites
│   ├── tweens.rs          # Gestor de tweens con curvas de aceleración y amortiguación
│   ├── voxel_forge.rs     # Motor de chunks voxel, Hidden-Face Culling, Vertex AO y DDA raycast
│   ├── level_forge.rs     # Terrenos procedurales, dispersión estilizada y mapas de altura
│   └── exporter.rs        # Empaquetador independiente en 1 clic y cosechador de dependencias
├── assets/                # Escenas, prefabs, scripts Lua, modelos .glb, texturas y sonido
├── AGENT_GUIDE.md         # Guía de referencia ultracompacta para agentes MCP (< 240 líneas)
└── Cargo.toml
```

---

## Licencia

Distribuido bajo licencia MIT. Consulta el archivo `LICENSE` para más información.
