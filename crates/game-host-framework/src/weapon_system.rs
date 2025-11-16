//! Weapon System
//!
//! Comprehensive weapon management for FPS, TPS, and action games.
//!
//! ## Features
//! - Multiple weapon types (hitscan, projectile, melee)
//! - Weapon switching & inventory
//! - Ammo management
//! - Recoil & spread
//! - Reload mechanics
//! - Weapon attachments & modifications
//! - Damage falloff over distance

use crate::math::Vec3;
use std::collections::HashMap;

/// Weapon inventory manager
#[derive(Debug, Clone)]
pub struct WeaponInventory {
    /// All weapons in inventory
    pub weapons: Vec<Weapon>,
    /// Currently equipped weapon index
    pub current_weapon: Option<usize>,
    /// Maximum number of weapons
    pub max_weapons: usize,
}

/// A weapon
#[derive(Debug, Clone)]
pub struct Weapon {
    /// Weapon ID
    pub id: String,
    /// Weapon name
    pub name: String,
    /// Weapon type
    pub weapon_type: WeaponType,
    /// Damage per shot
    pub damage: f32,
    /// Fire rate (shots per second)
    pub fire_rate: f32,
    /// Range (max effective distance)
    pub range: f32,
    /// Accuracy (0.0 = perfect, 1.0 = very inaccurate)
    pub accuracy: f32,
    /// Recoil amount
    pub recoil: f32,
    /// Magazine size
    pub magazine_size: u32,
    /// Current ammo in magazine
    pub current_ammo: u32,
    /// Reserve ammo
    pub reserve_ammo: u32,
    /// Max reserve ammo
    pub max_reserve_ammo: u32,
    /// Reload time (seconds)
    pub reload_time: f32,
    /// Currently reloading
    pub is_reloading: bool,
    /// Reload progress (0.0 to 1.0)
    pub reload_progress: f32,
    /// Time since last shot
    pub time_since_last_shot: f32,
    /// Attachments
    pub attachments: Vec<WeaponAttachment>,
    /// Damage falloff curve
    pub damage_falloff: DamageFalloff,
}

/// Weapon type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponType {
    /// Instant hit (bullets, lasers)
    Hitscan,
    /// Physical projectile (rockets, arrows)
    Projectile,
    /// Close range (sword, knife)
    Melee,
    /// Area effect (grenade, explosive)
    Explosive,
}

/// Weapon attachment/modification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeaponAttachment {
    /// Optical sight
    Scope { magnification: u32 },
    /// Suppressor (reduces noise)
    Suppressor,
    /// Extended magazine
    ExtendedMag { bonus_ammo: u32 },
    /// Laser sight (improves accuracy)
    LaserSight,
    /// Foregrip (reduces recoil)
    Foregrip,
    /// Stock (improves stability)
    Stock,
}

/// Damage falloff over distance
#[derive(Debug, Clone, Copy)]
pub struct DamageFalloff {
    /// Minimum distance (full damage)
    pub min_distance: f32,
    /// Maximum distance (minimum damage)
    pub max_distance: f32,
    /// Minimum damage multiplier (at max distance)
    pub min_damage_multiplier: f32,
}

impl WeaponInventory {
    /// Create a new weapon inventory
    pub fn new(max_weapons: usize) -> Self {
        Self {
            weapons: Vec::new(),
            current_weapon: None,
            max_weapons,
        }
    }

    /// Add a weapon to inventory
    pub fn add_weapon(&mut self, weapon: Weapon) -> Result<usize, String> {
        if self.weapons.len() >= self.max_weapons {
            return Err("Inventory full".to_string());
        }

        let index = self.weapons.len();
        self.weapons.push(weapon);

        // Auto-equip if no weapon equipped
        if self.current_weapon.is_none() {
            self.current_weapon = Some(index);
        }

        Ok(index)
    }

    /// Remove a weapon from inventory
    pub fn remove_weapon(&mut self, index: usize) -> Option<Weapon> {
        if index >= self.weapons.len() {
            return None;
        }

        let weapon = self.weapons.remove(index);

        // Update current weapon index
        if let Some(current) = self.current_weapon {
            if current == index {
                self.current_weapon = if self.weapons.is_empty() {
                    None
                } else {
                    Some(0)
                };
            } else if current > index {
                self.current_weapon = Some(current - 1);
            }
        }

        Some(weapon)
    }

