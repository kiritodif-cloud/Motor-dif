# Motor-dif — Fase 10: Motor de Siguiente Nivel (Next-Gen Indie Engine)

## 1. Resumen Ejecutivo
En esta Fase 10, **Motor-dif** evoluciona de un motor de prototipado a un motor indie 3D de alta fidelidad visual y capacidades completas de gameplay dinámico. Se introdujo una cadena completa de renderizado HDR con post-procesamiento en GPU, materiales estilizados modernos (Fresnel rim glow, modos de mezcla additiva/alfa, scroll de UVs), primitivos procedurales avanzados (`torus`, `ring`, `cone`, `icosphere`), sistema de estelas con ribbons additivos, dibujo 3D inmediato en Lua, rótulos de mundo y textos flotantes proyectados, atmósfera con skybox procedimental en GPU y niebla exponencial `FogExp2`, física de gameplay con triggers/sensores en Rapier3D, raycasting 3D en Lua y un sintetizador procedural de audio SFX en memoria.

---

## 2. Hitos Implementados

### Hito 1: Cadena de Renderizado HDR y Post-Procesamiento (`src/renderer.rs`)
- **Render Target HDR (`Rgba16Float`):** La escena 3D ya no se renderiza directamente sobre el framebuffer SDR de 8 bits. Se creó una textura intermedia en formato `wgpu::TextureFormat::Rgba16Float` con resolución nativa.
- **Shader de Post-Procesamiento (`POST_PROCESS_SHADER_SOURCE`):**
  - **Bloom Glow por Kernel 9-tap:** Muestreo de dos radios con umbral de luminancia (`bloom_threshold`) e intensidad configurable (`bloom_intensity`).
  - **ACES Filmic Tone Mapping:** Curva de mapeo tonal de grado cinematográfico que comprime los valores HDR $> 1.0$ evitando artefactos de saturación y banding:
    $$\text{ACES}(x) = \frac{x \cdot (2.51 \cdot x + 0.03)}{x \cdot (2.43 \cdot x + 0.59) + 0.14}$$
  - **Vignette Suave:** Oscurecimiento periférico radial modulado por exponente suave.
- **Control en Tiempo Real:** Sliders en el Inspector (`Post-Processing (HDR)`) y bindings en Lua (`set_post_process(bloom_enabled, bloom_intensity, bloom_threshold, vignette, exposure)` y `get_post_process()`).

### Hito 2: Materiales Estilizados y Modos de Mezcla (`src/renderer.rs` + `src/assets.rs`)
- **Atributos por Instancia (`InstanceRaw` expandido a 128 bytes):**
  - `fresnel_color` (`[f32; 3]`) y `fresnel_power` (`f32`): Borde luminoso dependiente del ángulo de visión:
    $$\text{factor} = \max(0.0, 1.0 - N \cdot V)^{\text{power}}$$
  - `uv_scroll` (`[f32; 2]`): Desplazamiento dinámico de coordenadas UV en GPU por tiempo total.
  - `blend_mode` (`"opaque"`, `"alpha"`, `"additive"`): Tubería de renderizado additivo separada (`additive_render_pipeline`) con función de mezcla `wgpu::BlendState { color: Add, alpha: Add }`.
  - `wireframe`: Dibujo de líneas de borde sobre las mallas 3D sin recompilar shaders.
  - `emissive`: Soporte de emisión de luz hasta 5.0 (activando el Bloom en el pase HDR).
- **Control en Lua e Inspector:** Función `set_material(id, opts)` e interfaz visual en el Inspector.

### Hito 3: Primitivos Procedurales, Estelas y Dibujo 3D Inmediato
- **Nuevos Primitivos Paramétricos:**
  - `torus`: Generador analítico de rosquilla 3D con radio mayor y radio de tubo.
  - `ring`: Disco anular plano con normales frontales y traseras.
  - `cone`: Cono cerrado con ápice afilado y base circular teselada.
  - `icosphere`: Esfera geodésica subdividida a partir de icosaedro regular con normalización radial perfecta.
  - Integrados en botones de jerarquía, paleta de comandos (`Ctrl+K`) e Inspector.
