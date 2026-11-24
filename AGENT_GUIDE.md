# Motor-dif — Agent & LLM Quick Reference Guide

## quickstart
- Execution lifecycle & rules:
  - `on_update(entity, dt)`: Called each frame during `PlayMode::Play`. `entity.x/y/z` (pos), `entity.rx/ry/rz` (Euler rot in RADIANS, not degrees! full turn = 6.283), `entity.sx/sy/sz` (scale). Method: `entity:apply_force(fx,fy,fz)`, `entity:apply_impulse(ix,iy,iz)`.
  - Init idiom: `local init=false; function on_update(e,dt) if not init then init=true; on_init(e) end ... end`.
  - Player-First Rule: Always spawn a player controller with `forge_spawn_controller({ controller_type: "fps"|"third_person"|"top_down"|"vehicle" })` before asking the user to test the scene.
  - Button callbacks: Define `function my_click() ... end` globally; referenced by name in `ui_button`.
  - File rule: Never overwrite `assets/scene.json` unless asked. Save new projects to `assets/<name>.json` and `assets/scripts/<name>.lua`, then load with `load_scene`.
  - Standalone & Export: Running with `game_config.json` auto-boots cleanly in Standalone Mode without editor chrome. Press `F11` to toggle fullscreen. Export anytime via `export_game` or `File -> Export Game...`.
  - Visual Quality Rules:
    - NEVER use `set_hud_text` for game HUDs. Always use `ui_panel`, `ui_bar`, `ui_label`, `ui_button` from `lua_ui`.
    - Use Texture Forge (`forge_create_texture`), Asset Forge (`forge_create_model`), Procedural Rig (`forge_create_rigged_character`), and VFX Forge (`forge_spawn_vfx`) instead of plain untextured grey blocks.
    - Safe lighting & bloom: `bloom_intensity: 0.25..0.45`, `bloom_threshold: 0.8`, `exposure: 1.0`. Avoid `emissive > 0.2` on large grounds/buildings to prevent washed-out white overexposure. Use gentle fog (`fog_density: 0.002..0.008`).
  - Cleanup rule: When resetting game state in scripts, always call `ui_clear()`, `clear_timers()`, and `clear_events()`.
- Mini Example: Movement + Declarative HUD (`assets/scripts/player.lua`):
```lua
local score, speed = 0, 5.0
function on_update(entity, dt)
    local mx, mz = 0, 0
    if is_key_down("W") then mz = mz - 1 end
    if is_key_down("S") then mz = mz + 1 end
    if is_key_down("A") then mx = mx - 1 end
    if is_key_down("D") then mx = mx + 1 end
    entity.x = entity.x + mx * speed * dt
    entity.z = entity.z + mz * speed * dt
    entity.ry = entity.ry + 1.2 * dt -- rot in radians
    ui_panel("hud_bg", "top_left", 16, 16, 160, 42, 0.1, 0.12, 0.16, 0.85)
    ui_label("hud_txt", "top_left", 28, 28, "Score: " .. score, 14, 0.4, 0.9, 1.0)
end
```

## scene_json
```json
{
  "camera": { "mode": "3d", "position": [0, 5, 12], "target": [0, 1, 0], "ortho_size": 10.0 },
  "post_process": { "bloom_enabled": true, "bloom_intensity": 0.35, "bloom_threshold": 0.8, "vignette_intensity": 0.35, "exposure": 1.0 },
  "environment": { "sky_mode": "gradient", "zenith_color": [0.07, 0.08, 0.12], "horizon_color": [0.13, 0.15, 0.21], "fog_color": [0.07, 0.08, 0.12], "fog_density": 0.005, "time_of_day": 12.0 },
  "entities": [
    {
      "id": "player", "mesh": "cube", "position": [0, 1, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1],
      "color": [0.2, 0.7, 1.0, 1.0], "emissive": 0.15, "fresnel_color": [0.4, 0.8, 1.0], "fresnel_power": 2.5,
      "blend_mode": "opaque", "parent": null, "script": "assets/scripts/player.lua",
      "physics": { "body": "dynamic", "collider": "cuboid", "mass": 1.0, "restitution": 0.2, "is_sensor": false },
      "point_light": { "color": [0.3, 0.8, 1.0], "intensity": 2.5, "radius": 8.0 },
      "vfx_emitter": { "preset_or_recipe": "fire", "enabled": true, "looping": true, "offset": [0, 1, 0] },
      "animation_player": { "default_clip": "idle_breathe" },
      "rig_controller": { "preset": "mech_biped", "locomotion_enabled": true }
    }
  ]
}
```
Meshes: `"cube"`, `"sphere"`, `"cylinder"`, `"quad"`, `"plane"`, `"torus"`, `"ring"`, `"cone"`, `"icosphere"`, `"sword"`, `"pickaxe"`, `"basalt_pillar"`, `"jagged_rock"`, `"crystal_cluster"`, `"sci_fi_crate"`, or `"assets/models/*.glb"`.
Rotation units: Euler angles in RADIANS `[rx, ry, rz]` (e.g. `[0, 1.57, 0]` for 90° yaw).
Blend modes: `"opaque"`, `"translucent"`, `"additive"`.
Sky modes: `"gradient"`, `"space"`, `"solid"`.
Physics bodies: `"dynamic"`, `"fixed"`. Colliders: `"cuboid"`, `"ball"`.
Camera modes: `"2d"`, `"3d"`. Ortho size controls orthographic zoom in 2D.

