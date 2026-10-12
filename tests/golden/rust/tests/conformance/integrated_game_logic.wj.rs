#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]

struct Stats {
    hp: i64,
    max_hp: i64,
    attack: i64,
}











impl Stats {
#[inline]
fn new(hp: i64, attack: i64) -> Stats {
        Stats { hp, max_hp: hp, attack }
}
#[inline]
fn is_alive(&self) -> bool {
        self.hp > 0_i64
}
#[inline]
fn take_damage(&mut self, amount: i64) {
        self.hp -= amount;
        if self.hp < 0_i64 {
            self.hp = 0_i64;
        }
}
}

#[derive(Debug, Clone, PartialEq)]

struct Player {
    name: String,
    stats: Stats,
    xp: i64,
}

impl Player {
#[inline]
fn new(name: String, hp: i64, attack: i64) -> Player {
        Player { name, stats: Stats::new(hp, attack), xp: 0_i64 }
}
#[inline]
fn attack_enemy(&self, _enemy: Enemy) -> i64 {
        self.stats.attack
}
#[inline]
fn gain_xp(&mut self, amount: i64) {
        self.xp += amount;
}
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum EnemyType {
    Goblin,
    Skeleton,
    Dragon,
}

#[derive(Debug, Clone, Copy, PartialEq)]

struct Enemy {
    enemy_type: EnemyType,
    stats: Stats,
}

impl Enemy {
#[inline]
fn new(enemy_type: EnemyType, hp: i64, attack: i64) -> Enemy {
        Enemy { enemy_type, stats: Stats::new(hp, attack) }
}
#[inline]
fn name(&self) -> String {
        enemy_type_name(self.enemy_type)
}
}

#[inline]
fn enemy_type_name(t: EnemyType) -> String {
    match t {
        EnemyType::Goblin => String::from("Goblin"),
        EnemyType::Skeleton => String::from("Skeleton"),
        EnemyType::Dragon => String::from("Dragon"),
    }
}

#[inline]
fn enemy_xp_reward(t: EnemyType) -> i64 {
    match t {
        EnemyType::Goblin => 50_i64,
        EnemyType::Skeleton => 75_i64,
        EnemyType::Dragon => 200_i64,
    }
}

fn main() {
    let mut player = Player::new("Hero".to_string(), 100_i64, 10_i64);
    println!("[game] Created player: {} (HP: {}, ATK: {})", player.name, player.stats.hp, player.stats.attack);
    let mut enemies: Vec<Enemy> = vec![];
    enemies.push(Enemy::new(EnemyType::Goblin, 30_i64, 5_i64));
    enemies.push(Enemy::new(EnemyType::Skeleton, 40_i64, 8_i64));
    enemies.push(Enemy::new(EnemyType::Dragon, 100_i64, 15_i64));
    println!("[game] Created {} enemies", enemies.len());
    let mut round = 1_i64;
    let mut enemy_idx: usize = 0_usize;
    let max_rounds = 5_i32;
    while round <= max_rounds as i64 && enemy_idx < enemies.len() {
        let defeated = do_combat_round(&mut player, &mut enemies[enemy_idx], round);
        if defeated {
            enemy_idx += 1;
        }
        round += 1;
    }
    println!("[game] --- Battle Summary ---");
    println!("[game] Hero: HP={}, XP={}", player.stats.hp, player.xp);
    let remaining = enemies.len() - enemy_idx;
    println!("[game] Enemies remaining: {}", remaining);
    println!("[game] PASSED");
}

fn do_combat_round(player: &mut Player, enemy: &mut Enemy, round: i64) -> bool {
    println!("[game] --- Round {} ---", round);
    let damage = player.stats.attack;
    enemy.stats.take_damage(damage);
    println!("[game] {} attacks {} for {} damage", player.name, enemy.name(), damage);
    if !enemy.stats.is_alive() {
        let _ = println!("[game] {} defeated!", enemy.name());
        let xp = enemy_xp_reward(enemy.enemy_type);
        let _ = player.gain_xp(xp);
        let _ = println!("[game] {} gains {} XP (total: {})", player.name, xp, player.xp);
        return true;
    }
    println!("[game] {} HP: {}", enemy.name(), enemy.stats.hp);
    let enemy_damage = enemy.stats.attack;
    player.stats.take_damage(enemy_damage);
    println!("[game] {} attacks {} for {} damage", enemy.name(), player.name, enemy_damage);
    println!("[game] {} HP: {}", player.name, player.stats.hp);
    false
}

