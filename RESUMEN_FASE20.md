# Motor-dif — Fase 20: VFX Forge (Sistema de Efectos Visuales Procedurales)

## 1. Resumen Ejecutivo

La **Fase 20: VFX Forge** dota a **Motor-dif** de un motor completo, ligero y desacoplado de efectos visuales procedurales basados en partículas aceleradas por hardware (*GPU instancing*). 

Diseñado bajo la filosofía de máxima potencia con cero dependencias pesadas, **VFX Forge** permite a desarrolladores humanos y agentes autónomos de IA componer, simular y desplegar fenómenos visuales cinemáticos (explosiones, fuego continuo, vórtices de portales místicos, chispas de impacto y auras de subida de nivel) con física 3D en tiempo real (gravedad, fricción y campos vectoriales helicoidales) y respuesta directa en el pipeline HDR Bloom.

![Showcase VFX Forge](viewport_fase20_vfx.png)

---

## 2. Arquitectura de Recetas VFX (`src/vfx_forge.rs`)

El núcleo del sistema reside en el formato de recetas serializables `VfxRecipe`, compatible con JSON y configurable en caliente sin recompilar el motor:

```rust
pub struct VfxRecipe {
    pub name: String,
    pub emitter_shape: String,      // "point", "sphere", "ring", "cone"
    pub shape_radius: f32,
    pub burst_count: u32,
    pub rate_per_sec: f32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub direction: [f32; 3],
    pub spread_deg: f32,
    pub size_start: f32,
    pub size_end: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub emissive_boost: f32,        // Inyección HDR para Bloom
    pub gravity: [f32; 3],          // Aceleración 3D (m/s^2)
    pub drag: f32,                  // Fricción / resistencia aerodinámica
    pub vortex_strength: f32,       // Velocidad angular tangencial XZ (rad/s)
}
```

### Muestreo Determinista de Formas de Emisión
La función `sample_particle_spawn(&VfxRecipe, origin, seed)` garantiza una distribución geométrica uniforme y robusta contra valores `NaN`:
- **Point:** Emisión puntual desde el origen exacto.
- **Sphere:** Distribución esférica uniforme usando coordenadas polares con corrección de latitud (`cos(phi)`).
- **Ring:** Distribución en anillo perimetral sobre el plano XZ.
- **Cone:** Distribución cónica directriz orientada en `direction` con apertura angular controlada por `spread_deg`.

---

## 3. Presets Nativos de Alta Calidad

Motor-dif incluye 5 presets afinados para cubrir los arquetipos visuales más comunes en videojuegos 3D:

| Preset | Forma | Emisión | Vida (s) | Velocidad | Gravedad | Vórtice | Color Inicial $\rightarrow$ Final | Emissive Boost |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`explosion`** | `sphere` | 90 burst | 0.4 – 1.0 | 4.0 – 9.0 | $[0, -9.8, 0]$ | 0.0 | Naranja brillante $\rightarrow$ Humo grisáceo | 2.5 (Super-Bloom) |
| **`fire`** | `cone` | 40/s cont. | 0.4 – 0.9 | 1.5 – 3.2 | $[0, 1.2, 0]$ | 0.3 | Amarillo fuego $\rightarrow$ Rojo ceniza | 1.8 (Glow cálido) |
| **`magic_portal`**| `ring` | 55/s cont. | 1.2 – 2.4 | 0.8 – 2.0 | $[0, 0.4, 0]$ | **3.5** | Cyan místico $\rightarrow$ Púrpura cósmico | 2.2 (Bloom etéreo) |
| **`hit_spark`** | `point` | 30 burst | 0.15 – 0.4| 6.0 – 13.0| $[0, -12.0, 0]$| 0.0 | Blanco deslumbrante $\rightarrow$ Naranja | 3.5 (Flash HDR) |
| **`level_up`** | `ring` | 70 burst | 1.2 – 2.2 | 1.5 – 3.8 | $[0, 1.5, 0]$ | **5.0** | Dorado solar $\rightarrow$ Verde esmeralda | 2.0 (Aura ascendente) |

---

## 4. Física y GPU Instancing (`src/particles.rs`)

### 1. Dinámica de Fuerzas por Partícula
En cada frame de simulación:
1. **Resistencia de Arrastre (Drag):** $\vec{v} \leftarrow \vec{v} \times \max(0, 1 - \text{drag} \cdot \Delta t)$.
2. **Gravedad Vectorial 3D:** $\vec{v} \leftarrow \vec{v} + \vec{g} \cdot \Delta t$.
3. **Fuerza de Vórtice Helicoidal (XZ):**
   $$\vec{r}_{xz} = (p_x - c_x, p_z - c_z)$$
   $$\vec{v}_{\text{tan}} = (-\hat{r}_z, \hat{r}_x) \cdot \omega_{\text{vortex}}$$
   $$\vec{v} \leftarrow \vec{v} + \vec{v}_{\text{tan}} \cdot \Delta t$$
4. **Posición:** $\vec{p} \leftarrow \vec{p} + \vec{v} \cdot \Delta t$.

