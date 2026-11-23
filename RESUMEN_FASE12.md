# RESUMEN FASE 12: ARQUITECTURA DE GAMEPLAY PRO

**Motor-dif v0.1.0 (Phase 12: Professional Gameplay Architecture)**
Fecha de completitud: 23 de Noviembre de 2026

---

## 1. Resumen Ejecutivo

En la **Fase 12**, **Motor-dif** ha dado el salto definitivo hacia una arquitectura de gameplay completa y lista para producción indie, incorporando sistemas nativos en Rust y enlaces de alto nivel en Lua para:
1. **Plantillas de Escena Listas para Usar y Fix del Exportador:** Creación de 4 plantillas completas (`empty_3d`, `top_down_2d`, `platformer_3d`, `space_arena`), selector en menú `File -> New from Template` e integración en el servidor MCP (`new_scene_from_template`). Corrección del exportador standalone para evitar la creación de subcarpetas redundantes (`dist/juego/juego` -> `dist/juego`).
2. **Bus de Eventos Desacoplado (Signals) y Timers Nativos:** Comunicación reactiva entre scripts y sistemas mediante señales (`emit_event`, `on_event`, `clear_events`) y gestión determinista de temporizadores (`after`, `every`, `cancel_timer`) con ciclo de vida automático.
3. **Mapeo de Acciones Lógicas y Proyección 3D de Ratón:** Abstracción de entradas (`is_action_down`, `is_action_just_pressed`, `bind_action`) persistible en `assets/input_map.json` y raycasting de cursor a planos horizontales (`get_mouse_world_pos(plane_y)`).
4. **Navegación e IA Ultra-Rápida en Rust (A\* Pathfinding):** Sistema de cuadrícula XZ (`NavGrid`) con heurística octil admisible, horneado dinámico de obstáculos desde la escena (`nav_bake`) y trazado de rutas óptimas (`find_path`).
5. **Audio Espacial 3D y Mezclador por Canales (Buses):** Control de volumen por canales (`master`, `sfx`, `music`), atenuación acústica lineal en espacio 3D (`play_sound_3d`), y soporte de buses en sonido y música.
6. **Showcase v12 y Verificación Standalone:** Demo interactiva en `assets/showcase_v12.json` y `assets/scripts/showcase_v12.lua` corriendo a 75 FPS estables en gráficos integrados, con captura de viewport `viewport_fase12_final.png` y exportación standalone verificada en `dist/showcase_v12`.

---

## 2. Detalle de Hitos Realizados

### Hito 1: Pulido de Fase 11 y Plantillas de Escena
- **Fix de carpeta en `src/exporter.rs`:** Resolución de ruta corregida para comparar el nombre final del directorio de salida con `clean_name`, asegurando que `output_dir = "dist"` genere `dist/nombre_juego/` directamente.
- **Tipografía e íconos en `src/main.rs`:** Reemplazo de glifo UTF-8 `🎬` por `[A] Animation Player` para evitar símbolos de glifo faltante `▯` en fuentes monospace de `egui`.
- **Módulo `src/templates.rs`:**
  - `create_template_scene(template_name)`: Soporte para 4 plantillas estructuradas con cámaras, iluminación, colisionadores y scripts iniciales:
    - `"empty_3d"`: Plano base neutral y cubo de referencia.
    - `"top_down_2d"`: Cámara ortográfica cenital con sprites y suelo.
    - `"platformer_3d"`: Plataformas escalonadas con suelo y obstáculos.
    - `"space_arena"`: Arena circular Dyson con cielo procedural `space` y núcleos luminosos.
  - `generate_and_save_template(template_name, save_path)`: Serialización y guardado a disco sin requerir contexto de ventana ni GPU.
- **Menú y MCP:**
  - Submenú `File -> New from Template -> [1] Empty 3D Scene | [2] Top-Down 2D | [3] 3D Platformer | [4] Space Arena`.
  - Herramienta MCP `new_scene_from_template` expuesta en `tools/list` y manejada en `tools/call`.

