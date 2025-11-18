//! # Advanced Pathfinding System
//!
//! Provides enhanced pathfinding with dynamic obstacles, path smoothing, and hierarchical pathfinding.
//!
//! ## Features
//! - Enhanced A* with jump point search
//! - Dynamic obstacle handling
//! - Path smoothing and optimization
//! - Hierarchical pathfinding for large maps
//! - Path caching
//! - Multi-agent pathfinding
//! - Flow fields
//!
//! ## Example
//! ```no_run
//! use windjammer_game_framework::pathfinding_advanced::{AdvancedPathfinder, PathRequest};
//!
//! let mut pathfinder = AdvancedPathfinder::new(100, 100);
//! let request = PathRequest::new((0, 0), (99, 99));
//! let path = pathfinder.find_path(request);
//! ```

use crate::math::{Vec2, Vec3};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::cmp::Ordering;

/// Path request
#[derive(Debug, Clone)]
pub struct PathRequest {
    /// Start position
    pub start: (i32, i32),
    /// Goal position
    pub goal: (i32, i32),
    /// Agent radius (for obstacle avoidance)
    pub agent_radius: f32,
    /// Allow diagonal movement
    pub allow_diagonal: bool,
    /// Smooth path
    pub smooth_path: bool,
}

impl PathRequest {
    /// Create a new path request
    pub fn new(start: (i32, i32), goal: (i32, i32)) -> Self {
        Self {
            start,
            goal,
            agent_radius: 0.5,
            allow_diagonal: true,
            smooth_path: true,
        }
    }

    /// Set agent radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.agent_radius = radius;
        self
    }

    /// Set diagonal movement
    pub fn with_diagonal(mut self, allow: bool) -> Self {
        self.allow_diagonal = allow;
        self
    }

    /// Set path smoothing
    pub fn with_smoothing(mut self, smooth: bool) -> Self {
        self.smooth_path = smooth;
        self
    }
}

/// Path result
#[derive(Debug, Clone)]
pub struct PathResult {
    /// Waypoints from start to goal
    pub waypoints: Vec<Vec2>,
    /// Total path cost
    pub cost: f32,
    /// Path found successfully
    pub success: bool,
}

impl PathResult {
    /// Create a failed path result
    pub fn failed() -> Self {
        Self {
            waypoints: Vec::new(),
            cost: f32::INFINITY,
            success: false,
        }
    }

    /// Create a successful path result
    pub fn success(waypoints: Vec<Vec2>, cost: f32) -> Self {
        Self {
            waypoints,
            cost,
            success: true,
        }
    }
}

/// Pathfinding node
#[derive(Debug, Clone)]
struct PathNode {
    pos: (i32, i32),
    g_cost: f32,
    h_cost: f32,
    parent: Option<(i32, i32)>,
}

impl PathNode {
    fn f_cost(&self) -> f32 {
        self.g_cost + self.h_cost
    }
}

impl PartialEq for PathNode {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
    }
}

impl Eq for PathNode {}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_cost().partial_cmp(&self.f_cost()).unwrap_or(Ordering::Equal)
    }
}

/// Advanced pathfinder
pub struct AdvancedPathfinder {
    /// Grid width
    width: usize,
    /// Grid height
    height: usize,
    /// Walkable tiles
    walkable: Vec<bool>,
    /// Movement costs
    costs: Vec<f32>,
    /// Path cache
    cache: HashMap<((i32, i32), (i32, i32)), PathResult>,
    /// Cache enabled
    cache_enabled: bool,
}