### 2. GPU Instancing y Activación HDR Bloom
Las partículas se dibujan en una única llamada de dibujado (*instanced quad batching*). El buffer de instancias interpola:
- **Escala:** $\text{size}(t) = \text{lerp}(\text{size\_start}, \text{size\_end}, t)$.
- **Color y Transparencia:** $\text{color}(t) = \text{lerp}(\text{color\_start}, \text{color\_end}, t) \times \text{alpha\_fade}$.
- **HDR Bloom Boost:** Inyectado a través del campo `extra[0]` de `InstanceRaw`, sobrepasando el umbral de luminancia del shader de Bloom ($> 0.8$) sin alterar la opacidad física.

---

## 5. Componentes ECS y Persistencia en Escena (`src/assets.rs`)

Las entidades en escena pueden portar emisores continuos de partículas de forma nativa:

```json
{
  "id": "portal_emitter",
  "mesh": "sphere",
  "position": [0.0, 3.2, -4.5],
  "vfx_emitter": {
    "preset_or_recipe": "magic_portal",
    "enabled": true,
    "looping": true,
    "offset": [0.0, 0.0, 0.0]
  }
}
```

Al cargarse la escena, el motor instancia automáticamente el componente `VfxEmitterComponent` en el ECS (`hecs`), actualizándolo en el bucle principal de forma paralela.

---

## 6. API de Scripting Lua (`src/scripting.rs`)

El motor expone funciones globales seguras en Lua 5.4:

```lua
-- Disparar un efecto puntual en coordenadas del mundo
spawn_vfx("explosion", -4.0, 1.0, 0.0)

-- Adjuntar un emisor persistente a una entidad existente
attach_vfx("player", "fire", 0.0, 1.2, 0.0)

-- Detener el emisor de una entidad
stop_vfx("player")
```

---

## 7. Herramienta MCP para Agentes de IA (`src/mcp_server.rs`)

El servidor MCP expone la herramienta `forge_spawn_vfx` para que asistentes y agentes externos generen efectos visuales en caliente sobre el cliente de juego:

- **Herramienta:** `forge_spawn_vfx`
- **Argumentos:**
  - `preset`: Nombre del preset (`"explosion"`, `"fire"`, `"magic_portal"`, `"hit_spark"`, `"level_up"`) o JSON serializado de `VfxRecipe`.
  - `x`, `y`, `z`: Coordenadas mundiales de emisión.
  - `count`: (Opcional) Número personalizado de partículas.

**Ejemplo de Petición JSON-RPC:**
```json
{
  "jsonrpc": "2.0",
  "id": 202,
  "method": "tools/call",
  "params": {
    "name": "forge_spawn_vfx",
    "arguments": {
      "preset": "explosion",
      "x": 0.0,
      "y": 2.0,
      "z": 0.0
    }
  }
}
```

---

## 8. Interfaz Gráfica de Usuario (Egui)

1. **Ventana VFX Forge (`[File] -> [V] VFX Forge...`):**
   - Selector dinámico de presets.
   - Sliders de forma geométrica, radio, recuento de ráfaga y tasa continua por segundo.
   - Rango de tiempo de vida y velocidad.
   - Controles de física: gravedad $X/Y/Z$, fricción y fuerza de vórtice.
   - Editores de color inicial y final RGBA con control de `emissive_boost`.
   - Botones rápidos: `[Spawn at Origin]`, `[Spawn at Target]`, `[Attach to Selected]`, `[Detach from Selected]`.
2. **Inspector de Entidades:**
   - Sección colapsable `[V] VFX Emitter` para inspeccionar y conmutar en vivo el emisor de cualquier entidad seleccionada.
3. **Command Palette (`Ctrl+K` / `Ctrl+P`):**
   - 5 acciones rápidas para invocar cualquier preset instantáneamente en la posición del cursor/cámara.

---

## 9. Métricas de Rendimiento y Verificación

Telemetría registrada en ejecución en vivo con la escena `assets/showcase_v20_vfx.json` y el script `assets/scripts/vfx_showcase.lua`:

| Métrica | Valor Medido | Objetivo | Estado |
| :--- | :--- | :--- | :--- |
| **FPS en Gráficos Integrados** | **75.15 FPS** | $\ge 60.0\text{ FPS}$ | **SUPERADO (+25%)** |
| **Tiempo de Frame** | **13.31 ms** | $< 16.6\text{ ms}$ | **ÓPTIMO** |
| **Draw Calls Totales** | **4 Draw Calls** | $< 50$ | **ULTRA-LIGERO** |
| **Partículas Activas Simultáneas** | **137 partículas** | Escena viva | **ESTABLE (0 alocaciones)** |
| **Entidades en Escena** | 17 entidades | - | Nominal |
| **Test Suite Automatizado** | **84 / 84 tests pasados** | $100\%$ | **PASS** |

---

## 10. Conclusión y Estado del Ecosistema

Con la **Fase 20 (VFX Forge)** concluida con éxito, **Motor-dif** dispone ahora de una suite integral de generación procedural que abarca:
- Modelado 3D por SDF y Primitivas (`Asset Forge`)
- Generación de Texturas PBR Procedurales (`Texture Forge`)
- Síntesis y Composición Musical Dinámica (`Music Forge`)
- Generación de Terrenos y Niveles (`Level Forge`)
- Animación Procedural y Rigging Modular (`Procedural Rig`)
- Efectos Visuales e Iluminación Cinemática HDR (`VFX Forge`)
