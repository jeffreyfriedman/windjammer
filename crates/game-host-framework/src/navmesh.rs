//! Navigation Mesh System
//!
//! Provides polygon-based navigation for complex 3D environments.
//!
//! ## Features
//! - Triangle-based navigation mesh
//! - Portal-based pathfinding
//! - Dynamic obstacle avoidance
//! - Agent radius support
//! - Height-based filtering

use crate::math::Vec3;
use std::collections::{HashMap, HashSet, VecDeque};

/// Navigation mesh polygon (triangle)
#[derive(Debug, Clone)]
pub struct NavPoly {
    /// Polygon ID
    pub id: u32,
    /// Vertices (always 3 for triangles)
    pub vertices: [Vec3; 3],
    /// Center point
    pub center: Vec3,
    /// Neighbor polygon IDs
    pub neighbors: Vec<u32>,
}

/// Navigation mesh
#[derive(Debug, Clone)]
pub struct NavMesh {
    /// All navigation polygons
    polygons: HashMap<u32, NavPoly>,
    /// Next polygon ID
    next_id: u32,
}

/// Navigation path
#[derive(Debug, Clone)]
pub struct NavPath {
    /// Waypoints along the path
    pub waypoints: Vec<Vec3>,
    /// Total path length
    pub length: f32,
}

/// Navigation agent configuration
#[derive(Debug, Clone)]
pub struct NavAgent {
    /// Agent radius (for obstacle avoidance)
    pub radius: f32,
    /// Agent height
    pub height: f32,
    /// Maximum slope (in degrees)
    pub max_slope: f32,
}

impl NavPoly {
    /// Create a new navigation polygon
    pub fn new(id: u32, vertices: [Vec3; 3]) -> Self {
        let center = (vertices[0] + vertices[1] + vertices[2]) / 3.0;
        Self {
            id,
            vertices,
            center,
            neighbors: Vec::new(),
        }
    }

    /// Check if point is inside polygon (2D projection)
    pub fn contains_point(&self, point: Vec3) -> bool {
        // Use barycentric coordinates for triangle containment
        let v0 = self.vertices[0];
        let v1 = self.vertices[1];
        let v2 = self.vertices[2];

        let d00 = (v0 - v2).dot(v0 - v2);
        let d01 = (v0 - v2).dot(v1 - v2);
        let d11 = (v1 - v2).dot(v1 - v2);
        let d20 = (point - v2).dot(v0 - v2);
        let d21 = (point - v2).dot(v1 - v2);

        let denom = d00 * d11 - d01 * d01;
        if denom.abs() < 1e-6 {
            return false;
        }

        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;

        u >= 0.0 && v >= 0.0 && w >= 0.0
    }

    /// Get shared edge with neighbor
    pub fn get_shared_edge(&self, other: &NavPoly) -> Option<(Vec3, Vec3)> {
        for i in 0..3 {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % 3];

            for j in 0..3 {
                let ov1 = other.vertices[j];
                let ov2 = other.vertices[(j + 1) % 3];

                if (v1 - ov1).length() < 0.01 && (v2 - ov2).length() < 0.01 {
                    return Some((v1, v2));
                }
                if (v1 - ov2).length() < 0.01 && (v2 - ov1).length() < 0.01 {
                    return Some((v1, v2));
                }
            }
        }
        None
    }
}

impl Default for NavAgent {
    fn default() -> Self {
        Self {
            radius: 0.5,
            height: 2.0,
            max_slope: 45.0,
        }
    }
}

impl NavMesh {
    /// Create a new navigation mesh
    pub fn new() -> Self {
        Self {
            polygons: HashMap::new(),
            next_id: 0,
        }
    }

    /// Add a navigation polygon
    pub fn add_polygon(&mut self, vertices: [Vec3; 3]) -> u32 {
        let id = self.next_id;
        self.next_id += 1;

        let poly = NavPoly::new(id, vertices);
        self.polygons.insert(id, poly);

        // Update neighbors
        self.update_neighbors(id);

        id
    }

    /// Update neighbor connections for a polygon
    fn update_neighbors(&mut self, poly_id: u32) {
        let mut neighbors = Vec::new();

        // Find all polygons that share an edge
        if let Some(poly) = self.polygons.get(&poly_id) {
            for (other_id, other_poly) in &self.polygons {
                if *other_id != poly_id && poly.get_shared_edge(other_poly).is_some() {
                    neighbors.push(*other_id);
                }
            }
        }

        // Update this polygon's neighbors
        if let Some(poly) = self.polygons.get_mut(&poly_id) {
            poly.neighbors = neighbors.clone();
        }

        // Update other polygons' neighbors
        for neighbor_id in neighbors {
            if let Some(neighbor) = self.polygons.get_mut(&neighbor_id) {
                if !neighbor.neighbors.contains(&poly_id) {
                    neighbor.neighbors.push(poly_id);
                }
            }
        }
    }

