//! Unit tests for Weapon System
//!
//! Tests weapon management, firing, reloading, attachments, and damage falloff.

use windjammer_game_framework::weapon_system::*;

// ============================================================================
// WeaponInventory Tests
// ============================================================================

#[test]
fn test_inventory_creation() {
    let inventory = WeaponInventory::new(3);
    assert_eq!(inventory.max_weapons, 3);
    assert_eq!(inventory.weapon_count(), 0);
    assert!(inventory.current_weapon.is_none());
    assert!(!inventory.is_full());
    println!("✅ WeaponInventory created");
}

#[test]
fn test_add_weapon() {
    let mut inventory = WeaponInventory::new(3);
    let weapon = Weapon::new("rifle".to_string(), "Assault Rifle".to_string(), WeaponType::Hitscan);
    
    let result = inventory.add_weapon(weapon);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0);
    assert_eq!(inventory.weapon_count(), 1);
    assert_eq!(inventory.current_weapon, Some(0));
    println!("✅ Weapon added to inventory");
}

#[test]
fn test_add_multiple_weapons() {
    let mut inventory = WeaponInventory::new(3);
    
    inventory.add_weapon(Weapon::new("pistol".to_string(), "Pistol".to_string(), WeaponType::Hitscan)).unwrap();
    inventory.add_weapon(Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan)).unwrap();
    inventory.add_weapon(Weapon::new("shotgun".to_string(), "Shotgun".to_string(), WeaponType::Hitscan)).unwrap();
    
    assert_eq!(inventory.weapon_count(), 3);
    assert!(inventory.is_full());
    println!("✅ Multiple weapons added");
}

#[test]
fn test_inventory_full() {
    let mut inventory = WeaponInventory::new(2);
    
    inventory.add_weapon(Weapon::new("weapon1".to_string(), "Weapon 1".to_string(), WeaponType::Hitscan)).unwrap();
    inventory.add_weapon(Weapon::new("weapon2".to_string(), "Weapon 2".to_string(), WeaponType::Hitscan)).unwrap();
    
    let result = inventory.add_weapon(Weapon::new("weapon3".to_string(), "Weapon 3".to_string(), WeaponType::Hitscan));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Inventory full");
    println!("✅ Inventory full handling works");
}

#[test]
fn test_remove_weapon() {
    let mut inventory = WeaponInventory::new(3);
    
    inventory.add_weapon(Weapon::new("weapon1".to_string(), "Weapon 1".to_string(), WeaponType::Hitscan)).unwrap();
    inventory.add_weapon(Weapon::new("weapon2".to_string(), "Weapon 2".to_string(), WeaponType::Hitscan)).unwrap();
    
    let removed = inventory.remove_weapon(0);
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id, "weapon1");
    assert_eq!(inventory.weapon_count(), 1);
    println!("✅ Weapon removed from inventory");
}

#[test]
fn test_switch_weapon() {
    let mut inventory = WeaponInventory::new(3);
    
    inventory.add_weapon(Weapon::new("weapon1".to_string(), "Weapon 1".to_string(), WeaponType::Hitscan)).unwrap();
    inventory.add_weapon(Weapon::new("weapon2".to_string(), "Weapon 2".to_string(), WeaponType::Hitscan)).unwrap();
    
    assert_eq!(inventory.current_weapon, Some(0));
    
    inventory.switch_weapon(1);
    assert_eq!(inventory.current_weapon, Some(1));
    println!("✅ Weapon switching works");
}

#[test]
fn test_next_previous_weapon() {
    let mut inventory = WeaponInventory::new(3);
    
    inventory.add_weapon(Weapon::new("weapon1".to_string(), "Weapon 1".to_string(), WeaponType::Hitscan)).unwrap();
    inventory.add_weapon(Weapon::new("weapon2".to_string(), "Weapon 2".to_string(), WeaponType::Hitscan)).unwrap();
    inventory.add_weapon(Weapon::new("weapon3".to_string(), "Weapon 3".to_string(), WeaponType::Hitscan)).unwrap();
    
    assert_eq!(inventory.current_weapon, Some(0));
    
    inventory.next_weapon();
    assert_eq!(inventory.current_weapon, Some(1));
    
    inventory.next_weapon();
    assert_eq!(inventory.current_weapon, Some(2));
    
    inventory.next_weapon(); // Should wrap to 0
    assert_eq!(inventory.current_weapon, Some(0));
    
    inventory.previous_weapon(); // Should wrap to 2
    assert_eq!(inventory.current_weapon, Some(2));
    
    println!("✅ Next/previous weapon cycling works");
}

#[test]
fn test_get_current_weapon() {
    let mut inventory = WeaponInventory::new(3);
    
    inventory.add_weapon(Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan)).unwrap();
    
    let current = inventory.get_current_weapon();
    assert!(current.is_some());
    assert_eq!(current.unwrap().id, "rifle");
    println!("✅ Get current weapon works");
}

// ============================================================================
// Weapon Tests
// ============================================================================