- **Sistema de Estelas (`TrailComponent`):**
  - Búfer circular de puntos `(pos, time)`.
  - Poda automática por tiempo de vida (`lifetime`).
  - Generación de malla de ribbon additivo orientada hacia la cámara en cada frame.
  - Control en Lua: `set_trail(id, r, g, b, a, lifetime, width)`.
- **Dibujo 3D Inmediato en Lua:**
  - `draw_line_3d(x1, y1, z1, x2, y2, z2, r, g, b, a)`
  - `draw_ring_3d(cx, cy, cz, radius, r, g, b, a)`
- **World Labels y Floating Text:**
  - Proyección de coordenadas 3D a pantalla mediante `camera.world_to_screen()`.
  - Rótulos permanentes: `set_world_label(entity_id, text, offset_y, r, g, b)`.
  - Textos animados con flotación vertical y fade out: `spawn_floating_text(x, y, z, text, r, g, b, lifetime)`.

### Hito 4: Atmósfera, Skybox GPU y Niebla Exp2 (`src/camera.rs` + `src/renderer.rs`)
- **Shader de Skybox Procedural (`skybox_pipeline`):**
  - Fullscreen triangle sin buffer de vértices (`@builtin(vertex_index)`).
  - Tres modos:
    1. `"space"`: Fondo estelar 3D por función hash cúbica, nebulosas de color degradado y estrellas con intensidad $> 1.0$ que generan destellos HDR con el Bloom.
    2. `"gradient"`: Gradiente suave entre color de cenit (`zenith_color`) y horizonte (`horizon_color`).
    3. `"solid"`: Color uniforme de fondo.
- **Niebla Exponencial FogExp2:**
  - Integrada en el shader de iluminación 3D:
    $$\text{fog\_factor} = 1.0 - \exp(-\text{dist} \cdot \text{fog\_density})$$
    $$\text{color} = \text{mix}(\text{color}, \text{fog\_color}, \text{fog\_factor})$$
- **Persistencia:** Guardado y carga en `SceneDef.environment` (`assets/showcase_v10.json`).

### Hito 5: Físicas de Gameplay Real (Triggers, Raycast y SFX Synth)
- **Colliders Sensores / Triggers (`rapier3d`):**
  - Propiedad opcional `is_sensor: Option<bool>` en `PhysicsDef`.
  - Creación con `.sensor(true)` y captura de eventos `CollisionEvent::Started(..., CollisionEventFlags::SENSOR)`.
  - Despacho automático al método Lua `function on_trigger_enter(other_id)`.
- **Raycasting 3D en Lua (`raycast`):**
  - Consulta `broad_phase.as_query_pipeline(...).cast_ray_and_get_normal()`.
  - Retorno en tabla Lua: `{ hit = bool, entity = id_str, dist = f32, nx = f32, ny = f32, nz = f32 }`.
- **Sintetizador SFX Procedural (`play_tone`):**
  - Generación de ondas PCM de 16-bit a 44.1 kHz en memoria (`sine`, `square`, `sawtooth`, `noise`).
  - Barrido de frecuencia `f_start -> f_end` con envolvente suave libre de clics.
  - Reproducción instantánea sin dependencias de disco vía `rodio`.

---

## 3. Demostración y Capturas

| Captura | Descripción |
|---|---|
| `viewport_fase10_hdr.png` / `viewport_fase10_play.png` | Vista completa en `--play` mostrando el núcleo de plasma con HDR Bloom, halo aditivo, anillo de teletransporte, anillos de vector 3D en suelo, etiquetas flotantes y skybox espacial. |
| `viewport_fase10_editor.png` | Vista del editor en vivo con jerarquía padre-hijo, gizmos 3D de transformación, inspector de materiales estilizados y consola MCP. |

---

## 4. Pruebas y Benchmarks

- **Compilación en Release:** `cargo build --release` completado con **0 errores y 0 warnings bloqueantes**.
- **Suite de Pruebas Automatizadas:** 17 tests unitarios pasando exitosamente (`17 passed; 0 failed`).
- **Rendimiento:** **75 FPS estables** en modo ventana y editor con post-procesamiento HDR, bloom 9-tap, niebla y partículas activas.
- **MCP Server:** Puerto 3030 probado y liberado limpiamente.