### Hito 2: Bus de Eventos (Signals) y Timers en Lua
- **Event Bus:**
  - `on_event(event_name, callback_name)`: Registro de callbacks oyentes por evento.
  - `emit_event(event_name, payload)`: Emisión de eventos con payloads complejos (tablas Lua, números, strings) convertidos bidireccionalmente a `serde_json::Value`.
  - `clear_events()`: Limpieza total de oyentes y eventos encolados.
- **Timers Gestionados:**
  - `after(delay_seconds, callback_name) -> timer_id`: Temporizador de disparo único.
  - `every(interval_seconds, callback_name) -> timer_id`: Temporizador periódico recurrente.
  - `cancel_timer(timer_id) -> bool`: Cancelación manual de timers activos.
  - `clear_timers()` y `clear_timers_and_events()`: Limpieza automática al detener la simulación o cambiar de escena.
  - Función de ciclo principal `process_timers_and_events(&mut world, &ctx, dt, &mut logs)` con vaciado de colas libre de interbloqueos (*deadlock-free*).

### Hito 3: Input Mapping y Mouse en Mundo 3D
- **Módulo `src/input_map.rs`:**
  - Estructura `InputMap` con diccionario de acciones a lista de teclas y botones de ratón.
  - Carga y persistencia automática en `assets/input_map.json`.
  - Mapeos por defecto listos para juego: `move_up` (W, ArrowUp), `move_down` (S, ArrowDown), `move_left` (A, ArrowLeft), `move_right` (D, ArrowRight), `jump` (Space), `fire` (MouseLeft, F, E), `alt_fire` (MouseRight, C), `interact` (E).
  - Detección precisa de flancos de subida (`is_action_just_pressed`) y estado sostenido (`is_action_down`).
- **Proyección de Ratón a Mundo 3D:**
  - Método `intersect_horizontal_plane` en `Camera` (`src/camera.rs`) para intersección analítica de rayo con plano `y = plane_y`.
  - Función Lua `get_mouse_world_pos(plane_y)` que devuelve `{ hit = bool, x = f32, y = f32, z = f32 }`.
  - Sincronización transparente de coordenadas tanto en el viewport del editor de `egui` como en el modo standalone `--play`.

### Hito 4: Navegación e IA en Rust (A\* Pathfinding)
- **Módulo `src/nav.rs`:**
  - `NavGrid`: Rejilla discreta sobre el plano XZ con tamaño de celda (`cell_size`) y radio de cobertura (`half_extent`).
  - Algoritmo A\* con `std::collections::BinaryHeap` y heurística octil (`coste diagonal = 1.414`, `recto = 1.0`).
  - Prevención de cruce a través de esquinas diagonales bloqueadas adyacentes.
  - Reconstrucción de waypoints con posición inicial y final continua para desplazamiento suave.
  - Horneado automático `bake_from_world(&self.world)` y `bake_from_entities_map(&entities)`.
- **Funciones Lua:**
  - `nav_bake([cell_size], [half_extent])`: Horneado reactivo e instantáneo.
  - `find_path(sx, sz, gx, gz)`: Retorna tabla de waypoints `{ {x=..., z=...}, ... }` o `nil` si el destino es inalcanzable.
  - `nav_set_blocked(wx, wz, is_blocked)` y `nav_is_blocked(wx, wz)`: Modificación de celdas en caliente.

### Hito 5: Audio Espacial 3D y Mezclador por Canales
- **Mezclador de Canales (`src/audio.rs`):**
  - Buses independientes: `"master"`, `"sfx"`, `"music"`.
  - `set_bus_volume(bus, vol)` y `get_bus_volume(bus) -> f32`.
  - Cálculo de volumen efectivo: `volumen_base * bus_vol * master_vol`.
  - Ajuste reactivo en tiempo real del sink musical al variar el bus de música o master.
