use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Copy, Clone, PartialEq)]
struct NodeState {
    cost: f32,
    heuristic: f32,
    gx: usize,
    gz: usize,
}

impl Eq for NodeState {}

impl Ord for NodeState {
    fn cmp(&self, other: &Self) -> Ordering {
        let f_self = self.cost + self.heuristic;
        let f_other = other.cost + other.heuristic;
        f_other.partial_cmp(&f_self).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for NodeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct NavGrid {
    pub cell_size: f32,
    pub half_extent: f32,
    pub width: usize,
    pub height: usize,
    pub origin_x: f32,
    pub origin_z: f32,
    pub blocked: Vec<bool>,
}

impl Default for NavGrid {
    fn default() -> Self {
        Self::new(1.0, 20.0)
    }
}

impl NavGrid {
    pub fn new(cell_size: f32, half_extent: f32) -> Self {
        let cs = cell_size.max(0.1);
        let extent = half_extent.max(1.0);
        let width = ((extent * 2.0) / cs).ceil() as usize;
        let height = width;
        let origin_x = -extent;
        let origin_z = -extent;
        let total_cells = width * height;
        Self {
            cell_size: cs,
            half_extent: extent,
            width,
            height,
            origin_x,
            origin_z,
            blocked: vec![false; total_cells],
        }
    }

    #[inline]
    pub fn index(&self, gx: usize, gz: usize) -> usize {
        gz * self.width + gx
    }

    #[inline]
    pub fn is_in_bounds(&self, gx: usize, gz: usize) -> bool {
        gx < self.width && gz < self.height
    }

    pub fn world_to_grid(&self, x: f32, z: f32) -> Option<(usize, usize)> {
        let gx = ((x - self.origin_x) / self.cell_size).floor() as isize;
        let gz = ((z - self.origin_z) / self.cell_size).floor() as isize;
        if gx >= 0 && (gx as usize) < self.width && gz >= 0 && (gz as usize) < self.height {
            Some((gx as usize, gz as usize))
        } else {
            None
        }
    }

    pub fn grid_to_world(&self, gx: usize, gz: usize) -> (f32, f32) {
        let x = self.origin_x + (gx as f32 + 0.5) * self.cell_size;
        let z = self.origin_z + (gz as f32 + 0.5) * self.cell_size;
        (x, z)
    }

    pub fn is_blocked(&self, gx: usize, gz: usize) -> bool {
        if self.is_in_bounds(gx, gz) {
            self.blocked[self.index(gx, gz)]
        } else {
            true
        }
    }

    pub fn is_world_blocked(&self, x: f32, z: f32) -> bool {
        if let Some((gx, gz)) = self.world_to_grid(x, z) {
            self.is_blocked(gx, gz)
        } else {
            true
        }
    }

    pub fn set_blocked(&mut self, gx: usize, gz: usize, blocked: bool) {
        if self.is_in_bounds(gx, gz) {
            let idx = self.index(gx, gz);
            self.blocked[idx] = blocked;
        }
    }

    pub fn set_world_blocked(&mut self, x: f32, z: f32, blocked: bool) {
        if let Some((gx, gz)) = self.world_to_grid(x, z) {
            self.set_blocked(gx, gz, blocked);
        }
    }

    pub fn add_obstacle_box(&mut self, min_x: f32, min_z: f32, max_x: f32, max_z: f32) {
        let g_min = self.world_to_grid(min_x, min_z).unwrap_or((0, 0));
        let g_max = self.world_to_grid(max_x, max_z).unwrap_or((self.width - 1, self.height - 1));

        let start_x = g_min.0.min(g_max.0);
        let end_x = g_min.0.max(g_max.0);
        let start_z = g_min.1.min(g_max.1);
        let end_z = g_min.1.max(g_max.1);

        for gz in start_z..=end_z {
            for gx in start_x..=end_x {
                self.set_blocked(gx, gz, true);
            }
        }
    }

    pub fn clear(&mut self) {
        self.blocked.fill(false);
    }

    pub fn bake_from_world(&mut self, world: &hecs::World) {
        self.clear();
        for (meta, transform) in world.query::<(&crate::ecs::EntityMeta, &crate::ecs::Transform)>().iter() {
            let id_low = meta.id.to_lowercase();
            // Ignore ground/floor/plane/player/camera/lights/particles/sky
            if id_low.contains("plane")
                || id_low.contains("ground")
                || id_low.contains("floor")
                || id_low.contains("player")
                || id_low.contains("camera")
                || id_low.contains("light")
                || id_low.contains("trigger")
            {
                continue;
            }

            // Treat any entity identified as wall, pillar, obstacle, box, rock, tree or barrier as a nav obstacle
            let is_obstacle = id_low.contains("wall")
                || id_low.contains("pillar")
                || id_low.contains("obstacle")
                || id_low.contains("barrier")
                || id_low.contains("box")
                || id_low.contains("cube")
                || id_low.contains("rock");

            if is_obstacle {
                let pos = transform.position;
                let half_sx = (transform.scale.x * 0.5).max(0.2);
                let half_sz = (transform.scale.z * 0.5).max(0.2);
                self.add_obstacle_box(
                    pos.x - half_sx,
                    pos.z - half_sz,
                    pos.x + half_sx,
                    pos.z + half_sz,
                );
            }
        }
    }

    pub fn bake_from_entities_map(&mut self, entities: &std::collections::HashMap<String, [f32; 9]>) {
        self.clear();
        for (id, data) in entities {
            let id_low = id.to_lowercase();
            if id_low.contains("plane")
                || id_low.contains("ground")
                || id_low.contains("floor")
                || id_low.contains("player")
                || id_low.contains("camera")
                || id_low.contains("light")
                || id_low.contains("trigger")
            {
                continue;
            }

            let is_obstacle = id_low.contains("wall")
                || id_low.contains("pillar")
                || id_low.contains("obstacle")
                || id_low.contains("barrier")
                || id_low.contains("box")
                || id_low.contains("cube")
                || id_low.contains("rock");

            if is_obstacle {
                let px = data[0];
                let pz = data[2];
                let sx = data[6];
                let sz = data[8];
                let half_sx = (sx * 0.5).max(0.2);
                let half_sz = (sz * 0.5).max(0.2);
                self.add_obstacle_box(
                    px - half_sx,
                    pz - half_sz,
                    px + half_sx,
                    pz + half_sz,
                );
            }
        }
    }

    pub fn find_path(&self, start_x: f32, start_z: f32, goal_x: f32, goal_z: f32) -> Option<Vec<[f32; 2]>> {
        self.find_path_budgeted(start_x, start_z, goal_x, goal_z, 2000)
    }

    pub fn find_path_budgeted(
        &self,
        start_x: f32,
        start_z: f32,
        goal_x: f32,
        goal_z: f32,
        max_iterations: usize,
    ) -> Option<Vec<[f32; 2]>> {
        let (start_gx, start_gz) = self.world_to_grid(start_x, start_z)?;
        let (goal_gx, goal_gz) = self.world_to_grid(goal_x, goal_z)?;

        if self.is_blocked(goal_gx, goal_gz) && (start_gx != goal_gx || start_gz != goal_gz) {
            // Goal is blocked
            return None;
        }

        if start_gx == goal_gx && start_gz == goal_gz {
            return Some(vec![[start_x, start_z], [goal_x, goal_z]]);
        }

        let total_cells = self.width * self.height;
        let mut dist = vec![f32::INFINITY; total_cells];
        let mut parent = vec![usize::MAX; total_cells];
        let mut heap = BinaryHeap::new();

        let start_idx = self.index(start_gx, start_gz);
        dist[start_idx] = 0.0;

        let octile_heuristic = |x1: usize, z1: usize, x2: usize, z2: usize| -> f32 {
            let dx = (x1 as isize - x2 as isize).abs() as f32;
            let dz = (z1 as isize - z2 as isize).abs() as f32;
            let min_d = dx.min(dz);
            let max_d = dx.max(dz);
            (max_d - min_d) + 1.41421356 * min_d
        };

        heap.push(NodeState {
            cost: 0.0,
            heuristic: octile_heuristic(start_gx, start_gz, goal_gx, goal_gz),
            gx: start_gx,
            gz: start_gz,
        });

        let mut reached = false;
        let mut iterations = 0usize;

        // 8 directions: dx, dz, cost
        let neighbors: [(isize, isize, f32); 8] = [
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (1, 1, 1.41421356),
            (1, -1, 1.41421356),
            (-1, 1, 1.41421356),
            (-1, -1, 1.41421356),
        ];

        while let Some(NodeState { cost, gx, gz, .. }) = heap.pop() {
            iterations += 1;
            if iterations > max_iterations {
                // Iteration budget exhausted to prevent frame drop
                return None;
            }

            let u_idx = self.index(gx, gz);
            if gx == goal_gx && gz == goal_gz {
                reached = true;
                break;
            }

            if cost > dist[u_idx] {
                continue;
            }

            for &(ndx, ndz, move_cost) in &neighbors {
                let ngx = gx as isize + ndx;
                let ngz = gz as isize + ndz;

                if ngx < 0 || ngx >= self.width as isize || ngz < 0 || ngz >= self.height as isize {
                    continue;
                }

                let ngx = ngx as usize;
                let ngz = ngz as usize;

                if self.is_blocked(ngx, ngz) {
                    continue;
                }

                // If moving diagonally, check that we don't cut through two blocked corners
                if ndx != 0 && ndz != 0 {
                    let side1_x = (gx as isize + ndx) as usize;
                    let side1_z = gz;
                    let side2_x = gx;
                    let side2_z = (gz as isize + ndz) as usize;
                    if self.is_blocked(side1_x, side1_z) && self.is_blocked(side2_x, side2_z) {
                        continue;
                    }
                }

                let v_idx = self.index(ngx, ngz);
                let next_cost = cost + move_cost;

                if next_cost < dist[v_idx] {
                    dist[v_idx] = next_cost;
                    parent[v_idx] = u_idx;
                    heap.push(NodeState {
                        cost: next_cost,
                        heuristic: octile_heuristic(ngx, ngz, goal_gx, goal_gz),
                        gx: ngx,
                        gz: ngz,
                    });
                }
            }
        }

        if !reached {
            return None;
        }

        // Reconstruct path
        let mut path_cells = Vec::new();
        let mut curr = self.index(goal_gx, goal_gz);
        while curr != usize::MAX {
            let cgx = curr % self.width;
            let cgz = curr / self.width;
            path_cells.push((cgx, cgz));
            if curr == start_idx {
                break;
            }
            curr = parent[curr];
        }
        path_cells.reverse();

        // Convert path to world points
        let mut waypoints = Vec::new();
        waypoints.push([start_x, start_z]);
        for (cgx, cgz) in path_cells.into_iter().skip(1) {
            if cgx == goal_gx && cgz == goal_gz {
                waypoints.push([goal_x, goal_z]);
            } else {
                let (wx, wz) = self.grid_to_world(cgx, cgz);
                waypoints.push([wx, wz]);
            }
        }

        Some(waypoints)
    }

    /// 3D / Voxel & Terrain aware A* pathfinding with step budget
    pub fn find_path_3d(
        &self,
        start: [f32; 3],
        goal: [f32; 3],
        max_iterations: usize,
        height_sampler: Option<&dyn Fn(f32, f32) -> f32>,
        obstacle_checker: Option<&dyn Fn(f32, f32, f32) -> bool>,
    ) -> Option<Vec<[f32; 3]>> {
        let (start_gx, start_gz) = self.world_to_grid(start[0], start[2])?;
        let (goal_gx, goal_gz) = self.world_to_grid(goal[0], goal[2])?;

        if self.is_blocked(goal_gx, goal_gz) && (start_gx != goal_gx || start_gz != goal_gz) {
            return None;
        }

        if start_gx == goal_gx && start_gz == goal_gz {
            return Some(vec![start, goal]);
        }

        let total_cells = self.width * self.height;
        let mut dist = vec![f32::INFINITY; total_cells];
        let mut parent = vec![usize::MAX; total_cells];
        let mut heap = BinaryHeap::new();

        let start_idx = self.index(start_gx, start_gz);
        dist[start_idx] = 0.0;

        let octile_heuristic = |x1: usize, z1: usize, x2: usize, z2: usize| -> f32 {
            let dx = (x1 as isize - x2 as isize).abs() as f32;
            let dz = (z1 as isize - z2 as isize).abs() as f32;
            let min_d = dx.min(dz);
            let max_d = dx.max(dz);
            (max_d - min_d) + 1.41421356 * min_d
        };

        heap.push(NodeState {
            cost: 0.0,
            heuristic: octile_heuristic(start_gx, start_gz, goal_gx, goal_gz),
            gx: start_gx,
            gz: start_gz,
        });

        let mut reached = false;
        let mut iterations = 0usize;

        let neighbors: [(isize, isize, f32); 8] = [
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (1, 1, 1.41421356),
            (1, -1, 1.41421356),
            (-1, 1, 1.41421356),
            (-1, -1, 1.41421356),
        ];

        while let Some(NodeState { cost, gx, gz, .. }) = heap.pop() {
            iterations += 1;
            if iterations > max_iterations {
                return None;
            }

            let u_idx = self.index(gx, gz);
            if gx == goal_gx && gz == goal_gz {
                reached = true;
                break;
            }

            if cost > dist[u_idx] {
                continue;
            }

            let (cur_wx, cur_wz) = self.grid_to_world(gx, gz);
            let cur_y = height_sampler.map(|f| f(cur_wx, cur_wz)).unwrap_or(start[1]);

            for &(ndx, ndz, move_cost) in &neighbors {
                let ngx = gx as isize + ndx;
                let ngz = gz as isize + ndz;

                if ngx < 0 || ngx >= self.width as isize || ngz < 0 || ngz >= self.height as isize {
                    continue;
                }

                let ngx = ngx as usize;
                let ngz = ngz as usize;

                if self.is_blocked(ngx, ngz) {
                    continue;
                }

                let (next_wx, next_wz) = self.grid_to_world(ngx, ngz);
                let next_y = height_sampler.map(|f| f(next_wx, next_wz)).unwrap_or(goal[1]);

                // Slope / step height check (max climb height 1.25m per grid step)
                if (next_y - cur_y).abs() > 1.25 {
                    continue;
                }

                // 3D obstacle / voxel collision check
                if let Some(checker) = obstacle_checker {
                    if checker(next_wx, next_y, next_wz) || checker(next_wx, next_y + 1.0, next_wz) {
                        continue;
                    }
                }

                // Diagonal corner cutting check
                if ndx != 0 && ndz != 0 {
                    let side1_x = (gx as isize + ndx) as usize;
                    let side1_z = gz;
                    let side2_x = gx;
                    let side2_z = (gz as isize + ndz) as usize;
                    if self.is_blocked(side1_x, side1_z) && self.is_blocked(side2_x, side2_z) {
                        continue;
                    }
                }

                let v_idx = self.index(ngx, ngz);
                let dy = next_y - cur_y;
                let step_dist = (move_cost * move_cost * self.cell_size * self.cell_size + dy * dy).sqrt();
                let next_cost = cost + step_dist;

                if next_cost < dist[v_idx] {
                    dist[v_idx] = next_cost;
                    parent[v_idx] = u_idx;
                    heap.push(NodeState {
                        cost: next_cost,
                        heuristic: octile_heuristic(ngx, ngz, goal_gx, goal_gz),
                        gx: ngx,
                        gz: ngz,
                    });
                }
            }
        }

        if !reached {
            return None;
        }

        let mut path_cells = Vec::new();
        let mut curr = self.index(goal_gx, goal_gz);
        while curr != usize::MAX {
            let cgx = curr % self.width;
            let cgz = curr / self.width;
            path_cells.push((cgx, cgz));
            if curr == start_idx {
                break;
            }
            curr = parent[curr];
        }
        path_cells.reverse();

        let mut waypoints = Vec::new();
        waypoints.push(start);
        for (cgx, cgz) in path_cells.into_iter().skip(1) {
            if cgx == goal_gx && cgz == goal_gz {
                waypoints.push(goal);
            } else {
                let (wx, wz) = self.grid_to_world(cgx, cgz);
                let y = height_sampler.map(|f| f(wx, wz)).unwrap_or(start[1]);
                waypoints.push([wx, y, wz]);
            }
        }

        Some(waypoints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nav_grid_coordinates() {
        let grid = NavGrid::new(1.0, 10.0);
        assert_eq!(grid.width, 20);
        assert_eq!(grid.height, 20);

        let (gx, gz) = grid.world_to_grid(0.0, 0.0).unwrap();
        assert_eq!(gx, 10);
        assert_eq!(gz, 10);

        let (wx, wz) = grid.grid_to_world(10, 10);
        assert!((wx - 0.5).abs() < 1e-4);
        assert!((wz - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_a_star_pathfinding_with_obstacle() {
        let mut grid = NavGrid::new(1.0, 10.0);

        // Place a wall at X = 0 from Z = -5 to Z = 5
        grid.add_obstacle_box(-0.4, -5.0, 0.4, 5.0);

        // Path from (-3, 0) to (3, 0)
        let path = grid.find_path(-3.0, 0.0, 3.0, 0.0);
        assert!(path.is_some(), "Expected path around wall");
        let waypoints = path.unwrap();
        assert!(waypoints.len() > 2);

        // Path should not pass through any blocked cell
        for pt in &waypoints {
            if let Some((gx, gz)) = grid.world_to_grid(pt[0], pt[1]) {
                assert!(!grid.is_blocked(gx, gz), "Waypoint {:?} landed on blocked cell", pt);
            }
        }
    }

    #[test]
    fn test_a_star_unreachable_goal() {
        let mut grid = NavGrid::new(1.0, 5.0);
        // Completely enclose (0, 0)
        grid.add_obstacle_box(-1.5, -1.5, 1.5, 1.5);

        let path = grid.find_path(-4.0, -4.0, 0.0, 0.0);
        assert!(path.is_none(), "Enclosed goal should be unreachable");
    }

    #[test]
    fn test_a_star_step_budget_exhausted() {
        let mut grid = NavGrid::new(1.0, 20.0);
        grid.add_obstacle_box(-0.4, -15.0, 0.4, 15.0);

        // A very tiny step budget of 2 iterations cannot solve a 40x40 maze
        let path = grid.find_path_budgeted(-15.0, 0.0, 15.0, 0.0, 2);
        assert!(path.is_none(), "Expected budget exhaustion to return None");
    }

    #[test]
    fn test_a_star_3d_terrain_and_voxels() {
        let mut grid = NavGrid::new(1.0, 10.0);
        // Add a wall obstacle
        grid.add_obstacle_box(-0.4, -4.0, 0.4, 4.0);

        // Height sampler: a gentle hill y = 0.5 * (x + 10)
        let height_fn = |x: f32, _z: f32| -> f32 { (x * 0.1).max(0.0) };

        let path = grid.find_path_3d(
            [-5.0, 0.0, 0.0],
            [5.0, 0.5, 0.0],
            1000,
            Some(&height_fn),
            None,
        );

        assert!(path.is_some(), "Expected valid 3D path");
        let waypoints = path.unwrap();
        assert!(waypoints.len() >= 2);
        assert_eq!(waypoints.first().unwrap()[0], -5.0);
        assert_eq!(waypoints.last().unwrap()[0], 5.0);

        // Verify obstacle checker blocks paths through voxels
        let blocked_by_voxel = grid.find_path_3d(
            [-5.0, 0.0, 0.0],
            [5.0, 0.5, 0.0],
            1000,
            Some(&height_fn),
            Some(&|_x, _y, _z| true), // Everything solid
        );
        assert!(blocked_by_voxel.is_none(), "Expected path to be blocked by voxels");
    }
}