## lua_core
- Entity management:
  - `spawn_entity(table)`: Spawns entity. Table accepts `{ id, mesh, x, y, z, rx, ry, rz, sx, sy, sz, color={r,g,b,a}, emissive, texture, script, parent, physics={body,collider,mass,restitution,is_sensor} }`.
  - `destroy_entity(id)`: Removes entity from ECS and physics world.
  - `get_entity(id) -> table|nil`: Returns `{ x, y, z, rx, ry, rz, sx, sy, sz }`.
  - `get_all_entities() -> table`: Returns `{ [id] = { id, x, y, z, ... } }`.
  - `set_parent(child_id, parent_id_or_nil)`: Hierarchical transform attachment.
  - `set_material(id, { color={r,g,b,a}, emissive=f, fresnel_color={r,g,b}, fresnel_power=f, blend_mode="opaque"|"additive", wireframe=b, uv_scroll={u,v} })`
  - `set_pbr_material(id, { metallic=f, roughness=f, normal_map=str, metallic_roughness_map=str, emissive_map=str, normal_scale=f, emissive_intensity=f })`
  - `set_light(id, r, g, b, intensity, radius)`: Updates/creates PointLight component.
  - `tween_position(id, x, y, z, duration)`: Smooth linear interpolation of position.
  - `tween_scale(id, sx, sy, sz, duration)`: Smooth scale interpolation.
  - `set_sprite_frame(id, cols, rows, frame_idx)`: 2D sprite sheet tile selection.
  - `load_scene(path)`: Queues scene transition on frame boundary.
  - `save_data(key, val)`, `load_data(key) -> val`: Legacy JSON persistence to `assets/save_data.json`.
  - `save_game(slot, data) -> bool`, `load_game(slot) -> table|nil`, `delete_save(slot) -> bool`, `list_saves() -> table`: Slotted JSON save/load in `saves/<slot>.json` with path traversal defense.
- Camera & Input:
  - `set_camera(x,y,z, tx,ty,tz)`, `follow_entity(id, ox,oy,oz, smoothness)`
  - `camera_shake(intensity, duration)`, `camera_lerp_to(ex,ey,ez, tx,ty,tz, duration)`
  - `camera_orbit_target(id, distance, pitch_deg, yaw_speed)`
  - `get_mouse_world_pos([plane_y]) -> { x, y, z }`, `get_mouse_delta() -> { dx, dy }`
  - `set_pointer_lock(enabled: bool)`, `is_pointer_locked() -> bool`
  - `is_key_down(key) -> bool`, `is_key_just_pressed(key) -> bool` (e.g. `"W"`, `"SPACE"`, `"LSHIFT"`)
  - `is_action_down(act) -> bool`, `is_action_just_pressed(act) -> bool`, `bind_action(act, key)`
- Physics & Raycast:
  - `apply_force(fx, fy, fz)`, `apply_impulse(ix, iy, iz)`: Operates on self entity rigid body.
  - `raycast(ox, oy, oz, dx, dy, dz, max_dist) -> { hit=bool, entity_id=str, point={x,y,z}, normal={x,y,z}, distance=f }|nil`
