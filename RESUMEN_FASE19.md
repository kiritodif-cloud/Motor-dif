# RESUMEN EJECUTIVO — FASE 19: AUTO-RIG Y ANIMACIÓN PROCEDURAL

## 1. Visión General y Objetivos Cumplidos

En la **Fase 19**, Motor-dif incorpora un subsistema de **Auto-Rig y Animación Procedural** de alto rendimiento en Rust puro, sin dependencias pesadas de skinning en GPU ni mallas esqueléticas tradicionales. Las animaciones se computan analíticamente en tiempo real sobre las transformaciones locales de la jerarquía padre-hijo del motor ECS (`hecs`), garantizando **60+ FPS continuos (74.91 FPS medidos a 13.35 ms)** en gráficos integrados.

### Logros Principales:
1. **Solver Analítico de Cinemática Inversa (2-Bone IK):** Solución analítica basada en la Ley de Cosenos en $\mathcal{O}(1)$, completamente libre de convergencias iterativas y con protección contra singularidades y `NaN`.
2. **Motor de Locomoción y Fisiología Procedural:**
   - **Bípedos:** Zancada proporcional a la velocidad horizontal real en XZ, balanceo opuesto de brazos y piernas, oscilación vertical de torso (*bobbing*) y respiración sinusoidal en reposo (*idle breathing*).
   - **Cuadrúpedos / Arácnidos:** Trote diagonal coordinado en pares (Front-Left + Back-Right vs Front-Right + Back-Left).
   - **Hover Bots:** Inclinación dinámica de cabeceo (*pitch*) y alabeo (*roll*) proporcional a la aceleración y velocidad.
   - **Torretas de Defensa:** Orientación desacoplada en acimut (yaw) y elevación (pitch) hacia blancos en el espacio 3D.
   - **Head Look-At Tracking:** Orientación suave de cabezas/sensores hacia objetivos tridimensionales arbitrarios.
   - **Retroceso Elástico de Armas:** Decaimiento exponencial y amortiguado ante disparos o impulsos de impacto.
3. **Generador Modular de Personajes:** 4 arquetipos ensamblables en 1 comando (`mech_biped`, `sci_fi_trooper`, `spider_drone`, `defense_turret`).
4. **API de Scripting en Lua:** Control en vivo desde Lua (`rig_set_look_target`, `rig_clear_look_target`, `rig_trigger_recoil`, `solve_ik_2bone`).
5. **Herramienta MCP:** `forge_create_rigged_character` para orquestación directa desde agentes de IA.
6. **Inspección e Interfaz Gráfica (`egui`):** Tarjeta colapsable `[R] Procedural Rig` en el Inspector con deslizadores y disparador de retroceso, submenú de generación en la barra de menú y accesos en la Command Palette (`Ctrl+P`).

---

## 2. Solver Analítico 2-Bone IK (Cinemática Inversa)

El solver cinemático resuelve la posición de la articulación intermedia (rodilla o codo) y las rotaciones de ambos segmentos dadas la raíz $\mathbf{R}$, el efector final $\mathbf{T}$, la dirección guía polar $\mathbf{P}$, y las longitudes de los huesos $L_1$ y $L_2$.

### Ecuación de la Ley de Cosenos:
Dado el vector de alcance $\mathbf{D} = \mathbf{T} - \mathbf{R}$ con distancia efectiva $d = \|\mathbf{D}\|$, acotada por:
$$d_{\text{min}} = |L_1 - L_2| + \epsilon \le d \le (L_1 + L_2) \cdot 0.9999 = d_{\text{max}}$$

El ángulo en la raíz $\alpha$ y en la articulación $\beta$ se obtienen de forma cerrada:
$$\cos \alpha = \frac{L_1^2 + d^2 - L_2^2}{2 L_1 d}, \quad \cos \beta = \frac{L_1^2 + L_2^2 - d^2}{2 L_1 L_2}$$

### Resiliencia Matemática:
- Los valores de entrada a $\arccos$ son clampeados de forma estricta a $[-1.0, 1.0]$.
- Si $d \le 0.001$, la distancia se estabiliza con un vector por defecto para prevenir divisiones por cero.
- La orientación de flexión se proyecta en el plano perpendicular al eje objetivo mediante el producto cruzado con el vector guía $\mathbf{P}$.

---

## 3. Arquitectura de Componentes y Modos de Rig

