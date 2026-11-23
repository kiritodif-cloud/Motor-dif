# Resumen Fase 18: Level & Terrain Forge — Generación Procedural de Mundos

## 1. Visión General
La **Fase 18** dota a **Motor-dif** de un conjunto completo de herramientas procedurales para la creación y generación en tiempo real de mundos 3D, mazmorras y distribución ecológica de props:
- **Terrain Forge:** Generador de terrenos 3D basado en mapas de altura (*heightmaps*) mediante ruido fractal (*Fractional Brownian Motion / FBM*), coloreado procedural según elevación y pendiente, exportación a glTF 2.0 binario (`.glb`) y generación de colliders de malla estática (`trimesh`).
- **Dungeon Forge:** Algoritmo determinista de generación de interiores y mazmorras con salas no solapadas, pasillos ortogonales conectados, muros perimetrales automáticos con colliders de caja estáticos, y puntos de interés (*Spawn Point*, *Boss Point*, *Loot Points*).
- **Scatter System:** Sistema de distribución orgánica de props mediante muestreo de disco de Poisson (*Poisson-disc rejection sampling*) respetando distancias mínimas (`min_spacing`), muestreo de altura sobre el terreno y exclusión de pendientes pronunciadas (`max_slope_deg`).
- **Integración Total:** Exposición completa a través de herramientas MCP (`127.0.0.1:3030/mcp`), API de scripting Lua (`get_terrain_height(x, z)`), y panel visual en el editor (`[L] Level Forge — Generador de Mundos`).

---

## 2. Componentes y Arquitectura

### 2.1 Terrenos Procedurales 3D (`TerrainRecipe` y `sample_terrain_height`)
Ubicación: `src/level_forge.rs`
- **Estructura de Receta:**
  ```rust
  pub struct TerrainRecipe {
      pub name: String,
      pub width: f32,
      pub depth: f32,
      pub resolution: usize, // e.g. 32..96 vértices por lado
      pub max_height: f32,
      pub noise_scale: f32,
      pub octaves: u32,
      pub seed: u32,
      pub island_falloff: bool,
      pub flat_shading: bool,
      pub palette: String, // "alpine" | "desert" | "alien" | "volcanic"
  }
  ```
- **Cálculo de Altura y Pendiente:**
  - Altura: FBM 2D de octavas acumuladas con persistencia 0.5 y lacunaridad 2.0.
  - *Island Falloff*: Atenuación radial cuadrática suave `max(0.0, 1.0 - (d / radius)^2)` que garantiza orillas al nivel del mar.
  - Normales de Superficie: Muestreo central por diferencias finitas en cruz `(dx, dz)` normalizado.
- **Paletas de Color Bioma:**
  - `"alpine"`: Arena de playa (`h < 0.12`), praderas verdes (`0.12..0.55`), roca escarpada (`0.55..0.75`), y cumbres nevadas (`h > 0.75`).
  - `"desert"`: Arenas doradas, dunas intermedias y cañones rocosos rojizos en pendientes altas (`slope > 0.45`).
  - `"alien"`: Abismos púrpuras, llanuras magenta bioluminiscentes y acantilados de cian brillante.
  - `"volcanic"`: Magma ardiente en depresiones llanas, basalto negro de obsidiana y ceniza volcánica en cumbres.
- **Horneado a GLB y Físicas Trimesh:**
  - `recipe.bake_to_glb(path)` genera un archivo `.glb` binario autocontenido.
  - `physics.add_trimesh_body(transform, vertices, indices, is_sensor)` integra la malla generada directamente en el motor de físicas `Rapier3d` como un cuerpo rígido estático.

### 2.2 Generador de Mazmorras Procedurales (`DungeonRecipe` y `generate_dungeon`)
Ubicación: `src/level_forge.rs`
- **Estructura:**
  ```rust
  pub struct DungeonRecipe {
      pub name: String,
      pub grid_width: usize,
      pub grid_height: usize,
      pub cell_size: f32,
      pub room_count: usize,
      pub seed: u32,
      pub wall_height: f32,
      pub theme: String,
  }
  ```
- **Generación Determinista:**
  - Generador de números pseudoaleatorios LCG reproducible.
  - Colocación de salas rectangulares con comprobación de solapamiento estricta más margen de separación.
  - Conexión secuencial de salas adyacentes mediante pasillos en forma de L ortogonales.
  - Detección de muros exteriores y bordes libres; generación automática de celdas de muro perimetrales.
  - Marcadores de posición de juego: `spawn_point` en la sala inicial, `boss_point` en la sala final y `loot_points` en las salas intermedias.

### 2.3 Sistema de Dispersión de Props (`ScatterRecipe` y `generate_scatter_instances`)
Ubicación: `src/level_forge.rs`
- **Muestreo Poisson-Disc:**
  - Muestreo en disco uniforme con rechazo por proximidad cuadrática `min_spacing^2`.
  - Muestreo de elevación `sample_terrain_height` y pendiente `acos(normal.y)` en grados.
  - Filtrado estricto por umbral de pendiente máxima (`max_slope_deg`).
  - Escala aleatoria configurable (`scale_min..scale_max`) y rotación arbitraria en Y (`random_yaw`).

---

## 3. Integración en Scripting Lua