- **Audio Espacial 3D:**
  - `play_sound_3d(path, sound_pos, listener_pos, max_dist, base_volume, bus)`.
  - Atenuación por distancia: `(1.0 - (dist / max_dist)).clamp(0.0, 1.0)`.
  - Descarte silencioso con corte a distancias fuera de alcance o volúmenes menores a 0.001.
- **Funciones Lua:**
  - `play_sound_3d(path, x, y, z, max_dist, [volume], [bus])`.
  - `set_bus_volume(bus, volume)` y `get_bus_volume(bus)`.
  - `play_sound(path, [volume], [bus])` actualizado para admitir canal.

### Hito 6: Showcase v12, Release Build y Verificación
- **Escena `assets/showcase_v12.json` y Script `assets/scripts/showcase_v12.lua`:**
  - Laberinto táctico con muros perimetrales, pilares centrales y obstáculos asimétricos.
  - Agente hero con iluminación dinámica propia que sigue rutas A\* calculadas en vivo al hacer clic en el suelo.
  - Dibujo de ruta 3D en tiempo real con `draw_line_3d` y anillo de objetivo con `draw_ring_3d`.
  - Despacho de señal `goal_reached` que dispara temporizadores de celebración (`after`), partículas doradas y audio espacial 3D (`play_sound_3d`).
  - HUD táctico declarativo (`game_ui`) con telemetría en tiempo real, contador de latidos (`every(1.0)`), botón de re-horneado de rejilla y botón de silenciamiento de canal SFX.
- **Captura y Exportación:**
  - Compilación release exitosa en 15.6s.
  - Exportación standalone verificada en `dist/showcase_v12` (con estructura limpia de un solo nivel).
  - Captura del viewport guardada en `viewport_fase12_final.png` (551 KB) con 75 FPS estables.
  - Proceso terminado y puerto 3030 liberado.

---

## 3. Resumen de Pruebas Unitarias

Todos los módulos del motor compilan con **0 advertencias** y superan **42 pruebas unitarias**:

| Módulo | Pruebas | Resultado |
|---|---|---|
| `animation` | Interpolación de clips, bucle y parada | PASS |
| `assets` | Serialización de entorno, prefabs, jerarquía padre-hijo, materiales y carga de showcases v10/v11/v12 | PASS |
| `audio` | Generación de WAV, tonos sintetizados, buses de audio y atenuación espacial 3D | PASS |
| `camera` / `tests` | Raycasting 3D, intersección con planos y jerarquía matricial con ciclos | PASS |
| `ecs` | Matemática de sprites y componentes de estela (trail) | PASS |
| `exporter` | Serialización de configuración y resolución limpia de ruta de exportación | PASS |
| `game_ui` | Cálculo de anclajes relativos en pantalla | PASS |
| `input_map` | Mapeo de acciones, botones de ratón y persistencia JSON | PASS |
| `mcp_server` | Serialización JSON-RPC v3 y comandos de inspección | PASS |
| `nav` | Coordenadas de rejilla, A\* con obstáculos y detección de metas inalcanzables | PASS |
| `particles` | Ciclo de vida y límites del sistema de partículas | PASS |
| `renderer` | Layout uniforme de luces y generación de texturas | PASS |
| `scripting` | Roundtrip JSON, bindings de animación, señales/timers, mapeo de acciones y navegación A\* | PASS |
| `templates` | Generación de las 4 plantillas y manejo de nombres inválidos | PASS |
| `tweens` | Ciclo de vida y evaluación de tweening | PASS |
| `ui_theme` | Aplicación de estilos visuales modernos | PASS |
| **Total** | **42 pruebas unitarias** | **100% OK** |

---

## 4. Estado de Archivos y Repositorio

- **Binario Release:** `target/release/motor_dif.exe`
- **Paquete Exportado Standalone:** `dist/showcase_v12/` (`showcase_v12.exe`, `game.json`, `assets/`)
- **Captura de Pantalla:** `viewport_fase12_final.png` (enviado a directorio de artefactos)
- **Servidor MCP:** Probado en `127.0.0.1:3030/mcp` y puerto liberado.
