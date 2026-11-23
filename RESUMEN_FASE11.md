# RESUMEN FASE 11: PRODUCCIÓN Y ECOSISTEMA (MOTOR-DIF)

## 1. Visión General y Logros Clave
La **Fase 11** transforma a **Motor-dif** en una plataforma completa de producción para estudios indie y creadores. Ahora el motor no solo permite crear y simular mundos 3D interactivos con scripts Lua y físicas Rapier3D, sino también:
1. **Exportar juegos standalone en 1 clic** (`dist/<game_name>/`) listos para ser distribuidos con arranque automático (`game.json`).
2. **Construir interfaces de usuario declarativas avanzadas desde Lua** (`game_ui.rs`) con anclajes en pantalla, paneles, barras de progreso y botones interactivos.
3. **Animar entidades por keyframes interpolados** (`animation.rs`) afectando posición, rotación, escala, color y emisión de luz/glow.
4. **Inspeccionar el rendimiento en tiempo real con el Profiler `F3`**, alternar modos de visualización (`Lit`, `Unlit`, `Wireframe`) y guardar/restaurar marcadores de cámara (`Ctrl+1..4` / `Alt+1..4`).
5. **Supervisar y controlar el motor mediante MCP v3** con telemetría en vivo (`get_performance_stats`, `get_scene_tree`, `get_console_logs`, `export_game`).

---

## 2. Nuevos Módulos y Arquitectura

### 2.1 Exportador de Juegos y Auto-Boot (`src/exporter.rs` + `src/main.rs`)
- **Exportación en 1 Clic:**
  - Función `crate::exporter::export_game(game_name, entry_scene, output_dir)` que clona recursivamente el directorio `assets/`, copia el binario compilado (`motor_dif.exe` -> `<game_name>.exe`) y genera el manifiesto `game.json`.
  - Soporte de exportación vía:
    - **Menú del Editor:** `File -> [Export] Export Game...` (abre ventana modal interactiva).
    - **Paleta de Comandos (`Ctrl+K`):** `Export Game (Standalone)`.
    - **Herramienta MCP:** `export_game`.
- **Detección Automática de Modo Standalone:**
  - Al ejecutar el binario sin argumentos, `check_auto_boot_game()` busca `game.json` en el directorio de trabajo actual o junto al ejecutable. Si existe, arranca inmediatamente en modo juego (`--play`) con el título y dimensiones especificadas, omitiendo la UI del editor y maximizando el rendimiento (75+ FPS).

### 2.2 Sistema de UI Declarativa desde Lua (`src/game_ui.rs` + `src/scripting.rs`)
- Elementos soportados:
  - `ui_panel(id, anchor, offset_x, offset_y, width, height, r, g, b, a)`
  - `ui_bar(id, anchor, offset_x, offset_y, width, height, progress, label, r, g, b)`
  - `ui_label(id, anchor, offset_x, offset_y, text, font_size, r, g, b)`
  - `ui_button(id, anchor, offset_x, offset_y, width, height, text, callback_function_name)`
  - `ui_clear()`
- Soporte de 7 puntos de anclaje de pantalla: `"top_left"`, `"top_center"`, `"top_right"`, `"center"`, `"bottom_left"`, `"bottom_center"`, `"bottom_right"`.
- Los botones interactivos enlazan directamente con funciones globales de Lua mediante el despachador de eventos `pending_button_clicks`.

### 2.3 Sistema de Animación por Keyframes (`src/animation.rs`)
- **Estructuras:**
  - `Keyframe`: define `time`, `position: Option<[f32; 3]>`, `rotation: Option<[f32; 3]>`, `scale: Option<[f32; 3]>`, `color: Option<[f32; 4]>`, `emissive: Option<f32>`.
  - `AnimationClip`: contenedor de keyframes ordenados temporalmente con muestreo lineal interpolado (`sample(time) -> InterpolatedState`).
  - `AnimationPlayer`: componente ECS con gestión de múltiples clips, velocidad configurable, modo bucle (`looping`), control de reproducción (`play`, `pause`, `stop`, `resume`) y captura de fotogramas (`capture_keyframe`).
- **Integración en Escenas JSON (`assets.rs`):**
  - Campo opcional `"animation_player"` en `EntityDef`, permitiendo cargar y guardar animaciones directamente en archivos de escena o prefabs.
