# Motor-dif — Agent & LLM Quick Reference Guide

## quickstart
- Execution lifecycle:
  - `on_update(entity, dt)`: Called each frame during `PlayMode::Play`. `entity.x/y/z` (pos), `entity.rx/ry/rz` (rot deg), `entity.sx/sy/sz` (scale). Method: `entity:apply_force(fx,fy,fz)`, `entity:apply_impulse(ix,iy,iz)`.
  - Init idiom: `local init=false; function on_update(e,dt) if not init then init=true; on_init(e) end ... end`.
  - `on_collision(entity, other_id)`: Contact callback for physical bodies.
  - `on_trigger_enter(other_id, entity)`: Overlap callback for sensor colliders (`is_sensor = true`).
  - Button callbacks: Define `function my_click() ... end` globally; referenced by name in `ui_button`.
- Editor vs Play Mode:
  - In Editor mode (`play_mode == Edit`), scripts and game audio DO NOT run.
  - Scripts and adaptive audio execute only when `> Play` is triggered or when running standalone (`--play`).
  - Switching scenes or pressing `Stop` automatically stops music and cleans timers/events.
- Cleanup rule: When resetting game state in scripts, always call `ui_clear()`, `clear_timers()`, and `clear_events()`.

## scene_json
```json
{
  "camera": { "mode": "3d", "position": [0, 5, 12], "target": [0, 1, 0], "ortho_size": 10.0 },
  "post_process": { "bloom_enabled": true, "bloom_intensity": 0.8, "bloom_threshold": 0.8, "vignette_intensity": 0.35, "exposure": 1.0 },
  "environment": { "sky_mode": "gradient", "zenith_color": [0.07, 0.08, 0.12], "horizon_color": [0.13, 0.15, 0.21], "fog_color": [0.07, 0.08, 0.12], "fog_density": 0.005 },
  "entities": [
    {
      "id": "player", "mesh": "cube", "position": [0, 1, 0], "rotation": [0, 0, 0], "scale": [1, 1, 1],
      "color": [0.2, 0.7, 1.0, 1.0], "emissive": 0.5, "fresnel_color": [0.4, 0.8, 1.0], "fresnel_power": 2.5,
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
Meshes: `"cube"`, `"sphere"`, `"cylinder"`, `"quad"`, `"plane"`, `"torus"`, `"ring"`, `"cone"`, `"icosphere"`, or `"assets/models/*.glb"`.
Blend modes: `"opaque"`, `"additive"`.
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
  - `set_light(id, r, g, b, intensity, radius)`: Updates/creates PointLight component.
  - `tween_position(id, x, y, z, duration)`: Smooth linear interpolation of position.
  - `tween_scale(id, sx, sy, sz, duration)`: Smooth scale interpolation.
  - `set_sprite_frame(id, cols, rows, frame_idx)`: 2D sprite sheet tile selection.
  - `load_scene(path)`: Queues scene transition on frame boundary.
  - `save_data(key, val)`, `load_data(key) -> val`: JSON persistence to `assets/save_data.json`.
- Camera & Input:
  - `set_camera(x,y,z, tx,ty,tz)`, `follow_entity(id, ox,oy,oz, smoothness)`
  - `camera_shake(intensity, duration)`, `camera_lerp_to(ex,ey,ez, tx,ty,tz, duration)`
  - `camera_orbit_target(id, distance, pitch_deg, yaw_speed)`
  - `get_mouse_world_pos([plane_y]) -> { x, y, z }`, `get_mouse_delta() -> { dx, dy }`
  - `is_key_down(key) -> bool`, `is_key_just_pressed(key) -> bool` (e.g. `"W"`, `"SPACE"`, `"LSHIFT"`)
  - `is_action_down(act) -> bool`, `is_action_just_pressed(act) -> bool`, `bind_action(act, key)`
- Physics & Raycast:
  - `apply_force(fx, fy, fz)`, `apply_impulse(ix, iy, iz)`: Operates on self entity rigid body.
  - `raycast(ox, oy, oz, dx, dy, dz, max_dist) -> { hit=bool, entity_id=str, point={x,y,z}, normal={x,y,z}, distance=f }|nil`
- Timers & Events:
  - `after(delay_secs, callback_name) -> timer_id`, `every(interval_secs, callback_name) -> timer_id`
  - `cancel_timer(timer_id)`, `clear_timers()`
  - `on_event(name, callback_name)`, `emit_event(name, payload)`, `clear_events()`

## lua_ui
- Declarative In-Game HUD:
  - Anchors: `"top_left"`, `"top_center"`, `"top_right"`, `"center"`, `"bottom_left"`, `"bottom_center"`, `"bottom_right"`.
  - `ui_clear()`: Clears all HUD widgets.
  - `ui_panel(id, anchor, x, y, w, h, r, g, b, a)`: Background container card.
  - `ui_bar(id, anchor, x, y, w, h, value_0_to_1, r, g, b, a)`: Dynamic progress/health bar.
  - `ui_label(id, anchor, x, y, text, size, r, g, b, a)`: Text label (re-call with same ID to update text).
  - `ui_button(id, anchor, x, y, w, h, text, callback_name)`: Interactive button triggering global Lua function.
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
  - `play_sound_3d(path, x, y, z, max_dist, [vol], [bus])`: Attenuated spatial 3D audio.
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
  - `find_path(start_x, start_z, goal_x, goal_z) -> { {x, z}, ... }|nil`: A* path.
  - `nav_set_blocked(wx, wz, is_blocked)`, `nav_is_blocked(wx, wz) -> bool`
- Level Forge:
  - `get_terrain_height(x, z) -> f32`: Evaluates procedural heightmap elevation at world coordinates.
- AnimationPlayer:
  - `play_anim(entity_id, clip_name, [loop])`: Plays clip from `AnimationPlayer`.
  - `play_preset_anim(entity_id, preset_name, [loop])`: Presets: `"idle_breathe"`, `"walk_bob"`, `"spin_continuous"`.
  - `stop_anim(entity_id)`, `is_anim_playing(entity_id) -> bool`
- Procedural Rig (Biomechanical Locomotion & Aiming):
  - `rig_set_look_target(entity_id, x, y, z)`: Head/turret look-at tracking.
  - `rig_clear_look_target(entity_id)`
  - `rig_trigger_recoil(entity_id, strength)`: Weapon kickback impulse with spring decay.
  - `solve_ik_2bone(rx, ry, rz, tx, ty, tz, len1, len2) -> { jx, jy, jz, ... }|nil`: 2-bone analytical IK.

## mcp_workflow
Recommended 4-step autonomous loop for AI agents (`http://127.0.0.1:3030/mcp`):
1. **Query Docs:** Call `get_engine_docs({ topic: "all" })` (or specific topic) to verify signatures without exploring source files.
2. **Author Content:**
   - Write scenes to `assets/<name>.json` and Lua scripts to `assets/scripts/<name>.lua`.
   - Or use procedural forges: `forge_create_model`, `forge_bake_glb`, `forge_create_texture`, `forge_create_music`, `forge_generate_terrain`, `forge_generate_dungeon`, `forge_scatter_props`, `forge_create_rigged_character`, `forge_spawn_vfx`.
