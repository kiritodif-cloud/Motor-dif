# 🎵 RESUMEN FASE 17: MUSIC FORGE — COMPOSITOR PROCEDURAL Y MÚSICA ADAPTATIVA
**Motor-dif — Motor Gráfico y de Audio 3D Ligero en Rust + WGPU + Rodio**  
*Fecha: 23 de Noviembre de 2026*  
*Estado:* **100% COMPLETADO — 63 TESTS UNITARIOS PASANDO — 0 ERRORES**

---

## 🌟 Visión General

**Music Forge** dota a **Motor-dif** de un motor completo de síntesis musical procedural y música interactiva adaptativa en tiempo real, desarrollado en 100% Rust puro sobre `rodio` (PCM 16-bit / 44.1 kHz, sin VSTs, plugins ni DAWs pesados externos). Permite tanto a desarrolladores humanos como a agentes de IA componer bandas sonoras dinámicas, libres de derechos y de tamaño microscópico (< 2 KB por receta JSON), capaces de reaccionar orgánicamente a la intensidad del juego (desde exploración tranquila hasta batallas intensas contra jefes).

---

## 🎼 1. Esquema JSON de Composición (`MusicRecipe`)

El archivo de partitura `MusicRecipe` se serializa en JSON y se divide en propiedades globales y 4 pistas de instrumentos independientes:

```json
{
  "name": "space_ambient",
  "bpm": 90.0,
  "bars": 4,
  "root_midi": 57,
  "scale": "dorian",
  "chord_progression": [0, 2, 4, 3],
  "tracks": {
    "pad": {
      "enabled": true,
      "waveform": "sine",
      "volume": 0.75,
      "detune_cents": 8.0,
      "octave": 0
    },
    "bass": {
      "enabled": true,
      "waveform": "triangle",
      "volume": 0.60,
      "cutoff_hz": 320.0,
      "pattern": [1, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0],
      "octave": -2
    },
    "lead": {
      "enabled": true,
      "waveform": "sine",
      "volume": 0.45,
      "pattern": [0, -1, 2, -1, 4, -1, 2, -1, 5, -1, 4, -1, 2, -1, 0, -1],
      "octave": 1
    },
    "drums": {
      "enabled": true,
      "volume": 0.50,
      "kick_pattern":  [1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
      "snare_pattern": [0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
      "hihat_pattern": [1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0]
    }
  }
}
```

### Escalas Soportadas:
- `"minor"` (Menor natural / Eólica: `[0, 2, 3, 5, 7, 8, 10]`)
- `"major"` (Mayor / Jónica: `[0, 2, 4, 5, 7, 9, 11]`)
- `"pentatonic"` (Pentatónica menor: `[0, 3, 5, 7, 10]`)
- `"dorian"` (Dórica espacial: `[0, 2, 3, 5, 7, 9, 10]`)
- `"phrygian"` (Frigia oscura / combate épico: `[0, 1, 3, 5, 7, 8, 10]`)

---

## 🎹 2. Presets Integrados (`get_music_preset(name)`)

1. **`"space_ambient"`** (90 BPM, Dórica en A3):  
   Atmósfera envolvente profunda con acordes senoidales con coro (*chorus/detune*), sub-bajo triangular y percusión sutil. Ideal para exploración espacial en Cosmos.io 3D.
2. **`"boss_battle"`** (138 BPM, Frigia en E3):  
   Línea de bajo agresiva en onda de sierra con galope en semicorcheas, arpegio rápido tenso y batería frenética. Diseñado para el encuentro con Apophis Leviathan.
3. **`"cyber_action"`** (125 BPM, Menor en A3):  
   Línea synthwave rodante con bajo en octavas, melodía de gancho en onda cuadrada y bombo continuo en cuatro cuartos.
4. **`"menu_chill"`** (85 BPM, Pentatónica en C4):  
   Ambiente relajado y limpio con acordes cálidos, bajo sub y percusiones mínimas. Ideal para menús y pausas.

---

## 🎚️ 3. Sistema de Música Adaptativa en Tiempo Real (`AdaptiveMusicSource`)

