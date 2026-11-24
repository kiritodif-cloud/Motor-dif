# RESUMEN DE LA FASE 22: SALTO VISUAL — SOMBRAS, ACES, SHADER DE TERRENO Y PROPS ESTILIZADOS

## 1. Hito 1: Sombras Direccionales Suaves (Shadow Mapping + PCF 3x3) en `wgpu`
- **Textura de Profundidad Dedicada:** Se implementó un mapa de sombras en formato `wgpu::TextureFormat::Depth32Float` con resolución de `2048x2048`, configurado con `usage: RENDER_ATTACHMENT | TEXTURE_BINDING`.
- **Muestreador de Comparación Hardware:** Creado con `compare: Some(wgpu::CompareFunction::LessEqual)` y modo de direccionamiento `ClampToEdge`.
- **Pipeline de Sombras (Depth-Only):**
  - Shader `SHADOW_SHADER_SOURCE` dedicado sin etapa de fragmentos (`fragment: None`), minimizando consumo en gráficas integradas.
  - Matriz de luz ortográfica en `ShadowUniform` (`[[-45, 45], [-45, 45], [1, 120]]`) centrada dinámicamente alrededor del objetivo de la cámara (`camera.target`).
  - `wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 }`.
- **Filtro PCF 3x3 con Bias Adaptativo:**
  - Función WGSL `fetch_shadow(shadow_pos, n_dot_l)` con 9 muestras ponderadas y slope-scaled bias `max(0.004 * (1.0 - n_dot_l), 0.001)`.
  - La sombra modula directamente la luz difusa y especular direccional, manteniendo intactas la luz ambiental, luces puntuales, emisiones y resplandor Fresnel.

## 2. Hito 2: Tone Mapping ACES con Saturación Emisiva, Ambiente Hemisférico y Supresión de Toasts
- **Tone Mapping Fílmico ACES Saturado:**
  - Se extendió el pase de post-procesamiento con `aces_filmic_saturated(color)`.
  - Mapea el pico de luminancia preservando el vector cromático original de materiales emisivos (65% balance de luminancia y 35% por canal), evitando que los neones, cristales y lava se desvanezcan a blanco puro quemado tras el bloom.
- **Iluminación Ambiental Hemisférica:**
  - Gradiente en fragment shader `mix(ground_ambient, sky_ambient, hemi_factor)` donde `sky_ambient` y `ground_ambient` se modulan en tiempo real con `camera.zenith_color` y `camera.horizon_color`.
- **Supresión de Toasts en Modo Play:**
  - El overlay inferior derecho de notificaciones del editor se oculta cuando `is_playing == true` (`is_standalone_play || play_mode == PlayMode::Play`), garantizando una experiencia de juego limpia y sin distracciones.

## 3. Hito 3: Micro-Detalle Procedural, Paleta Volcánica y Props Estilizados
- **Micro-Detalle de Superficie Procedural:**
  - Ruido bicúbico procedimental de alta frecuencia en WGSL (`micro_noise`) basado en `world_pos.xz` modulando superficies sin textura para eliminar la apariencia plástica o lisa en rocas y terreno.
- **Paleta Volcánica Optimizada:**
  - Se calibraron los colores de `evaluate_terrain_color("volcanic", h, s)`:
    - Grietas de magma incandescente: `[0.98, 0.36, 0.05, 1.0]`
    - Riscos y acantilados de ceniza: `[0.18, 0.16, 0.19, 1.0]`
    - Picos de basalto oscuro: `[0.14, 0.13, 0.15, 1.0]`
    - Suelo de basalto profundo: `[0.09, 0.09, 0.11, 1.0]`
- **4 Presets de Props Estilizados Low-Poly:**
  - `basalt_pillar`: Columna hexagonal biselada con capitel facetado.
  - `jagged_rock`: Roca poliédrica asimétrica deformada con normales planas.
  - `crystal_cluster`: Trío de prismas de cristal inclinados con brillo emisivo y halo Fresnel.
  - `sci_fi_crate`: Caja de suministros con biseles y paneles rehundidos.
  - Totalmente integrados en `GpuContext::new`, `ensure_mesh`, Level Forge y MCP.

## 4. Hito 4: Documentación, Tests y Compilación Release
- **Actualización de `AGENT_GUIDE.md`:** Se documentaron los nuevos tipos de malla y presets de dispersión manteniendo el archivo en 175 líneas (< 240 líneas).
- **Pruebas Unitarias:** 92 de 92 pruebas pasando al 100% (`cargo test --bin motor_dif`).
- **Compilación de Producción:** `cargo build --release` completado con 0 errores y 0 advertencias.
- **Validación en Vivo por MCP:**
  - Servidor verificado en `http://127.0.0.1:3030/mcp`.
  - Herramientas `get_engine_docs`, `load_scene`, `set_play_mode`, `capture_viewport` y `get_performance_stats` probadas exitosamente.
  - **Rendimiento:** ~74.8 FPS en modo Release con 20+ entidades, mapa de sombras 2048x2048 y post-procesamiento HDR activo.