- **Inspector de Animación:**
  - Tarjeta interactiva `🎬 Animation Player` con selector de clip, botones Play/Pause/Stop, slider de línea de tiempo con posición en segundos, checkbox de bucle, velocidad y botón `+ Capture Keyframe`.
- **Bindings Lua:**
  - `play_anim(entity_id, clip_name, [looping])`
  - `stop_anim(entity_id)`
  - `is_anim_playing(entity_id) -> bool`

### 2.4 Profiler Integrado (`F3`), Modos de Viewport y Marcadores de Cámara
- **Profiler `F3`:**
  - Panel superior derecho con gráfica de sparkline de los últimos 90 frames, líneas de referencia de 60 FPS (16.6ms) y 30 FPS (33.3ms), y telemetría de:
    - FPS y tiempo de cuadro en ms.
    - Draw calls / batches instanciados.
    - Entidades activas en el ECS.
    - Cuerpos rígidos (dinámicos + estáticos) en Rapier3D.
    - Partículas activas.
    - Luces puntuales activas.
- **Modos de Viewport:**
  - Selector desplegable en la cápsula superior del viewport:
    - `Lit`: sombreado completo con Blinn-Phong, Bloom HDR, luces puntuales, Fresnel y niebla.
    - `Unlit`: renderizado plano de color base y texturas (ideal para estilizado o depuración de albedo).
    - `Wireframe`: renderizado procedural de aristas poligonales sin sobrecosto de pipelines adicionales.
- **Marcadores Rápidos de Cámara:**
  - `Ctrl + 1..4`: Guarda la posición y objetivo actual de la cámara en el slot 1..4 con notificación Toast.
  - `Alt + 1..4`: Restaura instantáneamente la vista guardada en el slot.

### 2.5 Servidor MCP v3 (`src/mcp_server.rs`)
Herramientas ampliadas para agentes de IA y automatización externa:
- `get_performance_stats`: telemetría en tiempo real (`fps`, `frame_time_ms`, `entities`, `draw_calls`, `active_particles`, `physics_bodies`).
- `get_scene_tree`: árbol jerárquico recursivo de la escena con componentes presentes y entidad seleccionada.
- `get_console_logs`: logs del motor con filtro por tipo (`all`, `error`, `lua`, `mcp`) y límite.
- `export_game`: empaqueta un juego standalone a disco y devuelve la ruta generada.
- `capture_viewport`: captura el viewport en formato nativo PNG Base64.

---

## 3. Escena Showcase v11 (`assets/showcase_v11.json`)
- **Entidades:**
  - `arena_platform`: base fija con físicas Rapier3D.
  - `celestial_core`: núcleo central con `AnimationPlayer` activo (clip `"pulse_loop"`) que oscila verticalmente, rota sobre sus ejes y pulsa su emisión HDR.
  - `orbit_satellite`: satélite esférico hijo de `celestial_core` con luz puntual propia.
  - `halo_ring`: anillo toroidal con material aditivo y Fresnel.
  - `pillar_alpha` / `pillar_beta`: columnas laterales con iluminación puntual complementaria.
  - `showcase_controller`: entidad controladora con el script `showcase_v11.lua`.
- **Script Lua (`assets/scripts/showcase_v11.lua`):**
  - Demuestra UI declarativa con barra de energía animada en tiempo real.
  - Botón interactivo `SPAWN PARTICLES` con emisión sonora y partículas en 3D.
  - Botón interactivo `TOGGLE ANIMATION` que activa o detiene la animación del núcleo mediante `play_anim` / `stop_anim`.

---

## 4. Métricas de Verificación End-to-End
| Métrica | Valor Medido | Estado |
| :--- | :--- | :--- |
| **Pruebas Unitarias** | 27 / 27 pasadas (100%) | [OK] |
| **Tasa de Cuadros (Release)** | 75.16 FPS (13.3 ms/frame) | [OK] |
| **Draw Calls / Batches** | 6 batches instanciados | [OK] |
| **Respuesta Servidor MCP** | Puerto 3030 (`tools/list` 16 herramientas) | [OK] |
| **Exportación Standalone** | `dist/showcase_v11/showcase_v11/` verificado | [OK] |
| **Liberación de Puerto** | Puerto 3030 cerrado limpiamente tras prueba | [OK] |
| **Captura de Viewport** | `viewport_fase11_final.png` (450 KB) | [OK] |
