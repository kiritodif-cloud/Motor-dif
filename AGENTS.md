# Instructions for AI Agents Working on Motor-dif

Before inspecting engine Rust source code to write Lua scripts, author scenes, or build gameplay:
1. Consult the MCP tool `get_engine_docs` (or read `AGENT_GUIDE.md`). It contains compact, verified API signatures and schemas for all engine subsystems (`lua_core`, `lua_ui`, `lua_vfx_audio`, `lua_ai_rig`, `scene_json`).
2. Use procedural forges (`forge_create_model`, `forge_create_texture`, `forge_create_music`, `forge_generate_terrain`, `forge_generate_dungeon`, `forge_spawn_vfx`) for instant asset generation.
3. Rapid iteration loop via MCP at `http://127.0.0.1:3030/mcp`:
   - Edit or generate scenes in `assets/` and scripts in `assets/scripts/`.
   - Control runtime mode: `set_play_mode({ mode: "play" })`.
   - Verify execution and catch errors: `get_console_logs({ filter: "error" })` or `get_engine_logs`.
   - Inspect visually: `capture_viewport` (returns Base64 PNG).
