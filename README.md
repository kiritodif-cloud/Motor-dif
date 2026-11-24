# Motor-dif

**Motor-dif** es un motor de videojuegos 2D/3D ligero y **nativo para IA** escrito en **Rust**, diseñado para crear, iterar y exportar juegos completos conversando en lenguaje natural con agentes de IA a través de **MCP (Model Context Protocol)**, sin sacrificar rendimiento en gráficas integradas.

[![Rust](https://img.shields.io/badge/Rust-Stable-orange?logo=rust)](https://www.rust-lang.org/)
[![wgpu](https://img.shields.io/badge/Graphics-wgpu-blue)](https://wgpu.rs/)
[![MCP](https://img.shields.io/badge/AI--Native-MCP%20Server-8A2BE2)](https://modelcontextprotocol.io/)
[![Performance](https://img.shields.io/badge/Release-~75%20FPS%20(iGPU)-brightgreen)]()

---

## ¿Por qué Motor-dif?

La mayoría de los motores tradicionales fueron diseñados para flujos manuales complejos, mientras que los prototipos web generados por IA (HTML/Three.js) chocan rápido con límites de físicas, guardado de escenas, editor visual y empaquetado.

**Motor-dif** combina lo mejor de ambos mundos:
- **Núcleo nativo en Rust (`wgpu` + `Rapier3D` + `mlua` + `egui`):** Compila a un único ejecutable ligero y corre fluido (~75 FPS en modo *Release* con sombras `2048x2048` y HDR en gráficas integradas).
- **Servidor MCP integrado (`http://127.0.0.1:3030/mcp`):** Permite que cualquier agente (Antigravity, Claude Desktop, OpenCode, etc.) inspeccione la documentación del motor, construya mundos, capture el visor en vivo, lea errores de consola y pruebe el juego en tiempo real.
- **Forges nativas en Rust:** En lugar de obligar a la IA a ensamblar miles de primitivas sueltas o reescribir controladores desde cero, Motor-dif expone generadores nativos de alto rendimiento para terrenos, mundos voxel, música y controladores de jugador.

---

## Características Principales

### Renderizado Moderno y Ligero (`wgpu`)
- **Shadow Mapping Direccional (2048x2048):** Pase *depth-only* centrado dinámicamente en la cámara con filtro **PCF 3x3** y *slope-scaled bias*.
- **Post-Procesado HDR con ACES Saturado:** Tone Mapping fílmico diseñado para preservar el color de materiales emisivos (neones, cristales, lava) sin quemarlos a blanco.
- **Iluminación Ambiental Hemisférica y Ciclo Día/Noche:** Controlable en una línea desde Lua (`set_time_of_day(hour)`), ajustando sol/luna, cielo, niebla y dirección de sombras automáticamente.
- **Transparencia y Detalle:** Soporte de *Alpha Cutout* (con sombras recortadas para follaje), mezclado translúcido (agua/vidrio), *Vertex Ambient Occlusion*, muestreo *Nearest-Neighbor* para pixel-art y micro-ruido procedural en WGSL.
- **View-Model en 1ª Persona:** Renderizado sin *clipping* contra paredes con animaciones procedurales (`swing`, `recoil`, `bob`).

### Forges Nativas para Agentes de IA
- **`Voxel Forge`:** Motor de chunks en Rust con *Hidden-Face Culling*, atlas UV procedural con texturas por cara, *Vertex AO* horneado, colisiones rápidas (`voxel_is_solid`), raycast DDA exacto (`voxel_raycast`) y persistencia en sidecar `.voxels.json`.
- **`Level Forge`:** Generación de terrenos procedurales exportados a `.glb` con persistencia automática de mapa de alturas (`.heightmap.json` + reconstrucción *fallback* desde `.glb`) y dispersión de props estilizados (`basalt_pillar`, `jagged_rock`, `crystal_cluster`, `sci_fi_crate`).
- **Controladores Listos (`forge_spawn_controller`):** Inyección en 1 llamada de jugadores probados para `fps`, `third_person`, `top_down` y `vehicle`.
- **`Music Forge`:** Síntesis procedural de audio y música ambiental integrada.

### Editor, Scripting Lua y Distribución
- **Editor Visual (`egui`):** Jerarquía de escena, Inspector, Consola con filtros, Navegador de Assets, Editor de Scripts y botón **`Connect MCP`** en 1 clic.
- **Play Mode Limpio:** *Pointer Lock* nativo (`Esc` para liberar), restricción del cursor al visor y ocultamiento automático de rejilla, gizmos, *colliders* y *toasts* del editor.
- **Hot-Reload en Vivo:** Edita scripts `.lua` o escenas `.json` y observa los cambios al instante.
- **UI y Persistencia en Lua:** Interfaz con íconos, recortes de atlas y slots de inventario (`ui_image`, `ui_slot`), más sistema seguro de partidas guardadas (`save_game` / `load_game`).
- **Exportador en 1 Clic (`Export Game`):** Recolecta únicamente los assets usados por la escena y empaqueta un ejecutable independiente con `game_config.json` listo para distribuir.

---

## Inicio Rápido

### Requisitos
- [Rust](https://www.rust-lang.org/tools/install) (toolchain estable)
- GPU compatible con Vulkan, DirectX 12 o Metal (funciona en gráficas integradas)

### Compilar y Ejecutar
```bash
# Clonar el repositorio
git clone https://github.com/tu-usuario/Motor-dif.git
cd Motor-dif

# Ejecutar pruebas unitarias
cargo test --bin motor_dif

# Compilar en modo Release (recomendado para máximo rendimiento)
cargo build --release

# Iniciar el editor
./target/release/motor_dif
```

---

## Conectar con un Agente de IA (MCP)

1. Abre **Motor-dif**. El servidor MCP se iniciará automáticamente en `http://127.0.0.1:3030/mcp`.
2. Haz clic en **`Connect MCP`** en la barra del editor para copiar la configuración JSON a tu cliente compatible (Antigravity, Claude Desktop, OpenCode, etc.):

```json
{
  "mcpServers": {
    "motor-dif": {
      "url": "http://127.0.0.1:3030/mcp"
    }
  }
}
```

3. Pídele al agente que cree un juego en lenguaje natural, por ejemplo:
   > *"Crea un mundo estilo Minecraft con árboles, agua, ciclo día/noche y una barra de 9 bloques donde pueda picar y construir en primera persona usando Motor-dif."*

El agente consultará `get_engine_docs`, generará el mundo con `Voxel Forge`, inyectará el controlador FPS, cargará la escena, entrará en Modo Play y verificará visualmente el resultado con `capture_viewport`.

---

## Exportar tu Juego

Desde el editor ve a **`File -> Export Game...`** (o usa la herramienta MCP `export_game`):
- Elige el nombre del juego y si arrancará en pantalla completa.
- Motor-dif creará una carpeta limpia en `exports/<tu-juego>/` con el ejecutable renombrado, `game_config.json` y **únicamente** los modelos, texturas, scripts y audios utilizados por tu proyecto.

---

## Estructura del Proyecto

```text
Motor-dif/
├── src/
│   ├── main.rs            # Bucle principal, editor egui, Pointer Lock y modo standalone
│   ├── renderer.rs        # Pipeline wgpu, Shadow Mapping PCF, ACES, Skybox y View-Model
│   ├── scripting.rs       # Runtime Lua (mlua), API de juego, UI y save/load
│   ├── mcp_server.rs      # Servidor HTTP MCP, herramientas y errores auto-correctivos
│   ├── voxel_forge.rs     # Chunks nativos, Hidden-Face Culling, Vertex AO y DDA raycast
│   ├── level_forge.rs     # Terreno procedural, props low-poly y sidecars .heightmap.json
│   ├── physics.rs         # Integración de físicas Rapier3D
│   └── exporter.rs        # Empaquetador standalone y recolector de dependencias
├── assets/                # Escenas, scripts Lua, modelos .glb, texturas y audio
├── AGENT_GUIDE.md         # Guía compacta embebida para agentes MCP
└── Cargo.toml
```

---

## Licencia

Distribuido bajo licencia MIT. Consulta el archivo `LICENSE` para más información.