    /// Switch to a weapon by index
    pub fn switch_weapon(&mut self, index: usize) -> bool {
        if index < self.weapons.len() {
            self.current_weapon = Some(index);
            true
        } else {
            false
        }
    }

    /// Switch to next weapon
    pub fn next_weapon(&mut self) {
        if self.weapons.is_empty() {
            return;
        }

        self.current_weapon = Some(match self.current_weapon {
            Some(current) => (current + 1) % self.weapons.len(),
            None => 0,
        });
    }

    /// Switch to previous weapon
    pub fn previous_weapon(&mut self) {
        if self.weapons.is_empty() {
            return;
        }

        self.current_weapon = Some(match self.current_weapon {
            Some(current) => {
                if current == 0 {
                    self.weapons.len() - 1
                } else {
                    current - 1
                }
            }
            None => 0,
        });
    }

    /// Get current weapon
    pub fn get_current_weapon(&self) -> Option<&Weapon> {
        self.current_weapon.and_then(|i| self.weapons.get(i))
    }

    /// Get current weapon mutably
    pub fn get_current_weapon_mut(&mut self) -> Option<&mut Weapon> {
        self.current_weapon.and_then(|i| self.weapons.get_mut(i))
    }

    /// Get weapon by index
    pub fn get_weapon(&self, index: usize) -> Option<&Weapon> {
        self.weapons.get(index)
    }

    /// Get weapon count
    pub fn weapon_count(&self) -> usize {
        self.weapons.len()
    }

    /// Check if inventory is full
    pub fn is_full(&self) -> bool {
        self.weapons.len() >= self.max_weapons
    }
}

impl Weapon {
    /// Create a new weapon
    pub fn new(id: String, name: String, weapon_type: WeaponType) -> Self {
        Self {
            id,
            name,
            weapon_type,
            damage: 10.0,
            fire_rate: 10.0,
            range: 100.0,
            accuracy: 0.1,
            recoil: 0.1,
            magazine_size: 30,
            current_ammo: 30,
            reserve_ammo: 90,
            max_reserve_ammo: 300,
            reload_time: 2.0,
            is_reloading: false,
            reload_progress: 0.0,
            time_since_last_shot: 1.0, // Start ready to fire
            attachments: Vec::new(),
            damage_falloff: DamageFalloff::default(),
        }
    }

    /// Check if weapon can fire
    pub fn can_fire(&self) -> bool {
        !self.is_reloading
            && self.current_ammo > 0
            && self.time_since_last_shot >= 1.0 / self.fire_rate
    }

    /// Fire the weapon
    pub fn fire(&mut self) -> bool {
        if !self.can_fire() {
            return false;
        }

        self.current_ammo -= 1;
        self.time_since_last_shot = 0.0;
        true
    }

    /// Start reloading
    pub fn start_reload(&mut self) -> bool {
        if self.is_reloading || self.reserve_ammo == 0 || self.current_ammo == self.magazine_size {
            return false;
        }

        self.is_reloading = true;
        self.reload_progress = 0.0;
        true
    }

    /// Update weapon state
    pub fn update(&mut self, delta: f32) {
        // Update fire rate cooldown
        self.time_since_last_shot += delta;

        // Update reload
        if self.is_reloading {
            self.reload_progress += delta / self.reload_time;

            if self.reload_progress >= 1.0 {
                self.complete_reload();
            }
        }
    }

    /// Complete reload
    fn complete_reload(&mut self) {
        let ammo_needed = self.magazine_size - self.current_ammo;
        let ammo_to_add = ammo_needed.min(self.reserve_ammo);

        self.current_ammo += ammo_to_add;
        self.reserve_ammo -= ammo_to_add;

        self.is_reloading = false;
        self.reload_progress = 0.0;
    }

    /// Add ammo to reserves
    pub fn add_ammo(&mut self, amount: u32) {
        self.reserve_ammo = (self.reserve_ammo + amount).min(self.max_reserve_ammo);
    }

    /// Calculate damage at distance
    pub fn calculate_damage(&self, distance: f32) -> f32 {
        self.damage_falloff.calculate_damage(self.damage, distance)
    }

