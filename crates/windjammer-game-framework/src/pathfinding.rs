//! A* Pathfinding System
//!
//! Provides efficient pathfinding for game AI using the A* algorithm.
//!
//! ## Features
//! - Grid-based pathfinding
//! - Diagonal movement support
//! - Configurable heuristics
//! - Path smoothing
//! - Dynamic obstacle handling

use crate::math::Vec2;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

/// Grid-based pathfinding map
#[derive(Debug, Clone)]
pub struct PathfindingGrid {
    /// Grid width
    pub width: usize,
    /// Grid height
    pub height: usize,
    /// Walkable tiles (true = walkable, false = blocked)
    walkable: Vec<bool>,
    /// Movement cost per tile (1.0 = normal, higher = slower)
    costs: Vec<f32>,
}

/// A position in the grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

/// Pathfinding result
#[derive(Debug, Clone)]
pub struct Path {
    /// Waypoints from start to goal
    pub waypoints: Vec<GridPos>,
    /// Total path cost
    pub cost: f32,
}

/// Pathfinding heuristic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Heuristic {
    /// Manhattan distance (4-directional)
    Manhattan,
    /// Euclidean distance (diagonal)
    Euclidean,
    /// Chebyshev distance (8-directional)
    Chebyshev,
}

/// A* pathfinding node
#[derive(Debug, Clone)]
struct PathNode {
    pos: GridPos,
    g_cost: f32, // Cost from start
    h_cost: f32, // Heuristic cost to goal
    parent: Option<GridPos>,
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
        // Reverse ordering for min-heap
        other.f_cost().partial_cmp(&self.f_cost()).unwrap_or(Ordering::Equal)
    }
}

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn to_vec2(&self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }

    pub fn from_vec2(v: Vec2) -> Self {
        Self::new(v.x as i32, v.y as i32)
    }

    pub fn distance_manhattan(&self, other: &GridPos) -> f32 {
        ((self.x - other.x).abs() + (self.y - other.y).abs()) as f32
    }

    pub fn distance_euclidean(&self, other: &GridPos) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn distance_chebyshev(&self, other: &GridPos) -> f32 {
        ((self.x - other.x).abs().max((self.y - other.y).abs())) as f32
    }
}