- Timers & Events:
  - `after(delay_secs, callback_name) -> timer_id`, `every(interval_secs, callback_name) -> timer_id`
  - `cancel_timer(timer_id)`, `clear_timers()`
  - `on_event(name, callback_name)`, `emit_event(name, payload)`, `clear_events()`
- Atmosphere & View-Model:
  - `set_time_of_day(hour)`: Sets celestial time (0.0..24.0), updating sun/moon directional lighting, shadows, sky gradient, stars.
  - `get_time_of_day() -> f32`: Returns current celestial hour.
  - `set_viewmodel(mesh, [texture], [offset_xyz], [scale_xyz])`: Binds 1st-person viewmodel to camera with anti-clipping depth clear (presets: `"sword"`, `"pickaxe"`, `"cube"`, or `.glb`).
  - `clear_viewmodel()`: Clears active viewmodel.
  - `trigger_viewmodel_action(action_type, [intensity])`: Triggers procedural animation (`"swing"`, `"recoil"`, `"bob"`).

## lua_ui
- Declarative In-Game HUD:
  - Anchors: `"top_left"`, `"top_center"`, `"top_right"`, `"center"`, `"bottom_left"`, `"bottom_center"`, `"bottom_right"`.
  - `ui_clear()`: Clears all HUD widgets.
  - `ui_panel(id, anchor, x, y, w, h, r, g, b, a)`: Background container card.
  - `ui_bar(id, anchor, x, y, w, h, value_0_to_1, r, g, b, a)`: Dynamic progress/health bar.
  - `ui_label(id, anchor, x, y, text, size, r, g, b, a)`: Text label (re-call with same ID to update text).
  - `ui_button(id, anchor, x, y, w, h, text, callback_name)`: Interactive button triggering global Lua function.
  - `ui_image(path, x, y, w, h, [uv_rect])`: Draws 2D texture/sprite from `assets/`. `uv_rect` optional `{u0,v0,u1,v1}` for atlas slicing.
  - `ui_slot(x, y, size, [texture_path], [uv_rect], [count_text], [selected], [hotkey_label])`: Styled inventory/hotbar slot with gold highlight, icon, count, and hotkey.
- 3D Overlays & Visual Debug:
  - `set_world_label(entity_id, text, r, g, b, [a])`: Floating 3D billboarded text above entity.
  - `spawn_floating_text(x, y, z, text, r, g, b, [duration])`: Floating combat damage/status text.
  - `draw_line_3d(x1, y1, z1, x2, y2, z2, r, g, b, [a])`: 3D debug line.
  - `draw_ring_3d(cx, cy, cz, radius, r, g, b, [a])`: Horizontal circle in XZ.
  - `set_trail(id, r, g, b, [lifetime], [width])`: Ribbon trail trailing behind moving entity.

## lua_vfx_audio
- VFX Forge (Hardware Quad Instancing with HDR Bloom):
  - `spawn_vfx(preset_or_json, x, y, z)`: Spawns one-shot or burst particle effect at world position.
  - `attach_vfx(entity_id, preset_or_json, [ox, oy, oz])`: Attaches continuous emitter to entity.
  - `stop_vfx(entity_id)`: Detaches and removes emitter from entity.
  - Built-in presets: `"explosion"`, `"fire"`, `"magic_portal"`, `"hit_spark"`, `"level_up"`.
  - `spawn_particles(x, y, z, r, g, b, [count], [speed], [lifetime])`: Legacy particle burst.
- Audio System:
  - `play_sound(path, [vol], [bus])`: Buses: `"master"`, `"sfx"`, `"music"`.
  - `play_sound_3d(path, x, y, z, max_dist, [vol], [bus], [curve])`: Attenuated 3D audio with camera stereo panning (curves: `"linear"`, `"exponential"`, `"smoothstep"`).
  - `play_tone(f_start, f_end, wave_type, duration_secs, vol)`: Wave types: `"sine"`, `"square"`, `"triangle"`, `"sawtooth"`.
  - `set_bus_volume(bus, vol)`, `get_bus_volume(bus) -> vol`
- Music Forge (Procedural Adaptive Synthesis):
  - `play_music_preset(preset_name)`: Presets: `"space_ambient"`, `"boss_battle"`, `"cyber_action"`, `"menu_chill"`, `"synthwave"`.
  - `play_music_recipe(json_str)`: Plays serialized `MusicRecipe`.
  - `set_music_intensity(0.0_to_1.0)`: Dynamic adaptive layer blending.
  - `stop_music()`: Stops adaptive music stream.