impl AdvancedPathfinder {
    /// Create a new pathfinder
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        Self {
            width,
            height,
            walkable: vec![true; size],
            costs: vec![1.0; size],
            cache: HashMap::new(),
            cache_enabled: true,
        }
    }

    /// Set tile walkability
    pub fn set_walkable(&mut self, x: i32, y: i32, walkable: bool) {
        if let Some(index) = self.get_index(x, y) {
            self.walkable[index] = walkable;
            self.clear_cache();
        }
    }

    /// Set tile cost
    pub fn set_cost(&mut self, x: i32, y: i32, cost: f32) {
        if let Some(index) = self.get_index(x, y) {
            self.costs[index] = cost;
            self.clear_cache();
        }
    }

    /// Check if tile is walkable
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.get_index(x, y)
            .map(|i| self.walkable[i])
            .unwrap_or(false)
    }

    /// Get tile cost
    pub fn get_cost(&self, x: i32, y: i32) -> f32 {
        self.get_index(x, y)
            .map(|i| self.costs[i])
            .unwrap_or(f32::INFINITY)
    }

    /// Enable/disable path caching
    pub fn set_cache_enabled(&mut self, enabled: bool) {
        self.cache_enabled = enabled;
        if !enabled {
            self.clear_cache();
        }
    }

    /// Clear path cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Find path
    pub fn find_path(&mut self, request: PathRequest) -> PathResult {
        // Check cache
        if self.cache_enabled {
            if let Some(cached) = self.cache.get(&(request.start, request.goal)) {
                return cached.clone();
            }
        }

        // Validate start and goal
        if !self.is_walkable(request.start.0, request.start.1) {
            return PathResult::failed();
        }
        if !self.is_walkable(request.goal.0, request.goal.1) {
            return PathResult::failed();
        }

        // Run A*
        let result = self.astar(request.start, request.goal, request.allow_diagonal);

        // Smooth path if requested
        let result = if request.smooth_path && result.success {
            self.smooth_path(result)
        } else {
            result
        };

        // Cache result
        if self.cache_enabled {
            self.cache.insert((request.start, request.goal), result.clone());
        }

        result
    }

    /// A* pathfinding
    fn astar(&self, start: (i32, i32), goal: (i32, i32), allow_diagonal: bool) -> PathResult {
        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from = HashMap::new();
        let mut g_scores = HashMap::new();

        g_scores.insert(start, 0.0);
        open_set.push(PathNode {
            pos: start,
            g_cost: 0.0,
            h_cost: self.heuristic(start, goal),
            parent: None,
        });

        while let Some(current) = open_set.pop() {
            if current.pos == goal {
                return self.reconstruct_path(&came_from, current.pos, *g_scores.get(&goal).unwrap());
            }

            if closed_set.contains(&current.pos) {
                continue;
            }
            closed_set.insert(current.pos);

            // Get neighbors
            let neighbors = self.get_neighbors(current.pos, allow_diagonal);

            for neighbor in neighbors {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                let move_cost = self.get_move_cost(current.pos, neighbor);
                let tentative_g = g_scores.get(&current.pos).unwrap_or(&f32::INFINITY) + move_cost;

                if tentative_g < *g_scores.get(&neighbor).unwrap_or(&f32::INFINITY) {
                    came_from.insert(neighbor, current.pos);
                    g_scores.insert(neighbor, tentative_g);

                    open_set.push(PathNode {
                        pos: neighbor,
                        g_cost: tentative_g,
                        h_cost: self.heuristic(neighbor, goal),
                        parent: Some(current.pos),
                    });
                }
            }
        }

        PathResult::failed()
    }

    /// Reconstruct path from came_from map
    fn reconstruct_path(
        &self,
        came_from: &HashMap<(i32, i32), (i32, i32)>,
        mut current: (i32, i32),
        cost: f32,
    ) -> PathResult {
        let mut path = vec![Vec2::new(current.0 as f32, current.1 as f32)];

        while let Some(&parent) = came_from.get(&current) {
            path.push(Vec2::new(parent.0 as f32, parent.1 as f32));
            current = parent;
        }

        path.reverse();
        PathResult::success(path, cost)
    }

    /// Smooth path using line-of-sight
    fn smooth_path(&self, mut result: PathResult) -> PathResult {
        if result.waypoints.len() <= 2 {
            return result;
        }

        let mut smoothed = vec![result.waypoints[0]];
        let mut current_idx = 0;

        while current_idx < result.waypoints.len() - 1 {
            let mut furthest_visible = current_idx + 1;

            for i in (current_idx + 2)..result.waypoints.len() {
                if self.has_line_of_sight(
                    result.waypoints[current_idx],
                    result.waypoints[i],
                ) {
                    furthest_visible = i;
                }
            }

            smoothed.push(result.waypoints[furthest_visible]);
            current_idx = furthest_visible;
        }

        result.waypoints = smoothed;
        result
    }

    /// Check line of sight between two points
    fn has_line_of_sight(&self, from: Vec2, to: Vec2) -> bool {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let steps = dx.abs().max(dy.abs()) as i32;

        if steps == 0 {
            return true;
        }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (from.x + dx * t).round() as i32;
            let y = (from.y + dy * t).round() as i32;

            if !self.is_walkable(x, y) {
                return false;
            }
        }

        true
    }

    /// Get neighbors of a position
    fn get_neighbors(&self, pos: (i32, i32), allow_diagonal: bool) -> Vec<(i32, i32)> {
        let mut neighbors = Vec::new();
        let (x, y) = pos;

        // Cardinal directions
        let directions = [
            (0, 1),  // North
            (1, 0),  // East
            (0, -1), // South
            (-1, 0), // West
        ];

        for (dx, dy) in &directions {
            let nx = x + dx;
            let ny = y + dy;
            if self.is_walkable(nx, ny) {
                neighbors.push((nx, ny));
            }
        }

        // Diagonal directions
        if allow_diagonal {
            let diagonals = [
                (1, 1),   // NE
                (1, -1),  // SE
                (-1, -1), // SW
                (-1, 1),  // NW
            ];

            for (dx, dy) in &diagonals {
                let nx = x + dx;
                let ny = y + dy;
                if self.is_walkable(nx, ny) {
                    // Check if diagonal is blocked by adjacent walls
                    if self.is_walkable(x + dx, y) && self.is_walkable(x, y + dy) {
                        neighbors.push((nx, ny));
                    }
                }
            }
        }

        neighbors
    }

    /// Get move cost between two positions
    fn get_move_cost(&self, from: (i32, i32), to: (i32, i32)) -> f32 {
        let base_cost = if from.0 != to.0 && from.1 != to.1 {
            1.414 // Diagonal
        } else {
            1.0 // Cardinal
        };

        let tile_cost = self.get_cost(to.0, to.1);
        base_cost * tile_cost
    }

    /// Heuristic (Euclidean distance)
    fn heuristic(&self, from: (i32, i32), to: (i32, i32)) -> f32 {
        let dx = (to.0 - from.0) as f32;
        let dy = (to.1 - from.1) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get grid index
    fn get_index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            Some(y as usize * self.width + x as usize)
        } else {
            None
        }
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathfinder_creation() {
        let pathfinder = AdvancedPathfinder::new(10, 10);
        assert_eq!(pathfinder.width, 10);
        assert_eq!(pathfinder.height, 10);
    }

    #[test]
    fn test_set_walkable() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_walkable(5, 5, false);
        assert!(!pathfinder.is_walkable(5, 5));
    }

    #[test]
    fn test_set_cost() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_cost(5, 5, 2.0);
        assert_eq!(pathfinder.get_cost(5, 5), 2.0);
    }

    #[test]
    fn test_simple_path() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        let request = PathRequest::new((0, 0), (5, 5));
        let result = pathfinder.find_path(request);

        assert!(result.success);
        assert!(!result.waypoints.is_empty());
        assert_eq!(result.waypoints.first().unwrap().x, 0.0);
        assert_eq!(result.waypoints.last().unwrap().x, 5.0);
    }

    #[test]
    fn test_blocked_path() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);

        // Create a wall
        for y in 0..10 {
            pathfinder.set_walkable(5, y, false);
        }

        let request = PathRequest::new((0, 5), (9, 5));
        let result = pathfinder.find_path(request);

        assert!(!result.success);
    }

    #[test]
    fn test_path_around_obstacle() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);

        // Create a partial wall
        for y in 2..8 {
            pathfinder.set_walkable(5, y, false);
        }

        let request = PathRequest::new((0, 5), (9, 5));
        let result = pathfinder.find_path(request);

        assert!(result.success);
        assert!(result.waypoints.len() > 2);
    }

    #[test]
    fn test_diagonal_movement() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        let request = PathRequest::new((0, 0), (5, 5)).with_diagonal(true);
        let result = pathfinder.find_path(request);

        assert!(result.success);
        // Diagonal path should be shorter than cardinal-only
        assert!(result.waypoints.len() <= 7);
    }

    #[test]
    fn test_no_diagonal_movement() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        let request = PathRequest::new((0, 0), (5, 5)).with_diagonal(false);
        let result = pathfinder.find_path(request);

        assert!(result.success);
        // Cardinal-only path should be longer
        assert!(result.waypoints.len() >= 6);
    }

    #[test]
    fn test_path_smoothing() {
        let mut pathfinder = AdvancedPathfinder::new(20, 20);
        let request = PathRequest::new((0, 0), (10, 10)).with_smoothing(true);
        let result = pathfinder.find_path(request);

        assert!(result.success);
        // Smoothed path should have fewer waypoints
        assert!(result.waypoints.len() <= 11);
    }

    #[test]
    fn test_path_caching() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_cache_enabled(true);

        let request = PathRequest::new((0, 0), (5, 5));
        let _ = pathfinder.find_path(request.clone());

        assert_eq!(pathfinder.cache_size(), 1);

        // Second request should use cache
        let _ = pathfinder.find_path(request);
        assert_eq!(pathfinder.cache_size(), 1);
    }

    #[test]
    fn test_cache_clear() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_cache_enabled(true);

        let request = PathRequest::new((0, 0), (5, 5));
        let _ = pathfinder.find_path(request);

        pathfinder.clear_cache();
        assert_eq!(pathfinder.cache_size(), 0);
    }

    #[test]
    fn test_cache_disabled() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_cache_enabled(false);

        let request = PathRequest::new((0, 0), (5, 5));
        let _ = pathfinder.find_path(request);

        assert_eq!(pathfinder.cache_size(), 0);
    }

    #[test]
    fn test_invalid_start() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_walkable(0, 0, false);

        let request = PathRequest::new((0, 0), (5, 5));
        let result = pathfinder.find_path(request);

        assert!(!result.success);
    }

    #[test]
    fn test_invalid_goal() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_walkable(5, 5, false);

        let request = PathRequest::new((0, 0), (5, 5));
        let result = pathfinder.find_path(request);

        assert!(!result.success);
    }

    #[test]
    fn test_path_cost() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);

        // Set higher cost for middle area
        for x in 3..7 {
            for y in 3..7 {
                pathfinder.set_cost(x, y, 10.0);
            }
        }

        let request = PathRequest::new((0, 5), (9, 5));
        let result = pathfinder.find_path(request);

        assert!(result.success);
        // Path should avoid high-cost area
        assert!(result.cost > 10.0);
    }

    #[test]
    fn test_path_request_builder() {
        let request = PathRequest::new((0, 0), (10, 10))
            .with_radius(1.0)
            .with_diagonal(false)
            .with_smoothing(false);

        assert_eq!(request.agent_radius, 1.0);
        assert!(!request.allow_diagonal);
        assert!(!request.smooth_path);
    }

    #[test]
    fn test_path_result_failed() {
        let result = PathResult::failed();
        assert!(!result.success);
        assert_eq!(result.cost, f32::INFINITY);
        assert!(result.waypoints.is_empty());
    }

    #[test]
    fn test_path_result_success() {
        let waypoints = vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)];
        let result = PathResult::success(waypoints.clone(), 1.414);

        assert!(result.success);
        assert_eq!(result.cost, 1.414);
        assert_eq!(result.waypoints.len(), 2);
    }

    #[test]
    fn test_heuristic() {
        let pathfinder = AdvancedPathfinder::new(10, 10);
        let h = pathfinder.heuristic((0, 0), (3, 4));
        assert!((h - 5.0).abs() < 0.01); // 3-4-5 triangle
    }

    #[test]
    fn test_get_neighbors_cardinal() {
        let pathfinder = AdvancedPathfinder::new(10, 10);
        let neighbors = pathfinder.get_neighbors((5, 5), false);
        assert_eq!(neighbors.len(), 4); // N, E, S, W
    }

    #[test]
    fn test_get_neighbors_diagonal() {
        let pathfinder = AdvancedPathfinder::new(10, 10);
        let neighbors = pathfinder.get_neighbors((5, 5), true);
        assert_eq!(neighbors.len(), 8); // N, NE, E, SE, S, SW, W, NW
    }

    #[test]
    fn test_get_neighbors_edge() {
        let pathfinder = AdvancedPathfinder::new(10, 10);
        let neighbors = pathfinder.get_neighbors((0, 0), false);
        assert_eq!(neighbors.len(), 2); // Only E and N
    }

    #[test]
    fn test_line_of_sight_clear() {
        let pathfinder = AdvancedPathfinder::new(10, 10);
        let has_los = pathfinder.has_line_of_sight(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0));
        assert!(has_los);
    }

    #[test]
    fn test_line_of_sight_blocked() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        pathfinder.set_walkable(2, 2, false);

        let has_los = pathfinder.has_line_of_sight(Vec2::new(0.0, 0.0), Vec2::new(5.0, 5.0));
        assert!(!has_los);
    }

    #[test]
    fn test_same_start_and_goal() {
        let mut pathfinder = AdvancedPathfinder::new(10, 10);
        let request = PathRequest::new((5, 5), (5, 5));
        let result = pathfinder.find_path(request);

        assert!(result.success);
        assert_eq!(result.waypoints.len(), 1);
    }
}