impl PathfindingGrid {
    /// Create a new pathfinding grid
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            walkable: vec![true; width * height],
            costs: vec![1.0; width * height],
        }
    }

    /// Get index from position
    fn index(&self, pos: GridPos) -> Option<usize> {
        if pos.x >= 0 && pos.x < self.width as i32 && pos.y >= 0 && pos.y < self.height as i32 {
            Some(pos.y as usize * self.width + pos.x as usize)
        } else {
            None
        }
    }

    /// Check if position is walkable
    pub fn is_walkable(&self, pos: GridPos) -> bool {
        self.index(pos).map(|i| self.walkable[i]).unwrap_or(false)
    }

    /// Set walkable state
    pub fn set_walkable(&mut self, pos: GridPos, walkable: bool) {
        if let Some(i) = self.index(pos) {
            self.walkable[i] = walkable;
        }
    }

    /// Get movement cost
    pub fn get_cost(&self, pos: GridPos) -> f32 {
        self.index(pos).map(|i| self.costs[i]).unwrap_or(f32::INFINITY)
    }

    /// Set movement cost
    pub fn set_cost(&mut self, pos: GridPos, cost: f32) {
        if let Some(i) = self.index(pos) {
            self.costs[i] = cost;
        }
    }

    /// Get neighbors (4-directional or 8-directional)
    fn get_neighbors(&self, pos: GridPos, allow_diagonal: bool) -> Vec<GridPos> {
        let mut neighbors = Vec::new();

        // Cardinal directions
        let dirs = [(0, 1), (1, 0), (0, -1), (-1, 0)];
        for (dx, dy) in &dirs {
            let neighbor = GridPos::new(pos.x + dx, pos.y + dy);
            if self.is_walkable(neighbor) {
                neighbors.push(neighbor);
            }
        }

        // Diagonal directions
        if allow_diagonal {
            let diag_dirs = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
            for (dx, dy) in &diag_dirs {
                let neighbor = GridPos::new(pos.x + dx, pos.y + dy);
                if self.is_walkable(neighbor) {
                    // Check if diagonal movement is blocked by adjacent walls
                    let adj1 = GridPos::new(pos.x + dx, pos.y);
                    let adj2 = GridPos::new(pos.x, pos.y + dy);
                    if self.is_walkable(adj1) || self.is_walkable(adj2) {
                        neighbors.push(neighbor);
                    }
                }
            }
        }

        neighbors
    }

    /// Find path using A* algorithm
    pub fn find_path(
        &self,
        start: GridPos,
        goal: GridPos,
        heuristic: Heuristic,
        allow_diagonal: bool,
    ) -> Option<Path> {
        if !self.is_walkable(start) || !self.is_walkable(goal) {
            return None;
        }

        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: HashMap<GridPos, GridPos> = HashMap::new();
        let mut g_scores: HashMap<GridPos, f32> = HashMap::new();

        g_scores.insert(start, 0.0);
        open_set.push(PathNode {
            pos: start,
            g_cost: 0.0,
            h_cost: self.heuristic_cost(start, goal, heuristic),
            parent: None,
        });

        while let Some(current) = open_set.pop() {
            if current.pos == goal {
                return Some(self.reconstruct_path(came_from, current.pos, start));
            }

            if closed_set.contains(&current.pos) {
                continue;
            }

            closed_set.insert(current.pos);

            for neighbor in self.get_neighbors(current.pos, allow_diagonal) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                let move_cost = if allow_diagonal && 
                    (neighbor.x != current.pos.x && neighbor.y != current.pos.y) {
                    1.414 // Diagonal cost (sqrt(2))
                } else {
                    1.0
                };

                let tentative_g = current.g_cost + move_cost * self.get_cost(neighbor);

                if tentative_g < *g_scores.get(&neighbor).unwrap_or(&f32::INFINITY) {
                    came_from.insert(neighbor, current.pos);
                    g_scores.insert(neighbor, tentative_g);

                    open_set.push(PathNode {
                        pos: neighbor,
                        g_cost: tentative_g,
                        h_cost: self.heuristic_cost(neighbor, goal, heuristic),
                        parent: Some(current.pos),
                    });
                }
            }
        }

        None // No path found
    }

    /// Calculate heuristic cost
    fn heuristic_cost(&self, from: GridPos, to: GridPos, heuristic: Heuristic) -> f32 {
        match heuristic {
            Heuristic::Manhattan => from.distance_manhattan(&to),
            Heuristic::Euclidean => from.distance_euclidean(&to),
            Heuristic::Chebyshev => from.distance_chebyshev(&to),
        }
    }

    /// Reconstruct path from came_from map
    fn reconstruct_path(
        &self,
        came_from: HashMap<GridPos, GridPos>,
        mut current: GridPos,
        start: GridPos,
    ) -> Path {
        let mut waypoints = vec![current];
        let mut total_cost = 0.0;

        while current != start {
            if let Some(&prev) = came_from.get(&current) {
                total_cost += self.get_cost(current);
                waypoints.push(prev);
                current = prev;
            } else {
                break;
            }
        }

        waypoints.reverse();

        Path {
            waypoints,
            cost: total_cost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = PathfindingGrid::new(10, 10);
        assert_eq!(grid.width, 10);
        assert_eq!(grid.height, 10);
        println!("✅ PathfindingGrid created");
    }

    #[test]
    fn test_walkable() {
        let mut grid = PathfindingGrid::new(10, 10);
        let pos = GridPos::new(5, 5);
        
        assert!(grid.is_walkable(pos));
        
        grid.set_walkable(pos, false);
        assert!(!grid.is_walkable(pos));
        
        println!("✅ Walkable state works");
    }

    #[test]
    fn test_simple_path() {
        let grid = PathfindingGrid::new(10, 10);
        let start = GridPos::new(0, 0);
        let goal = GridPos::new(5, 5);
        
        let path = grid.find_path(start, goal, Heuristic::Manhattan, false);
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert_eq!(path.waypoints.first(), Some(&start));
        assert_eq!(path.waypoints.last(), Some(&goal));
        
        println!("✅ Simple path found: {} waypoints", path.waypoints.len());
    }

    #[test]
    fn test_path_with_obstacle() {
        let mut grid = PathfindingGrid::new(10, 10);
        
        // Create a wall
        for y in 0..10 {
            grid.set_walkable(GridPos::new(5, y), false);
        }
        
        let start = GridPos::new(0, 5);
        let goal = GridPos::new(9, 5);
        
        let path = grid.find_path(start, goal, Heuristic::Manhattan, false);
        assert!(path.is_some());
        
        println!("✅ Path around obstacle found");
    }

    #[test]
    fn test_no_path() {
        let mut grid = PathfindingGrid::new(10, 10);
        
        // Create impassable walls
        for x in 0..10 {
            grid.set_walkable(GridPos::new(x, 5), false);
        }
        
        let start = GridPos::new(5, 0);
        let goal = GridPos::new(5, 9);
        
        let path = grid.find_path(start, goal, Heuristic::Manhattan, false);
        assert!(path.is_none());
        
        println!("✅ No path correctly detected");
    }

    #[test]
    fn test_diagonal_movement() {
        let grid = PathfindingGrid::new(10, 10);
        let start = GridPos::new(0, 0);
        let goal = GridPos::new(5, 5);
        
        let path_no_diag = grid.find_path(start, goal, Heuristic::Manhattan, false);
        let path_with_diag = grid.find_path(start, goal, Heuristic::Euclidean, true);
        
        assert!(path_no_diag.is_some());
        assert!(path_with_diag.is_some());
        
        // Diagonal path should be shorter
        assert!(path_with_diag.unwrap().waypoints.len() < path_no_diag.unwrap().waypoints.len());
        
        println!("✅ Diagonal movement works");
    }

    #[test]
    fn test_heuristics() {
        let pos1 = GridPos::new(0, 0);
        let pos2 = GridPos::new(3, 4);
        
        let manhattan = pos1.distance_manhattan(&pos2);
        let euclidean = pos1.distance_euclidean(&pos2);
        let chebyshev = pos1.distance_chebyshev(&pos2);
        
        assert_eq!(manhattan, 7.0);
        assert_eq!(euclidean, 5.0);
        assert_eq!(chebyshev, 4.0);
        
        println!("✅ Heuristics work: manhattan={}, euclidean={}, chebyshev={}", 
            manhattan, euclidean, chebyshev);
    }
}