#[test]
fn test_weapon_creation() {
    let weapon = Weapon::new("pistol".to_string(), "Pistol".to_string(), WeaponType::Hitscan);
    assert_eq!(weapon.id, "pistol");
    assert_eq!(weapon.name, "Pistol");
    assert_eq!(weapon.weapon_type, WeaponType::Hitscan);
    assert_eq!(weapon.current_ammo, 30);
    assert_eq!(weapon.reserve_ammo, 90);
    assert!(!weapon.is_reloading);
    println!("✅ Weapon created");
}

#[test]
fn test_weapon_types() {
    assert_ne!(WeaponType::Hitscan, WeaponType::Projectile);
    assert_ne!(WeaponType::Projectile, WeaponType::Melee);
    assert_ne!(WeaponType::Melee, WeaponType::Explosive);
    assert_eq!(WeaponType::Hitscan, WeaponType::Hitscan);
    println!("✅ WeaponType enum works");
}

#[test]
fn test_weapon_can_fire() {
    let weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    assert!(weapon.can_fire(), "New weapon should be able to fire");
    println!("✅ Weapon can_fire check works");
}

#[test]
fn test_weapon_fire() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    
    let initial_ammo = weapon.current_ammo;
    let fired = weapon.fire();
    
    assert!(fired, "Weapon should fire successfully");
    assert_eq!(weapon.current_ammo, initial_ammo - 1);
    assert_eq!(weapon.time_since_last_shot, 0.0);
    println!("✅ Weapon firing works");
}

#[test]
fn test_weapon_empty() {
    let mut weapon = Weapon::new("pistol".to_string(), "Pistol".to_string(), WeaponType::Hitscan);
    weapon.current_ammo = 0;
    
    assert!(!weapon.can_fire(), "Empty weapon should not be able to fire");
    let fired = weapon.fire();
    assert!(!fired, "Empty weapon should not fire");
    println!("✅ Empty weapon handling works");
}

#[test]
fn test_fire_rate_cooldown() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    weapon.fire_rate = 10.0; // 10 shots per second = 0.1s between shots
    
    weapon.fire();
    assert!(!weapon.can_fire(), "Should not be able to fire immediately");
    
    weapon.update(0.05); // Half the cooldown
    assert!(!weapon.can_fire(), "Should still be on cooldown");
    
    weapon.update(0.05); // Complete the cooldown
    assert!(weapon.can_fire(), "Should be able to fire after cooldown");
    
    println!("✅ Fire rate cooldown works");
}

// ============================================================================
// Reload Tests
// ============================================================================

#[test]
fn test_start_reload() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    weapon.current_ammo = 10; // Partially empty
    
    let started = weapon.start_reload();
    assert!(started, "Reload should start");
    assert!(weapon.is_reloading);
    assert_eq!(weapon.reload_progress, 0.0);
    println!("✅ Reload starts correctly");
}

#[test]
fn test_reload_full_magazine() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    
    let started = weapon.start_reload();
    assert!(!started, "Should not reload with full magazine");
    println!("✅ Full magazine reload prevention works");
}

#[test]
fn test_reload_no_ammo() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    weapon.current_ammo = 0;
    weapon.reserve_ammo = 0;
    
    let started = weapon.start_reload();
    assert!(!started, "Should not reload with no reserve ammo");
    println!("✅ No ammo reload prevention works");
}

#[test]
fn test_reload_completion() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    weapon.current_ammo = 10;
    weapon.reload_time = 2.0;
    
    weapon.start_reload();
    
    // Update for full reload time
    weapon.update(2.0);
    
    assert!(!weapon.is_reloading, "Should have finished reloading");
    assert_eq!(weapon.current_ammo, weapon.magazine_size);
    println!("✅ Reload completion works");
}

#[test]
fn test_partial_reload() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    weapon.current_ammo = 10;
    weapon.reserve_ammo = 5; // Not enough to fill magazine
    
    weapon.start_reload();
    weapon.update(2.0); // Complete reload
    
    assert_eq!(weapon.current_ammo, 15); // 10 + 5
    assert_eq!(weapon.reserve_ammo, 0);
    println!("✅ Partial reload works");
}

// ============================================================================
// Ammo Management Tests
// ============================================================================

#[test]
fn test_add_ammo() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    let initial_reserve = weapon.reserve_ammo;
    
    weapon.add_ammo(30);
    assert_eq!(weapon.reserve_ammo, initial_reserve + 30);
    println!("✅ Add ammo works");
}

#[test]
fn test_add_ammo_max_cap() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    weapon.reserve_ammo = weapon.max_reserve_ammo - 10;
    
    weapon.add_ammo(50); // Try to add more than cap allows
    assert_eq!(weapon.reserve_ammo, weapon.max_reserve_ammo);
    println!("✅ Ammo max cap works");
}

// ============================================================================
// Damage Falloff Tests
// ============================================================================

#[test]
fn test_damage_falloff_creation() {
    let falloff = DamageFalloff::default();
    assert_eq!(falloff.min_distance, 10.0);
    assert_eq!(falloff.max_distance, 100.0);
    assert_eq!(falloff.min_damage_multiplier, 0.5);
    println!("✅ DamageFalloff created");
}

