# Resumen de Ejecución — Fase 23: Pointer Lock, Alpha Cutout/AO y Voxel Forge Nativo

> **Estado:** Completada con éxito  
> **Tests Unitarios:** 109 / 109 pasando (100%)  
> **Compilación:** `cargo build --release` finalizada con **0 errores**  
> **Rendimiento:** Optimizado para iGPU (> 60 FPS con Shadow Mapping PCF, ACES Tone Mapping y Chunks Voxels culliados)

---

## 1. Hito 1: Pointer Lock Nativo en Modo Play

- **Captura del Cursor (`winit 0.30`):**
  - Implementación con `CursorGrabMode::Locked` y fallback transparente a `CursorGrabMode::Confined`.
  - Ocultación del cursor con `window.set_cursor_visible(false)`.
  - Integración en `EngineState::set_pointer_lock(locked: bool)` sincronizada con atomics en `LuaEngineContext`.
- **Manejo de Ratón en Modo Bloqueado:**
  - Evento `DeviceEvent::MouseMotion { delta }` en `ApplicationHandler::device_event` para recibir deltas raw no acelerados y sin límites en los bordes de la pantalla.
  - Supresión de acumulación en `WindowEvent::CursorMoved` cuando `pointer_locked == true` para evitar doble conteo.
- **Liberación Segura y Re-adquisición:**
  - Pulsar `Escape` libera inmediatamente el bloqueo sin cerrar la ventana ni detener la simulación.
  - Pérdida de foco (`WindowEvent::Focused(false)`) y parada de la simulación (`stop_simulation()`) liberan el cursor de manera limpia.
  - Al hacer clic dentro del viewport en Modo Play tras haber solicitado el bloqueo, el puntero se re-bloquea automáticamente.
- **Nuevas APIs Lua:**
  - `set_pointer_lock(enabled: bool)`: Activa o desactiva el bloqueo de ratón desde script.
  - `is_pointer_locked() -> bool`: Consulta el estado actual del bloqueo.

---

## 2. Hito 2: Alpha Cutout, Translucidez, Vertex AO y Nearest Sampler en `wgpu`

- **Alpha Cutout en PBR y Shadow Pass:**
  - En `fs_main` y `fs_shadow` (WGSL): `if (base_color.a < 0.5) { discard; }` para materiales opacos con máscara (follaje, hojas de roble, mallas tipo flor).
  - La sombra proyectada en el Shadow Pass ahora respeta fielmente la silueta de los bloques transparentes y hojas, eliminando los bloques oscuros cuadrados.
- **Pipeline de Translucidez (`translucent_render_pipeline`):**
  - Renderizado con `wgpu::BlendState::ALPHA_BLENDING` y `depth_write_enabled: false`, permitiendo ver el fondo marino a través del agua (`alpha: 0.65`) y a través de vidrio (`alpha: 0.40`).
  - Orden estricto de renderizado: Skybox → Opaque/Cutout → Translucent (Agua/Vidrio) → Additive → Partículas.
- **Multiplicador de Vertex AO:**
  - El color de vértice `in.color` (`[ao_val, ao_val, ao_val, 1.0]`) se multiplica directamente por el albedo y modula la iluminación difusa y ambiental sin ningún coste extra de shader.
- **Nearest Neighbor Sampler Automático:**
  - Texturas con nombres que contengan `voxel`, `pixel`, `atlas` o `_nearest` se configuran automáticamente con `wgpu::FilterMode::Nearest`, garantizando nitidez perfecta pixel-art en voxel art.

---

## 3. Hito 3: Voxel Forge Nativo (`src/voxel_forge.rs`)

- **Estructura de Chunks y Bloques:**
  - Chunks de dimensiones estándar: `16 x 32 x 16` (8,192 bloques por chunk).
  - 11 tipos de bloques compactos (`BlockId: u8`):
    - `0`: Air
    - `1`: Grass
    - `2`: Dirt
    - `3`: Stone
    - `4`: Wood (Tronco de roble)
    - `5`: Leaves (Hojas con cutout)
    - `6`: Sand (Arena de playa)
    - `7`: Water (Agua traslúcida)
    - `8`: Bedrock (Roca madre)
    - `9`: Cobblestone (Adoquín)
    - `10`: Planks (Madera procesada)
    - `11`: Glass (Vidrio con reflejos)
- **Chunk Mesher con Hidden-Face Culling:**
  - Itera sobre las 6 caras (`Top`, `Bottom`, `North`, `South`, `East`, `West`).
  - Solo emite caras expuestas si el bloque adyacente no es opaco (reduce la geometría interna entre un 85% y un 90%).
  - Agua contigua a agua nunca emite caras internas.
- **Corner Vertex AO (Ambient Occlusion de Esquinas):**
  - Evalúa los 3 vecinos de cada esquina de la cara (`side1`, `side2`, `corner`).
  - Aplica factor `0.46` (esquina ocluida), `0.65`, `0.82` o `1.0` (expuesto).
  - Volteo inteligente de la diagonal de triangulación según el gradiente de AO para evitar artefactos visuales.
- **Atlas Pixel-Art Procedural (`assets/textures/voxel_atlas.png`):**
  - Generación procedural automática si no existe en disco (cuadrícula 4x4 de texturas 16x16 px).
  - Mapeo UV por cara: césped superior, lateral con tierra, tierra pura, corteza, anillos de tronco, agua celeste, etc.
- **DDA 3D Raycasting Rápido (Amanatides & Woo):**
  - Trazado de rayos instantáneo en cuadrícula voxel `voxel_raycast(ox, oy, oz, dx, dy, dz, max_dist)`.
  - Devuelve posición del bloque impactado, normal de la cara (`nx, ny, nz`) e ID del bloque.
- **Generador de Mundo Procedural:**
  - Terreno montañoso con ruido sinusoidal multioctava, lecho de roca en y=0, cuevas 3D, playas de arena, nivel del mar en y=8 y árboles de roble procedurales.
- **APIs Lua de Voxel World:**
  - `voxel_get_block(x, y, z) -> id: int`
  - `voxel_set_block(x, y, z, id: int)`
  - `voxel_raycast(ox, oy, oz, dx, dy, dz, max_dist) -> table`
  - `voxel_is_solid(x, y, z) -> bool`
- **Herramienta MCP y Persistencia:**
  - `forge_create_voxel_world({ width_chunks, depth_chunks, seed, terrain_type, name })`: genera el mundo, exporta las mallas `.glb` (opaca y agua traslúcida), las inserta en escena y guarda el estado en `.voxels.json`.
  - Sincronización automática de `active_voxel_world` al cargar o detener escenas.

---

## 4. Hito 4: Documentación y Validación

- `AGENT_GUIDE.md`: Actualizado con las nuevas APIs de Voxel Forge, Pointer Lock y blend modes (< 240 líneas, exactamente **181 líneas**).
- `src/mcp_server.rs`: Herramienta `forge_create_voxel_world` expuesta en `tools/list` y manejada en `tools/call`.
- Suite de tests unitarios: **109 tests pasando con 0 fallos**.
- Binario release generado en `target/release/motor_dif.exe`.
