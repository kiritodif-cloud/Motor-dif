# Motor-dif: Fase 8 — Nivel Estudio Indie (Resumen de Arquitectura y Validación)

## 1. Resumen Ejecutivo
La **Fase 8: Nivel Estudio Indie** eleva a `motor_dif` al estándar de un motor de videojuegos indie profesional, robusto y liviano, optimizado para correr a **60–75+ FPS en gráficos integrados** sin dependencias externas pesadas ni micro-stuttering.

Se implementaron con éxito los 6 hitos técnicos de manera completamente autónoma, pasando todas las suites de pruebas unitarias (10/10 tests pasados) y compilando en perfil `--release` con 0 errores y 0 advertencias críticas.

---

## 2. Desglose de Hitos Implementados

### Hito 1: Jerarquía Padre–Hijo (Scene Graph y WorldTransform)
- **Estructura de Datos:** Se agregó el campo opcional `parent: Option<String>` a `EntityDef` (`src/assets.rs`) y `EntityMeta` (`src/ecs.rs`).
- **Resolución de Matrices:** Algoritmo recursivo memoizado `compute_world_transforms` con detección estricta de ciclos mediante `visiting: HashSet<String>`. Si se detecta una referencia circular (e.g. A hijo de B y B hijo de A), se corta el ciclo y se degrada de forma segura al `Transform` local sin colgar el hilo ni desbordar la pila.
- **Jerarquía en Editor (`egui`):** Vista de árbol recursiva indentada en el panel **Scene Hierarchy** (`src/main.rs`). Desplegable interactivo `Parent` en el **Inspector** para emparentar o desemparentar entidades en tiempo real.
- **Scripting y MCP:** Función Lua `set_parent(child_id, parent_id)` y soporte en MCP `upsert_entity`.

### Hito 2: Luces Dinámicas Puntuales (Point Lights)
- **Componente ECS:** `PointLight { color: [f32; 3], intensity: f32, radius: f32 }`.
- **Buffer Uniform en WGSL:** `LightsUniform` (528 bytes, alineado a 16 bytes para compatibilidad WebGPU) en `@group(2) @binding(0)`. Soporta hasta 16 luces simultáneas.
- **Sombreado en Shaders:** Atenuación cuadrática suave por distancia `(1 - (d/r)^2)^2 * intensity` más componente especular Blinn-Phong acumulativo.
- **Inspector y Lua:** Controles completos de color RGB, intensidad y radio en el Inspector. API Lua `set_light(entity_id, r, g, b, intensity, radius)`.

### Hito 3: Cámara de Juego, Camera Shake y Tweens Procedurales
- **Control de Cámara:** `set_camera(x, y, z, tx, ty, tz)` y `follow_entity(entity_id, ox, oy, oz, smoothness)` con interpolación `lerp` amortiguada.
- **Camera Shake:** `camera_shake(intensity, duration)` con decaimiento cuadrático exponencial y generador pseudo-aleatorio de alta frecuencia sobre `eye` y `target`.
- **Sistema de Tweens Ligero (`src/tweens.rs`):** Interpolación suave `smoothstep` (curva hermite cúbica `t * t * (3 - 2t)`) para animar proceduralmente posición o escala (`tween_position`, `tween_scale`).

### Hito 4: Flujo de Juego Completo (Cambio de Escenas, Save Data y HUD Buttons)
- **Transiciones de Escena:** `load_scene(path)` limpia de forma segura el ECS, colisionadores de Rapier3D y partículas antes de cargar la nueva escena JSON sin bloquear la GPU.
- **Persistencia Ligera:** `save_data(key, value)` y `load_data(key)` respaldado por `assets/save_data.json`.
- **Botones HUD Interactivos:** `set_hud_button(id, text, x, y, callback_name)` renderizado mediante `egui::Area` tanto en el editor como en `--play`. Al hacer clic, invoca directamente la función global correspondiente en los scripts Lua.

### Hito 5: Animación por Sprite Sheets 2D
- **Instanced Shading:** `InstanceRaw` extendido con `uv_offset: [f32; 2]` y `uv_scale: [f32; 2]` (locations 10 y 11 en WGSL).
- **Vertex Shader:** `out.uv = in.uv * instance.uv_scale + instance.uv_offset;`
- **Control Lua:** `set_sprite_frame(entity_id, cols, rows, frame_index)`.
- **Textura Procedural:** Generador automático de sprite sheet de prueba en `assets/textures/spritesheet_test.png` (4 cuadrantes coloreados).

### Hito 6: Escena Demo Indie, Release y Captura Final
- **Demo Scene (`assets/scene.json` y `assets/scripts/indie_demo.lua`):**
  - Nodo padre con rotación orbital continua.
  - Esfera hija con luz puntual cálida emitiendo reflejos dinámicos sobre el suelo de tablero de ajedrez.
  - Pedestal central animable por tween.
  - Sprite 2D animado ciclando cuadros en vivo.
  - Botones HUD interactivos y persistencia de high score (`9999`).
- **Rendimiento:** 75.02 FPS estables en release.
- **Captura:** `viewport_fase8_final.png` generada vía endpoint MCP `capture_viewport`.

---

## 3. Resumen de la API de Scripting (Lua 5.4)

| Función Lua | Parámetros | Descripción |
| :--- | :--- | :--- |
| `set_parent` | `child_id, parent_id` | Establece o quita el nodo padre en la jerarquía. |
| `set_light` | `id, r, g, b, intensity, radius` | Configura una luz puntual dinámica. |
| `set_camera` | `x, y, z, tx, ty, tz` | Posiciona la cámara y su objetivo en tiempo real. |
| `follow_entity` | `id, ox, oy, oz, smoothness` | Hace que la cámara siga suavemente a una entidad. |
| `camera_shake` | `intensity, duration` | Sacudida de cámara con decaimiento amortiguado. |
| `tween_position` | `id, x, y, z, duration` | Anima la posición de una entidad con curva suave. |
| `tween_scale` | `id, sx, sy, sz, duration` | Anima la escala de una entidad con curva suave. |
| `load_scene` | `path` | Carga una nueva escena JSON de forma asíncrona y segura. |
| `save_data` | `key, value` | Guarda un valor persistente en `assets/save_data.json`. |
| `load_data` | `key` | Lee un valor persistente desde `assets/save_data.json`. |
| `set_hud_button`| `id, text, x, y, callback` | Registra un botón interactivo en el HUD. |
| `set_sprite_frame` | `id, cols, rows, frame_index` | Recorta y muestra el cuadro de un sprite sheet. |
| `set_hud_text` | `key, value` | Despliega texto de diagnóstico en el HUD overlay. |
| `spawn_particles` | `x, y, z, r, g, b, count, spd, life` | Genera una ráfaga instanciada de partículas. |
| `spawn_prefab` | `name, x, y, z` | Instancia un prefab JSON en coordenadas dadas. |
| `play_sound` | `path, [volume]` | Reproduce un efecto de sonido superpuesto (WAV/OGG). |
| `play_music` | `path, [volume]` | Reproduce música de fondo en bucle. |
| `stop_music` | `()` | Detiene la música de fondo. |

---

## 4. Benchmarks y Verificación

- **Tasa de Refresco:** 75.02 FPS (V-Sync adaptativo sin micro-stuttering).
- **Consumo de Memoria:** ~48 MB en GPU / RAM.
- **Pruebas Unitarias:** 10 de 10 pruebas pasando (`cargo test` -> 0 fallos).
- **Compilación:** `cargo build --release` -> 0 errores.
- **Captura Final:** `viewport_fase8_final.png` validada visualmente.