    /// Add attachment
    pub fn add_attachment(&mut self, attachment: WeaponAttachment) {
        // Apply attachment effects
        match &attachment {
            WeaponAttachment::ExtendedMag { bonus_ammo } => {
                self.magazine_size += bonus_ammo;
            }
            WeaponAttachment::LaserSight => {
                self.accuracy *= 0.8; // 20% accuracy improvement
            }
            WeaponAttachment::Foregrip => {
                self.recoil *= 0.7; // 30% recoil reduction
            }
            WeaponAttachment::Stock => {
                self.accuracy *= 0.9; // 10% accuracy improvement
                self.recoil *= 0.9; // 10% recoil reduction
            }
            _ => {}
        }

        self.attachments.push(attachment);
    }

    /// Check if has attachment type
    pub fn has_attachment(&self, attachment_type: &str) -> bool {
        self.attachments.iter().any(|a| match a {
            WeaponAttachment::Scope { .. } => attachment_type == "scope",
            WeaponAttachment::Suppressor => attachment_type == "suppressor",
            WeaponAttachment::ExtendedMag { .. } => attachment_type == "extended_mag",
            WeaponAttachment::LaserSight => attachment_type == "laser_sight",
            WeaponAttachment::Foregrip => attachment_type == "foregrip",
            WeaponAttachment::Stock => attachment_type == "stock",
        })
    }
}

impl DamageFalloff {
    /// Calculate damage at distance
    pub fn calculate_damage(&self, base_damage: f32, distance: f32) -> f32 {
        if distance <= self.min_distance {
            base_damage
        } else if distance >= self.max_distance {
            base_damage * self.min_damage_multiplier
        } else {
            let t = (distance - self.min_distance) / (self.max_distance - self.min_distance);
            let multiplier = 1.0 - t * (1.0 - self.min_damage_multiplier);
            base_damage * multiplier
        }
    }
}

impl Default for DamageFalloff {
    fn default() -> Self {
        Self {
            min_distance: 10.0,
            max_distance: 100.0,
            min_damage_multiplier: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weapon_inventory_creation() {
        let inventory = WeaponInventory::new(3);
        assert_eq!(inventory.max_weapons, 3);
        assert_eq!(inventory.weapon_count(), 0);
        assert!(inventory.current_weapon.is_none());
        println!("✅ WeaponInventory created");
    }

    #[test]
    fn test_add_weapon() {
        let mut inventory = WeaponInventory::new(3);
        let weapon = Weapon::new("rifle".to_string(), "Assault Rifle".to_string(), WeaponType::Hitscan);
        
        let result = inventory.add_weapon(weapon);
        assert!(result.is_ok());
        assert_eq!(inventory.weapon_count(), 1);
        assert_eq!(inventory.current_weapon, Some(0));
        println!("✅ Weapon added to inventory");
    }

    #[test]
    fn test_weapon_creation() {
        let weapon = Weapon::new("pistol".to_string(), "Pistol".to_string(), WeaponType::Hitscan);
        assert_eq!(weapon.id, "pistol");
        assert_eq!(weapon.name, "Pistol");
        assert_eq!(weapon.weapon_type, WeaponType::Hitscan);
        assert_eq!(weapon.current_ammo, 30);
        println!("✅ Weapon created");
    }

    #[test]
    fn test_weapon_fire() {
        let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
        
        let initial_ammo = weapon.current_ammo;
        assert!(weapon.can_fire());
        
        let fired = weapon.fire();
        assert!(fired);
        assert_eq!(weapon.current_ammo, initial_ammo - 1);
        println!("✅ Weapon fired");
    }

    #[test]
    fn test_damage_falloff() {
        let falloff = DamageFalloff::default();
        let base_damage = 100.0;
        
        // At min distance (full damage)
        let damage1 = falloff.calculate_damage(base_damage, 10.0);
        assert_eq!(damage1, 100.0);
        
        // At max distance (min damage)
        let damage2 = falloff.calculate_damage(base_damage, 100.0);
        assert_eq!(damage2, 50.0);
        
        // Halfway
        let damage3 = falloff.calculate_damage(base_damage, 55.0);
        assert!(damage3 > 50.0 && damage3 < 100.0);
        
        println!("✅ Damage falloff works");
    }
}