## lua_ai_rig
- Navigation & Pathfinding:
  - `nav_bake([cell_size], [extent])`: Bakes 2D collision grid over colliders.
  - `find_path(sx, [sy], sz, gx, [gy], gz, [budget]) -> { {x, y, z}, ... }|nil`: 2D/3D voxel & terrain-aware A* path with step budget.
  - `steer_towards(entity_or_id, tx, ty, tz, speed, [turn_speed], [dt]) -> bool`: Moves entity towards target at `speed*dt` with smooth yaw rotation. Returns `true` if arrived (<0.2m).
  - `draw_path(waypoints, [r, g, b, a])`: Renders 3D path debug lines in viewport.
  - `nav_set_blocked(wx, wz, is_blocked)`, `nav_is_blocked(wx, wz) -> bool`
- Level Forge:
  - `get_terrain_height(x, z) -> f32`: Evaluates procedural heightmap elevation at world coordinates.
- Voxel Forge:
  - `voxel_get_block(x, y, z) -> id: int`, `voxel_set_block(x, y, z, id: int)`: Reads/writes blocks in active voxel world.
  - `voxel_is_solid(x, y, z) -> bool`: Fast collision check for voxel physics/navigation.
  - `voxel_raycast(ox, oy, oz, dx, dy, dz, max_dist) -> { hit=bool, x, y, z, nx, ny, nz, id, dist }`: DDA grid raycast for block break/place.
- AnimationPlayer:
  - `play_anim(entity_id, clip_name, [blend_sec], [loop])`: Plays clip with linear crossfade blending.
  - `play_preset_anim(entity_id, preset_name, [loop])`: Presets: `"idle_breathe"`, `"walk_bob"`, `"spin_continuous"`.
  - `stop_anim(entity_id)`, `set_anim_speed(entity_id, speed)`, `is_anim_playing(entity_id) -> bool`
- Procedural Rig (Biomechanical Locomotion & Aiming):
  - `rig_set_look_target(entity_id, x, y, z)`: Head/turret look-at tracking.
  - `rig_clear_look_target(entity_id)`
  - `rig_trigger_recoil(entity_id, strength)`: Weapon kickback impulse with spring decay.
  - `solve_ik_2bone(rx, ry, rz, tx, ty, tz, len1, len2) -> { jx, jy, jz, ... }|nil`: 2-bone analytical IK.

## player_templates
- `forge_spawn_controller({ controller_type, [entity_id], [position], [voxel_collision], [speed], [jump_force], [overwrite] })`:
  - Instantly spawns a player entity with mesh, dynamic collider, and complete Lua controller script in `assets/scripts/<entity_id>_controller.lua`.
  - Types:
    - `"fps"`: Pointer lock, mouse look (yaw/pitch), WASD camera-relative movement, Space jump, optional voxel collision.
    - `"third_person"`: Smooth chase camera, WASD movement, turning towards heading, Space jump, 'V' perspective toggle.
    - `"top_down"`: Isometric/overhead elevated follow camera, WASD XZ movement, character facing movement/cursor.
    - `"vehicle"`: Forward/reverse acceleration, steering with yaw accumulation, natural drag/friction, dynamic follow camera.
  - Template source files persist in `assets/scripts/templates/<type>_controller.lua`.

## mcp_workflow
Recommended 4-step autonomous loop for AI agents (`http://127.0.0.1:3030/mcp`):
1. **Query Docs:** Call `get_engine_docs({ topic: "all" })` (or specific topic) to verify signatures and workspace root without exploring source files.
2. **Author Content:**
   - Save new scene to `assets/<name>.json` and Lua scripts to `assets/scripts/<name>.lua`.
   - Or use procedural forges: `forge_create_model`, `forge_bake_glb`, `forge_create_texture`, `forge_create_music`, `forge_generate_terrain`, `forge_generate_dungeon`, `forge_scatter_props`, `forge_create_rigged_character`, `forge_spawn_vfx`.
3. **Load & Run:**
   - Call `load_scene({ path: "assets/<name>.json" })` (or bare `<name>.json`).
   - Call `set_play_mode({ playing: true })` to start simulation, initialize scripts, and hide editor gizmos.