### Estructura `ProceduralRig`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralRig {
    pub rig_type: String, // "biped" | "quadruped" | "hover_bot" | "turret"
    pub head: Option<String>,
    pub torso: Option<String>,
    pub arm_l: Option<String>,
    pub arm_r: Option<String>,
    pub leg_l: Option<String>,
    pub leg_r: Option<String>,
    pub weapon: Option<String>,
    pub leg_fl: Option<String>,
    pub leg_fr: Option<String>,
    pub leg_bl: Option<String>,
    pub leg_br: Option<String>,

    pub walk_speed_scale: f32,
    pub stride_angle_deg: f32,
    pub bob_amplitude: f32,
    pub breath_speed: f32,
    pub breath_amplitude: f32,

    // Runtime state (no serializado)
    pub phase: f32,
    pub last_pos: [f32; 3],
    pub current_speed: f32,
    pub look_target: Option<[f32; 3]>,
    pub recoil_timer: f32,
    pub recoil_strength: f32,
    pub base_transforms: HashMap<String, Transform>,
}
```

### Prevención de Deriva Flotante (*Zero-Drift*):
Para evitar que las transformaciones locales sufran acumulación de errores de punto flotante por multiplicaciones sucesivas frame a frame, el sistema almacena las transformaciones de reposo iniciales en `base_transforms`. Todas las rotaciones y desplazamientos calculados en cada frame se aplican de forma pura y relativa a la pose base.

---

## 4. Presets Modulares Disponibles

| Preset | Entidades Creadas | Tipo de Rig | Características Fisiológicas |
| :--- | :--- | :--- | :--- |
| `mech_biped` | 8 entidades | `biped` | Zancada contrapesada, pistón vertical de torso, head tracking y cañón de retroceso elástico. |
| `sci_fi_trooper` | 8 entidades | `biped` | Humanoide con extremidades estilizadas, balanceo natural de brazos y rifle acoplado. |
| `spider_drone` | 7 entidades | `quadruped` | Chasis central bajo con 4 patas acodadas en trote diagonal en contrafase ($\Delta\phi = \pi$). |
| `defense_turret` | 4 entidades | `turret` | Anillo base estacionario, cabezal acimutal libre y cañón doble con elevación vertical. |

---

## 5. API de Scripting en Lua y Herramientas MCP

### Funciones Lua Globales:
1. `rig_set_look_target(entity_id, x, y, z)`: Define el blanco al que apuntará la cabeza o sensor del rig.
2. `rig_clear_look_target(entity_id)`: Desactiva el seguimiento cefálico.
3. `rig_trigger_recoil(entity_id, strength)`: Desencadena una sacudida elástica de retroceso en el arma y torso.
4. `solve_ik_2bone(rx, ry, rz, tx, ty, tz, len1, len2) -> table`:
   Retorna `{ jx, jy, jz, pitch1, yaw1, roll1, pitch2, yaw2, roll2 }`.

### Herramienta MCP JSON-RPC:
- **`forge_create_rigged_character`**:
  ```json
  {
    "method": "tools/call",
    "params": {
      "name": "forge_create_rigged_character",
      "arguments": {
        "preset": "mech_biped",
        "id_prefix": "mech_alpha",
        "position": [0.0, 0.0, 0.0]
      }
    }
  }
  ```

---

## 6. Escena de Demostración (`showcase_v19_rig.json`)

La escena combina los 4 presets de personajes modulares en una arena de pruebas iluminada con un faro central holográfico:
- **Titán Bípedo (`mech_root`):** Patrulla activamente en círculo en torno al faro mediante el script `assets/scripts/rig_showcase.lua`. Su cabeza rastrea de forma permanente el núcleo del faro mientras sus cañones descargan impulsos de retroceso periódicos.
- **Dron Arácnido (`spider_root`):** Demostración del trote diagonal cuadrúpedo.
- **Torreta Centinela (`turret_root`):** Rastreo de acimut y pitch.
- **Soldado Cibernético (`trooper_root`):** Pose de inspección táctica con breathing idle.
- **Faro Central de Plasma (`rig_beacon`):** Punto de luz dinámico con anillos giratorios aditivos.

---

## 7. Rendimiento y Verificación Técnica

```
=== Starting Motor-dif with showcase_v19_rig.json ===
Waiting 5 seconds for Procedural Rig engine, locomotion & scripts...
STATS: {
  "fps": 74.9054,
  "frame_time_ms": 13.350172,
  "entities": 32,
  "draw_calls": 5,
  "active_particles": 0,
  "physics_bodies": 0
}
Requesting viewport capture via MCP (http://127.0.0.1:3030/mcp)...
SUCCESS: Saved viewport_fase19_rig.png (463057 bytes)
Terminating motor_dif process...
Port 3030 clean and verified.
```

- **Suite de Pruebas Unitarias:** **77 tests pasados, 0 fallidos** (`cargo test`).
- **Compilación en Release:** 0 errores y 0 warnings bloqueantes (`cargo build --release`).
- **Tasa de Cuadros:** **74.91 FPS**, superando con holgura el objetivo de 60 FPS.
- **Servidor MCP:** Operativo y probado en `127.0.0.1:3030/mcp`.

---

## 8. Captura de Viewport

A continuación se presenta la captura directa renderizada por el motor en modo Release:

![Viewport Fase 19: Auto-Rig y Animación Procedural](file:///C:/Users/difer/.gemini/antigravity/brain/4c43111a-e23d-4652-8bfd-0d250e748e9a/viewport_fase19_rig.png)