- **Síntesis y Stems en Memoria:**  
  Genera 4 capas independientes (*pad*, *bass*, *lead*, *drums*) renderizadas con bucle perfecto (crossfade de 80 muestras en las fronteras para **0 clics**).
- **Control de Intensidad (`0.0` a `1.0`):**  
  Almacenado de forma no bloqueante en un `Arc<AtomicU32>` con filtro paso-bajo de suavizado continuo (~50 ms) para evitar ruidos de cremallera (*zipper noise*):
  - **`0.0 - 0.35` (Calma / Exploración):** Solo suena el *Pad* etéreo.
  - **`0.35 - 0.70` (Tensión / Acercamiento):** Entran progresivamente el *Bass* y el *Lead*.
  - **`0.70 - 1.00` (Combate / Clímax):** Entra la sección completa de *Drums* y máxima apertura espectral.
- **Conectado al Bus `"music"`:**  
  Hereda automáticamente el control de volumen del bus `"music"` y el bus global `"master"` de `AudioManager`.

---

## 📜 4. API de Lua (`src/scripting.rs`)

| Función Lua | Descripción |
| :--- | :--- |
| `play_music_preset("preset_name")` | Inicia en bucle infinito adaptativo uno de los 4 presets (`"space_ambient"`, `"boss_battle"`, etc.). |
| `play_music_recipe(json_o_ruta)` | Carga una receta JSON (en cadena o archivo `.json`) y la reproduce adaptativamente. |
| `set_music_intensity(0.0..1.0)` | Ajusta en vivo la intensidad musical con transición suave sin cortes. |
| `bake_music(preset_o_json, out_path)`| Hornea la música directamente a un archivo `.wav` PCM estándar (44.1 kHz, 16-bit). |
| `stop_music()` | Detiene limpiamente la reproducción de música y libera los recursos. |

---

## 🤖 5. Herramientas MCP (`src/mcp_server.rs`)

Servidor expuesto en `127.0.0.1:3030/mcp`:
1. **`forge_create_music`:**
   - Parámetros:
     - `preset: Option<String>`
     - `recipe: Option<MusicRecipe>`
     - `bake_path: Option<String>` (ej. `"assets/music/battle.wav"`)
     - `play_now: bool`
     - `intensity: Option<f32>`
   - Retorna: metadatos del tema (`recipe_name`, `bpm`, `bars`, `scale`, `baked_path`, `playing`).
2. **`set_music_intensity`:**
   - Parámetros: `intensity: f32` (`0.0` a `1.0`).
   - Retorna: confirmación con el valor ajustado.

---

## 🖥️ 6. Interfaz del Editor en `egui`

Accesible desde:
- Menú superior: **`File -> 🎵 Music Forge...`**
- Command Palette (`Ctrl+P` / `>`): **`Music Forge (Procedural Music)`**

**Características del panel:**
- Desplegable interactivo para alternar entre presets.
- Sliders de **BPM** (60 a 180) e **Intensidad adaptativa** (0.0 a 1.0) que modulan el audio en vivo al deslizar.
- Visualización de la escala armónica y compases.
- Botones **▶ Play Adaptive**, **⏹ Stop** y **Bake to assets/music/...** con toasts visuales de estado.

---

## 🧪 7. Verificación Práctica y Pruebas

- **Archivo horneado de verificación:** `assets/music/cosmos_theme.wav` (940 KB, 44.1 kHz, 16-bit PCM, bucle limpio).
- **Tests Unitarios (63 pasando, 0 fallidos):**
  - `music_forge::tests::test_music_presets_generation_and_no_nan`
  - `music_forge::tests::test_bake_music_to_wav_valid_file`
  - `music_forge::tests::test_adaptive_music_source_intensity_levels`
  - `music_forge::tests::test_bake_cosmos_theme_wav`
  - `scripting::tests::test_music_forge_lua_bindings`
  - `mcp_server::tests::test_mcp_forge_tools_schema_and_commands`
- **Binario compilado:** `target/release/motor_dif.exe` (21.7 MB, 60+ FPS estables).
