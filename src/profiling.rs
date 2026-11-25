//! Frame timing histograms and an in-engine benchmark harness.
//!
//! The engine is a binary-only crate, so benchmarks cannot live in a separate
//! Criterion `[[bench]]` target. They are exposed through MCP instead, where
//! they run on the engine thread with direct access to the real subsystems.

use std::collections::VecDeque;
use std::time::Instant;

use serde::Serialize;
use serde_json::{json, Value};

use crate::assets::{apply_scene_to_world, extract_scene_from_world, SceneDef};
use crate::camera::Camera;
use crate::ecs::{EntityMeta, MeshRender, Transform};
use crate::nav::NavGrid;
use crate::scripting::json_to_lua;
use crate::voxel_forge::{voxel_raycast, BlockId, VoxelWorld};

pub const HISTORY_CAPACITY: usize = 120;
pub const BENCHMARKS: &[&str] = &[
    "world_transforms",
    "nav_bake",
    "nav_find_path",
    "json_to_lua",
    "scene_apply_extract",
    "voxel_raycast",
];

/// CPU stages of a single frame. The discriminants are used as array indices.
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStage {
    McpHotReload = 0,
    PhysicsSnapshot,
    LuaUpdate,
    TimersEvents,
    ForcesAndMoves,
    PhysicsStep,
    CollisionCallbacks,
    SceneDrains,
    Transforms,
    BatchBuild,
    RenderSubmit,
    Frame,
}

impl FrameStage {
    pub const COUNT: usize = FrameStage::Frame as usize + 1;

    pub const ALL: &'static [FrameStage] = &[
        FrameStage::McpHotReload,
        FrameStage::PhysicsSnapshot,
        FrameStage::LuaUpdate,
        FrameStage::TimersEvents,
        FrameStage::ForcesAndMoves,
        FrameStage::PhysicsStep,
        FrameStage::CollisionCallbacks,
        FrameStage::SceneDrains,
        FrameStage::Transforms,
        FrameStage::BatchBuild,
        FrameStage::RenderSubmit,
        FrameStage::Frame,
    ];

    pub fn name(self) -> &'static str {
        match self {
            FrameStage::McpHotReload => "mcp_hot_reload",
            FrameStage::PhysicsSnapshot => "physics_snapshot",
            FrameStage::LuaUpdate => "lua_update",
            FrameStage::TimersEvents => "timers_events",
            FrameStage::ForcesAndMoves => "forces_and_moves",
            FrameStage::PhysicsStep => "physics_step",
            FrameStage::CollisionCallbacks => "collision_callbacks",
            FrameStage::SceneDrains => "scene_drains",
            FrameStage::Transforms => "transforms",
            FrameStage::BatchBuild => "batch_build",
            FrameStage::RenderSubmit => "render_submit",
            FrameStage::Frame => "frame",
        }
    }

    /// Compact label for the F3 bar chart.
    pub fn short_name(self) -> &'static str {
        match self {
            FrameStage::McpHotReload => "mcp",
            FrameStage::PhysicsSnapshot => "snap",
            FrameStage::LuaUpdate => "lua",
            FrameStage::TimersEvents => "timer",
            FrameStage::ForcesAndMoves => "force",
            FrameStage::PhysicsStep => "phys",
            FrameStage::CollisionCallbacks => "col",
            FrameStage::SceneDrains => "drain",
            FrameStage::Transforms => "xform",
            FrameStage::BatchBuild => "batch",
            FrameStage::RenderSubmit => "submit",
            FrameStage::Frame => "frame",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct SampleStats {
    pub count: usize,
    pub last_ms: f32,
    pub min_ms: f32,
    pub avg_ms: f32,
    pub p50_ms: f32,
    pub p95_ms: f32,
    pub p99_ms: f32,
    pub max_ms: f32,
}

/// Rolling per-stage timings for the last [`HISTORY_CAPACITY`] frames.
pub struct FrameMetrics {
    history: [VecDeque<f32>; FrameStage::COUNT],
    last: [f32; FrameStage::COUNT],
    pending: [Option<Instant>; FrameStage::COUNT],
}

impl Default for FrameMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameMetrics {
    pub fn new() -> Self {
        Self {
            history: std::array::from_fn(|_| VecDeque::with_capacity(HISTORY_CAPACITY)),
            last: [0.0; FrameStage::COUNT],
            pending: [None; FrameStage::COUNT],
        }
    }

    pub fn begin(&mut self, stage: FrameStage) {
        self.pending[stage as usize] = Some(Instant::now());
    }

    pub fn end(&mut self, stage: FrameStage) {
        let idx = stage as usize;
        if let Some(start) = self.pending[idx].take() {
            self.record(stage, start.elapsed().as_secs_f32() * 1000.0);
        }
    }

    pub fn record(&mut self, stage: FrameStage, ms: f32) {
        let idx = stage as usize;
        self.last[idx] = ms;
        let history = &mut self.history[idx];
        history.push_back(ms);
        if history.len() > HISTORY_CAPACITY {
            history.pop_front();
        }
    }

    pub fn last(&self, stage: FrameStage) -> f32 {
        self.last[stage as usize]
    }

    pub fn stats(&self, stage: FrameStage) -> Option<SampleStats> {
        stats_from_iter(self.history[stage as usize].iter())
    }

    pub fn clear(&mut self) {
        for history in &mut self.history {
            history.clear();
        }
        self.last.fill(0.0);
        self.pending.fill(None);
    }

    /// Machine-readable summary for `get_frame_metrics`.
    pub fn summary_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        for &stage in FrameStage::ALL {
            if let Some(stats) = self.stats(stage) {
                map.insert(stage.name().to_string(), json!(stats));
            }
        }
        Value::Object(map)
    }
}