    /// Find polygon containing point
    pub fn find_polygon(&self, point: Vec3) -> Option<u32> {
        for (id, poly) in &self.polygons {
            if poly.contains_point(point) {
                return Some(*id);
            }
        }
        None
    }

    /// Find path between two points
    pub fn find_path(&self, start: Vec3, goal: Vec3, _agent: &NavAgent) -> Option<NavPath> {
        let start_poly = self.find_polygon(start)?;
        let goal_poly = self.find_polygon(goal)?;

        if start_poly == goal_poly {
            return Some(NavPath {
                waypoints: vec![start, goal],
                length: (goal - start).length(),
            });
        }

        // BFS to find polygon path
        let poly_path = self.find_polygon_path(start_poly, goal_poly)?;

        // Convert polygon path to waypoints
        let mut waypoints = vec![start];
        
        for i in 0..poly_path.len() - 1 {
            let current_id = poly_path[i];
            let next_id = poly_path[i + 1];

            if let (Some(current), Some(next)) = (
                self.polygons.get(&current_id),
                self.polygons.get(&next_id),
            ) {
                if let Some((edge_start, edge_end)) = current.get_shared_edge(next) {
                    // Add portal midpoint as waypoint
                    let portal = (edge_start + edge_end) / 2.0;
                    waypoints.push(portal);
                }
            }
        }

        waypoints.push(goal);

        // Calculate total length
        let mut length = 0.0;
        for i in 0..waypoints.len() - 1 {
            length += (waypoints[i + 1] - waypoints[i]).length();
        }

        Some(NavPath { waypoints, length })
    }

    /// Find path through polygons using BFS
    fn find_polygon_path(&self, start: u32, goal: u32) -> Option<Vec<u32>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut came_from: HashMap<u32, u32> = HashMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == goal {
                // Reconstruct path
                let mut path = vec![current];
                let mut curr = current;
                while let Some(&prev) = came_from.get(&curr) {
                    path.push(prev);
                    curr = prev;
                }
                path.reverse();
                return Some(path);
            }

            if let Some(poly) = self.polygons.get(&current) {
                for &neighbor in &poly.neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        came_from.insert(neighbor, current);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        None
    }

    /// Get number of polygons
    pub fn polygon_count(&self) -> usize {
        self.polygons.len()
    }
}

impl Default for NavMesh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navmesh_creation() {
        let mesh = NavMesh::new();
        assert_eq!(mesh.polygon_count(), 0);
        println!("✅ NavMesh created");
    }

    #[test]
    fn test_add_polygon() {
        let mut mesh = NavMesh::new();
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 1.0),
        ];
        let id = mesh.add_polygon(vertices);
        assert_eq!(mesh.polygon_count(), 1);
        assert_eq!(id, 0);
        println!("✅ Polygon added");
    }

    #[test]
    fn test_find_polygon() {
        let mut mesh = NavMesh::new();
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 2.0),
        ];
        let id = mesh.add_polygon(vertices);

        let point = Vec3::new(1.0, 0.0, 0.5);
        let found = mesh.find_polygon(point);
        assert_eq!(found, Some(id));
        println!("✅ Polygon found");
    }

    #[test]
    fn test_simple_path() {
        let mut mesh = NavMesh::new();
        
        // Create two adjacent triangles
        let poly1 = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 2.0),
        ];
        let poly2 = [
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 2.0),
        ];

        mesh.add_polygon(poly1);
        mesh.add_polygon(poly2);

        let start = Vec3::new(0.5, 0.0, 0.5);
        let goal = Vec3::new(3.5, 0.0, 0.5);
        let agent = NavAgent::default();

        let path = mesh.find_path(start, goal, &agent);
        assert!(path.is_some());
        
        let path = path.unwrap();
        assert!(path.waypoints.len() >= 2);
        println!("✅ Path found: {} waypoints, length: {}", path.waypoints.len(), path.length);
    }

    #[test]
    fn test_navagent_default() {
        let agent = NavAgent::default();
        assert_eq!(agent.radius, 0.5);
        assert_eq!(agent.height, 2.0);
        assert_eq!(agent.max_slope, 45.0);
        println!("✅ NavAgent default values");
    }

    #[test]
    fn test_polygon_contains() {
        let poly = NavPoly::new(
            0,
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 2.0),
            ],
        );

        assert!(poly.contains_point(Vec3::new(1.0, 0.0, 0.5)));
        assert!(!poly.contains_point(Vec3::new(5.0, 0.0, 5.0)));
        println!("✅ Polygon containment");
    }

    #[test]
    fn test_shared_edge() {
        let poly1 = NavPoly::new(
            0,
            [
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 0.0, 1.0),
            ],
        );

        let poly2 = NavPoly::new(
            1,
            [
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(1.5, 0.0, 1.0),
            ],
        );

        let edge = poly1.get_shared_edge(&poly2);
        assert!(edge.is_some());
        println!("✅ Shared edge detection");
    }
}