Se expuso la función global de consulta de altura sobre el terreno:
```lua
-- Obtiene la altura del terreno activo en coordenadas de mundo (x, z)
local ground_y = get_terrain_height(x, z)
```
- Vinculado al `active_terrain: Arc<RwLock<Option<TerrainRecipe>>>` dentro del contexto `LuaEngineContext`.
- Permite a los agentes, rovers y cámaras cinemáticas seguir la topografía del terreno en tiempo real con latencia cero.

---

## 4. Herramientas MCP (`127.0.0.1:3030/mcp`)

Se implementaron y registraron 3 nuevas herramientas en el servidor MCP:

1. **`forge_generate_terrain`:**
   - Parámetros: `{ recipe: TerrainRecipe, output_name: string, insert_in_scene: bool }`
   - Retorna: `{ success: true, path: string, vertices: number, triangles: number, inserted: bool }`
   - Actualiza automáticamente `active_terrain` para que los scripts Lua puedan consultar alturas inmediatamente.
2. **`forge_generate_dungeon`:**
   - Parámetros: `{ recipe: DungeonRecipe, insert_in_scene: bool }`
   - Retorna: `{ success: true, rooms: number, floor_cells: number, wall_cells: number, spawn_point: [x,y,z], boss_point: [x,y,z], loot_points: [...] }`
   - Instancia salas, suelos y muros con colliders `cuboid` estáticos en el `World`.
3. **`forge_scatter_props`:**
   - Parámetros: `{ recipe: ScatterRecipe, terrain_recipe?: TerrainRecipe }`
   - Retorna: `{ success: true, prop_mesh: string, count: number, positions: [[x,y,z], ...] }`
   - Dispersa e inserta las entidades en la escena respetando el mapa de alturas y pendientes.

---

## 5. Interfaz Visual en el Editor (`egui`)

- **Acceso:**
  - Menú Superior: `File` -> `[L] Level Forge...`
  - Command Palette (`Ctrl+P`): `Level Forge (Worlds & Dungeons)`
- **Pestaña `Terrain`:**
  - Controles deslizantes para `width`, `depth`, `resolution` (con aviso preventivo si `> 96`), `max_height`, `noise_scale`, `octaves`, `seed`, `island_falloff`, `flat_shading` y selector de paleta (`alpine`, `desert`, `alien`, `volcanic`).
  - Campo de texto para nombre de archivo `.glb` y botón `[Generar y Hornear Terreno]`.
- **Pestaña `Dungeon`:**
  - Controles para `room_count`, dimensiones de grid `grid_width` / `grid_height`, tamaño de celda, altura de muros y semilla.
  - Botón `[Generar Mazmorra en Escena]`.
- **Pestaña `Scatter`:**
  - Selector de modelo de prop (`mesh` o ruta `.glb`), cantidad de props, radio de dispersión, espaciado mínimo (`min_spacing`), pendiente máxima (`max_slope_deg`), rangos de escala y rotación aleatoria.
  - Botón `[Dispersar Props]`.

---

## 6. Escena Showcase y Verificación Visual

- **Archivo de Escena:** `assets/showcase_v18_terrain.json`
- **Modelo de Terreno Horneado:** `assets/models/terrain_demo.glb` (302 KB, 64x64 resolución, paleta `"alien"`).
- **Props Dispersados:** Monolitos de cristal bioluminiscente cian y magenta distribuidos en el terreno con `min_spacing = 4.5`.
- **Scripting y Navegación:** `assets/scripts/terrain_scout.lua`:
  - Rover de exploración patrullando en órbita continua sobre el terreno.
  - Consulta en tiempo real de `get_terrain_height(x, z)` para flotar a altura constante sobre el suelo.
  - Ajuste dinámico de la intensidad musical de `Music Forge` (`space_ambient`) según la elevación del rover.
- **Captura Verificada:** `viewport_fase18_level.png` (489,369 bytes) capturada mediante MCP a 75 FPS con 33 entidades activas.

---

## 7. Resultados de Compilación y Tests

- **Compilación Release:** `cargo build --release` -> **0 errores, 0 warnings**.
- **Tests Automatizados:** **71 tests ejecutados, 71 pasados (100% de éxito)**.
  - `test_terrain_height_and_mesh_generation_no_nan`: Vértices, normales unitarias y caídas sin NaN/Inf.
  - `test_bake_alien_terrain_demo_glb`: Horneado de archivo GLB binario válido.
  - `test_dungeon_generation_and_connectivity`: Salas no superpuestas, conectividad y puntos de interés.
  - `test_scatter_min_spacing_and_slope_rejection`: Cumplimiento estricto de Poisson-disc y rechazo de pendientes.
  - `test_generate_showcase_v18_terrain_scene`: Generación determinista de escena de demostración.
  - `test_load_showcase_v18_terrain`: Carga y deserialización de la escena completa en el motor.
  - `test_terrain_height_lua_binding`: Consulta precisa de altura desde scripts Lua.
  - `test_terrain_scout_script_compilation`: Ejecución y actualización del rover sobre el terreno.
  - `test_mcp_forge_tools_schema_and_commands`: Esquema y despacho de herramientas MCP de Level Forge.
- **Estado de Red:** Puerto `3030` libre y limpio.