/// Linear-interpolated percentile of an already sorted slice (0.0..=1.0).
pub fn percentile(sorted: &[f32], q: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let clamped = q.clamp(0.0, 1.0);
    let pos = clamped * (sorted.len() - 1) as f32;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(sorted.len() - 1);
    let t = pos - lo as f32;
    sorted[lo] + (sorted[hi] - sorted[lo]) * t
}

pub fn stats_from_iter<'a, I>(samples: I) -> Option<SampleStats>
where
    I: IntoIterator<Item = &'a f32>,
    I::IntoIter: ExactSizeIterator,
{
    let iter = samples.into_iter();
    let len = iter.len();
    if len == 0 {
        return None;
    }
    let values: Vec<f32> = iter.copied().collect();
    let mut sorted = values.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let sum: f32 = values.iter().sum();
    Some(SampleStats {
        count: len,
        last_ms: values[len - 1],
        min_ms: sorted[0],
        avg_ms: sum / len as f32,
        p50_ms: percentile(&sorted, 0.50),
        p95_ms: percentile(&sorted, 0.95),
        p99_ms: percentile(&sorted, 0.99),
        max_ms: sorted[len - 1],
    })
}

pub fn stats_from_samples(samples: &[f32]) -> Option<SampleStats> {
    stats_from_iter(samples.iter())
}

fn measure<F: FnMut()>(iterations: usize, mut op: F) -> Vec<f32> {
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        op();
        samples.push(start.elapsed().as_secs_f32() * 1000.0);
    }
    samples
}

fn bench_world(count: usize) -> hecs::World {
    let mut world = hecs::World::new();
    for i in 0..count {
        let id = if i % 7 == 0 {
            format!("wall_{i}")
        } else {
            format!("prop_{i}")
        };
        let x = ((i % 64) as f32 - 32.0) * 0.75;
        let z = ((i / 64) as f32 - 32.0) * 0.75;
        world.spawn((
            EntityMeta::new(id.clone(), id),
            Transform::from_xyz(x, 0.5, z),
            MeshRender::new("cube", [0.7, 0.7, 0.7, 1.0], 0.0),
        ));
    }
    world
}

fn bench_scene(count: usize) -> SceneDef {
    let mut scene = SceneDef::default();
    let template = scene
        .entities
        .first()
        .cloned()
        .expect("default scene must contain a template entity");
    scene.entities.clear();
    for i in 0..count {
        let mut entity = template.clone();
        entity.id = format!("bench_{i}");
        entity.parent = None;
        entity.position = [((i % 32) as f32) * 0.5, 0.0, ((i / 32) as f32) * 0.5];
        scene.entities.push(entity);
    }
    scene
}

fn bench_json() -> Value {
    json!({
        "kind": "benchmark",
        "version": 3,
        "name": "motor-dif",
        "flags": [true, false, true, false],
        "values": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        "nested": {
            "camera": { "position": [0.0, 3.5, 9.0], "target": [0.0, 0.0, 0.0] },
            "entities": [
                { "id": "a", "scale": [1.0, 1.0, 1.0] },
                { "id": "b", "scale": [2.0, 2.0, 2.0] }
            ]
        }
    })
}

fn bench_result(name: &str, iterations: usize, samples: &[f32]) -> Value {
    json!({
        "name": name,
        "iterations": iterations,
        "stats": stats_from_samples(samples),
    })
}