#[test]
fn test_damage_at_min_distance() {
    let falloff = DamageFalloff::default();
    let damage = falloff.calculate_damage(100.0, 10.0);
    assert_eq!(damage, 100.0, "Should deal full damage at min distance");
    println!("✅ Full damage at min distance");
}

#[test]
fn test_damage_at_max_distance() {
    let falloff = DamageFalloff::default();
    let damage = falloff.calculate_damage(100.0, 100.0);
    assert_eq!(damage, 50.0, "Should deal 50% damage at max distance");
    println!("✅ Reduced damage at max distance");
}

#[test]
fn test_damage_falloff_curve() {
    let falloff = DamageFalloff::default();
    
    let damage_close = falloff.calculate_damage(100.0, 30.0);
    let damage_mid = falloff.calculate_damage(100.0, 55.0);
    let damage_far = falloff.calculate_damage(100.0, 80.0);
    
    assert!(damage_close > damage_mid, "Closer should deal more damage");
    assert!(damage_mid > damage_far, "Mid-range should deal more than far");
    assert!(damage_far > 50.0, "Far should still deal more than min");
    
    println!("✅ Damage falloff curve works: close={}, mid={}, far={}", damage_close, damage_mid, damage_far);
}

#[test]
fn test_weapon_calculate_damage() {
    let weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    
    let damage_close = weapon.calculate_damage(10.0);
    let damage_far = weapon.calculate_damage(100.0);
    
    assert!(damage_close >= damage_far, "Close range should deal more or equal damage");
    println!("✅ Weapon damage calculation works");
}

// ============================================================================
// Attachment Tests
// ============================================================================

#[test]
fn test_add_attachment() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    
    weapon.add_attachment(WeaponAttachment::Scope { magnification: 4 });
    assert_eq!(weapon.attachments.len(), 1);
    println!("✅ Attachment added");
}

#[test]
fn test_extended_mag_attachment() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    let initial_mag_size = weapon.magazine_size;
    
    weapon.add_attachment(WeaponAttachment::ExtendedMag { bonus_ammo: 10 });
    assert_eq!(weapon.magazine_size, initial_mag_size + 10);
    println!("✅ Extended mag attachment works");
}

#[test]
fn test_laser_sight_attachment() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    let initial_accuracy = weapon.accuracy;
    
    weapon.add_attachment(WeaponAttachment::LaserSight);
    assert!(weapon.accuracy < initial_accuracy, "Laser sight should improve accuracy");
    println!("✅ Laser sight attachment works");
}

#[test]
fn test_foregrip_attachment() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    let initial_recoil = weapon.recoil;
    
    weapon.add_attachment(WeaponAttachment::Foregrip);
    assert!(weapon.recoil < initial_recoil, "Foregrip should reduce recoil");
    println!("✅ Foregrip attachment works");
}

#[test]
fn test_stock_attachment() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    let initial_accuracy = weapon.accuracy;
    let initial_recoil = weapon.recoil;
    
    weapon.add_attachment(WeaponAttachment::Stock);
    assert!(weapon.accuracy < initial_accuracy, "Stock should improve accuracy");
    assert!(weapon.recoil < initial_recoil, "Stock should reduce recoil");
    println!("✅ Stock attachment works");
}

#[test]
fn test_has_attachment() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    
    weapon.add_attachment(WeaponAttachment::Scope { magnification: 4 });
    weapon.add_attachment(WeaponAttachment::Suppressor);
    
    assert!(weapon.has_attachment("scope"));
    assert!(weapon.has_attachment("suppressor"));
    assert!(!weapon.has_attachment("foregrip"));
    println!("✅ Has attachment check works");
}

#[test]
fn test_multiple_attachments() {
    let mut weapon = Weapon::new("rifle".to_string(), "Rifle".to_string(), WeaponType::Hitscan);
    
    weapon.add_attachment(WeaponAttachment::Scope { magnification: 4 });
    weapon.add_attachment(WeaponAttachment::ExtendedMag { bonus_ammo: 10 });
    weapon.add_attachment(WeaponAttachment::Foregrip);
    
    assert_eq!(weapon.attachments.len(), 3);
    println!("✅ Multiple attachments work");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_combat_scenario() {
    let mut inventory = WeaponInventory::new(2);
    
    // Add weapons
    let mut rifle = Weapon::new("rifle".to_string(), "Assault Rifle".to_string(), WeaponType::Hitscan);
    rifle.fire_rate = 10.0;
    rifle.magazine_size = 30;
    rifle.current_ammo = 30;
    
    inventory.add_weapon(rifle).unwrap();
    
    // Fire some shots
    for _ in 0..10 {
        if let Some(weapon) = inventory.get_current_weapon_mut() {
            if weapon.can_fire() {
                weapon.fire();
            }
            weapon.update(0.1); // Simulate frame
        }
    }
    
    let weapon = inventory.get_current_weapon().unwrap();
    assert!(weapon.current_ammo < 30, "Should have fired some shots");
    
    println!("✅ Combat scenario works: ammo={}", weapon.current_ammo);
}