3. **Run & Validate:**
   - Call `set_play_mode({ mode: "play" })`.
   - Query `get_console_logs({ filter: "error" })` or `get_engine_logs` to ensure 0 Lua runtime errors.
4. **Visual & Performance Inspection:**
   - Call `capture_viewport` to verify visual composition, materials, and HUD in Base64 PNG.
   - Call `get_performance_stats` to verify 60+ FPS, draw calls, and entity counts.

MCP Tools Directory:
- `get_engine_docs({ topic })`: Compact API signatures and schemas (topics: all, quickstart, scene_json, lua_core, lua_ui, lua_vfx_audio, lua_ai_rig, mcp_workflow).
- `inspect_world()`: Returns active camera and entity definitions JSON.
- `upsert_entity(entity_def)`: Dynamically adds or modifies an entity in active scene.
- `delete_entity({ id })`: Removes entity by ID.
- `execute_lua({ code })`: Runs Lua code immediately in active environment.
- `get_console_logs({ filter })`: Retrieves engine/script log buffer.
- `get_engine_logs()`: Compact diagnostic metrics (FPS, logs, Lua error string).
- `capture_viewport()`: Captures rendered frame as Base64 PNG data.
- `set_play_mode({ mode })`: Sets engine state (`"play"`, `"pause"`, `"edit"`).
- `load_scene({ path })`: Loads `.json` scene from `assets/`.
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
- `forge_scatter_props({ recipe, positions })`: Instanced or individual prop scattering.
- `forge_create_rigged_character({ preset, id_prefix, position })`: Spawns modular rigged character.
- `forge_spawn_vfx({ preset, recipe, position, attach_to_entity })`: Spawns or attaches procedural VFX emitter.