/// Runs a deterministic micro-benchmark on the engine thread.
///
/// `iterations` is clamped to 1..=1000. `"all"` runs every registered bench.
pub fn run_benchmark(name: &str, iterations: usize) -> Result<Value, String> {
    let iterations = iterations.clamp(1, 1000);
    let name = name.trim().to_ascii_lowercase();

    if name == "all" {
        let mut results = serde_json::Map::new();
        for &bench in BENCHMARKS {
            results.insert(bench.to_string(), run_benchmark(bench, iterations)?);
        }
        return Ok(Value::Object(results));
    }

    let samples = match name.as_str() {
        "world_transforms" => {
            let world = bench_world(512);
            measure(iterations, || {
                let _ = crate::compute_world_transforms(&world);
            })
        }
        "nav_bake" => {
            let world = bench_world(512);
            let mut grid = NavGrid::new(0.5, 32.0);
            measure(iterations, || {
                grid.bake_from_world_opts(&world, 0.3, Some(2.0));
            })
        }
        "nav_find_path" => {
            let world = bench_world(512);
            let mut grid = NavGrid::new(0.5, 32.0);
            grid.bake_from_world_opts(&world, 0.3, Some(2.0));
            measure(iterations, || {
                let _ = grid.find_path_budgeted(-20.0, -20.0, 20.0, 20.0, 2000);
            })
        }
        "json_to_lua" => {
            let lua = mlua::Lua::new();
            let value = bench_json();
            measure(iterations, || {
                let _ = json_to_lua(&lua, &value);
            })
        }
        "scene_apply_extract" => {
            let scene = bench_scene(256);
            let mut world = hecs::World::new();
            let mut camera =
                Camera::new_3d(glam::Vec3::new(0.0, 3.5, 9.0), glam::Vec3::ZERO, 16.0 / 9.0);
            let mut physics = crate::physics::PhysicsEngine::new();
            measure(iterations, || {
                apply_scene_to_world(&scene, &mut world, &mut camera, &mut physics, None);
                let _ = extract_scene_from_world(&world, &camera, &physics);
            })
        }
        "voxel_raycast" => {
            let mut voxel_world = VoxelWorld::new();
            for x in 8..24 {
                voxel_world.set_block(x, 8, 0, BlockId::Stone);
            }
            measure(iterations, || {
                let _ = voxel_raycast(&voxel_world, 0.0, 8.5, 0.0, 1.0, 0.0, 0.0, 32.0);
            })
        }
        _ => {
            return Err(format!(
                "unknown benchmark '{name}'. Available: {}",
                BENCHMARKS.join(", ")
            ));
        }
    };

    Ok(bench_result(&name, iterations, &samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_handles_empty_single_and_interpolated() {
        assert_eq!(percentile(&[], 0.5), 0.0);
        assert_eq!(percentile(&[4.0], 0.95), 4.0);
        assert_eq!(percentile(&[0.0, 10.0], 0.5), 5.0);
        assert_eq!(percentile(&[0.0, 10.0], 0.25), 2.5);
    }

    #[test]
    fn frame_metrics_keeps_rolling_history_and_stats() {
        let mut metrics = FrameMetrics::new();
        for i in 0..HISTORY_CAPACITY + 10 {
            metrics.record(FrameStage::LuaUpdate, i as f32);
        }
        let stats = metrics.stats(FrameStage::LuaUpdate).expect("stats");
        assert_eq!(stats.count, HISTORY_CAPACITY);
        assert_eq!(stats.min_ms, 10.0);
        assert_eq!(stats.max_ms, HISTORY_CAPACITY as f32 + 9.0);
        assert!(stats.p95_ms >= stats.p50_ms);
        metrics.clear();
        assert!(metrics.stats(FrameStage::LuaUpdate).is_none());
    }

    #[test]
    fn benchmark_unknown_name_reports_available_benches() {
        let err = run_benchmark("does_not_exist", 1).unwrap_err();
        assert!(err.contains("world_transforms"));
    }

    #[test]
    fn benchmark_world_transforms_runs() {
        let result = run_benchmark("world_transforms", 2).expect("bench runs");
        assert_eq!(result["name"], "world_transforms");
        assert_eq!(result["iterations"], 2);
        assert!(result["stats"]["count"].as_u64().unwrap() >= 2);
    }

    /// Run with `-- --nocapture` to print a baseline sweep before optimizing.
    #[test]
    fn benchmark_all_reports_stats() {
        let result = run_benchmark("all", 5).expect("all benchmarks run");
        let map = result.as_object().expect("benchmark map");
        for &name in BENCHMARKS {
            let stats = map.get(name).unwrap_or_else(|| panic!("missing {name}"));
            eprintln!(
                "BASELINE {name}: p50={:?}ms p95={:?}ms avg={:?}ms",
                stats["stats"]["p50_ms"], stats["stats"]["p95_ms"], stats["stats"]["avg_ms"],
            );
        }
    }
}