4. **Inspect & Validate:**
   - Query `get_console_logs({ filter: "error" })` or `get_engine_logs` to ensure 0 Lua runtime errors.
   - Call `capture_viewport` to visually inspect scene composition, lighting, and HUD in Base64 PNG.
   - Call `get_performance_stats` to verify 60+ FPS, draw calls, and entity counts.
   - To stop or reset: call `set_play_mode({ playing: false })`.

MCP Tools Directory:
- `get_engine_docs({ topic })`: Compact API signatures and schemas (topics: all, quickstart, scene_json, lua_core, lua_ui, lua_vfx_audio, lua_ai_rig, mcp_workflow).
- `inspect_world()`: Returns active camera and entity definitions JSON.
- `upsert_entity(entity_def)`: Dynamically adds or modifies an entity in active scene.
- `delete_entity({ id })`: Removes entity by ID.
- `execute_lua({ code })`: Runs Lua code immediately in active environment.
- `get_console_logs({ filter })`: Retrieves engine/script log buffer.
- `get_engine_logs()`: Compact diagnostic metrics (FPS, logs, Lua error string).
- `capture_viewport()`: Captures rendered frame as Base64 PNG data.
- `load_scene({ path })`: Loads `.json` scene from `assets/` into editor, safely stopping simulation if active.
- `set_play_mode({ playing: bool })`: Starts (`playing: true`) or stops (`playing: false`) gameplay simulation.
- `save_scene({ path })`: Serializes active scene to `assets/<path>.json`.
- `get_scene_tree()`: Hierarchical parent-child tree and entity components.
- `get_performance_stats()`: Profiler metrics (FPS, frame_time_ms, entities, draw_calls, particles, bodies).
- `forge_create_model({ name, mesh, color, ... })`: Spawns 3D mesh with procedural material.
- `forge_bake_glb({ mesh_type, path })`: Exports procedural geometric mesh as GLB.
- `forge_create_texture({ style, filename, size, seed, color_tint })`: Generates procedural texture PNG.
- `forge_create_music({ preset, recipe, bake_path, play_now, intensity })`: Synthesizes procedural adaptive music.
- `set_music_intensity({ intensity })`: Blends adaptive music intensity layers (0.0 to 1.0).
- `forge_generate_terrain({ recipe, bake_path, add_to_scene })`: Generates procedural heightmap terrain.
- `forge_generate_dungeon({ recipe, add_to_scene })`: Generates procedural grid dungeon.
- `forge_scatter_props({ recipe, positions })`: Poisson-disc prop scattering (presets: `"basalt_pillar"`, `"jagged_rock"`, `"crystal_cluster"`, `"sci_fi_crate"`, or custom `.glb`).
- `forge_create_rigged_character({ preset, id_prefix, position })`: Spawns modular rigged character.
- `forge_spawn_vfx({ preset, recipe, position, attach_to_entity })`: Spawns or attaches procedural VFX emitter.
- `forge_create_voxel_world({ width_chunks, depth_chunks, seed, terrain_type, name })`: Generates procedural voxel world (16x32x16 chunks) with culling, AO, and pixel atlas.
- `forge_spawn_controller({ controller_type, entity_id, position, voxel_collision, speed, jump_force, overwrite })`: Generates ready-to-play controller template script and spawns player entity.
- `export_game({ title, scene_path, output_dir, fullscreen })`: 1-Click standalone exporter with smart asset dependency harvesting, `game_config.json`, and executable bundle.
- `spawn_prefab({ name, position })`: Instantiates `.prefab.json` or `.json` prefab hierarchy with root and children.
- `save_prefab({ entity_id, path })`: Exports entity hierarchy to `assets/prefabs/<name>.prefab.json`.
- `find_path({ from, to, max_steps })`: Computes 3D A* path between coordinates.
- Editor Shortcuts: `Ctrl+Z` (Undo), `Ctrl+Y`/`Ctrl+Shift+Z` (Redo), `Ctrl+S` (Save), `W`/`E`/`R` (Gizmo), `🧲 Snap` (0.25-2.0m, 15°-90°).
- Auth: Optional `--mcp-token <TOKEN>` (or `MOTOR_MCP_TOKEN`), requires `Authorization: Bearer <TOKEN>` header.
