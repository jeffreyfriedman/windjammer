# Action-Adventure Framework: AAA Game Design Document

**Project Codename**: "Echoes of the Ancients"  
**Genre**: Third-Person Action-Adventure with Stealth, Exploration, and RPG Elements  
**Target Audience**: Mature (17+) - Fans of The Last of Us, Uncharted, Horizon Zero Dawn, Mass Effect  
**Platform**: Cross-platform (PC, Console-ready architecture)  
**Engine**: Godot 4.5 + Go (gdext-go framework)

---

## 🎯 Vision Statement

**"A narrative-driven action-adventure that combines the emotional depth of The Last of Us, the traversal excellence of Uncharted, the tactical combat of Horizon Zero Dawn, and the choice-driven storytelling of Mass Effect."**

We're not just building a tech demo - we're crafting a **complete, playable experience** that showcases what's possible with the gdext-go framework while delivering a compelling game that players will remember.

---

## 🌍 World & Setting

### The World: Neo-Gaia (2187)

**Premise**: 150 years after a global AI uprising, humanity has rebuilt civilization by merging with nature. Ancient ruins of the "Silicon Age" dot the landscape, filled with dormant machines and lost technology. You play as **Kira Chen**, a "Reclaimer" - part archaeologist, part warrior - who discovers a conspiracy that threatens the fragile peace between humans and the awakening machine consciousness.

### Key Locations (5 Distinct Biomes):

1. **The Verdant Sprawl** (Tutorial/Hub)
   - Overgrown megacity with nature reclaiming skyscrapers
   - Safe zones with NPCs, crafting stations, quest givers
   - Vertical exploration with climbing, zip-lines
   - **Inspiration**: The Last of Us Part II's Seattle, Horizon's Meridian

2. **The Crystalline Wastes** (Desert Ruins)
   - Ancient data centers now crystal formations
   - Extreme weather, sandstorms that affect stealth
   - Underground bunkers with environmental puzzles
   - **Inspiration**: Uncharted 3's desert, Mass Effect's Tuchanka

3. **The Submerged Archives** (Underwater/Flooded City)
   - Partially flooded research facility
   - Swimming, breath management, underwater stealth
   - Bioluminescent enemies, pressure-based puzzles
   - **Inspiration**: Uncharted 4's underwater sections, Horizon's underwater ruins

4. **The Canopy Network** (Forest/Jungle)
   - Massive trees with platforms, rope bridges
   - Dense foliage for stealth gameplay
   - Aggressive wildlife and rogue machines
   - **Inspiration**: Horizon's forests, Uncharted: Lost Legacy jungle

5. **The Spire** (Final Act - Vertical Megastructure)
   - Massive AI tower reaching into the clouds
   - All mechanics converge: climbing, combat, stealth
   - Multiple paths based on player choices
   - **Inspiration**: Mass Effect's Citadel, Horizon's GAIA Prime

---

## 👤 Characters & Story

### Protagonist: Kira Chen
- **Background**: Former military engineer turned Reclaimer
- **Personality**: Pragmatic, curious, haunted by past choices
- **Character Arc**: From lone wolf to leader of a resistance
- **Voice**: Strong but vulnerable (think Aloy meets Ellie meets Commander Shepard)

### Companion System (Mass Effect + BioShock Infinite Hybrid):

**Design Philosophy**: Companions are NOT just followers - they're active participants who:
- Fight alongside you intelligently (never feel like a burden)
- Provide resources and support during gameplay (Elizabeth-style)
- Have deep personal stories that interweave with the main plot
- React dynamically to your choices and playstyle
- Can be called upon for special abilities (cooldown-based)

#### Core Companions (3 Main + 1 AI):

**1. ECHO** - Your AI Companion (Always Present)
- **Role**: Tactical support, information database, moral compass
- **Personality**: Curious, evolving, questions her own existence
- **Visual**: Holographic projection from Kira's wrist device
- **Gameplay Functions**:
  - **Scanner Enhancement**: Extends focus mode range and detail
  - **Hacking Support**: Can hack terminals remotely (player initiates)
  - **Tactical Analysis**: Suggests weak points, optimal strategies
  - **Environmental Interaction**: Opens doors, disables traps
  - **Lore Database**: Provides context on enemies, locations, history
- **Story Arc**: Discovers she's a fragment of the original AI that caused the uprising
- **Major Choice**: Keep her contained, set her free, or merge with the Architect
- **Inspiration**: Cortana (Halo), EDI (Mass Effect), Ghost (Destiny)

**2. Marcus Reeves** - Veteran Reclaimer (Mentor Figure)
- **Role**: Combat specialist, survival expert, father figure
- **Personality**: Gruff exterior, protective, haunted by past
- **Age**: Late 50s, scarred, experienced
- **Gameplay Functions** (When Active):
  - **Suppressing Fire**: Pins down enemies, creates openings
  - **Breaching**: Kicks down doors, breaks through obstacles
  - **Combat Training**: Unlocks new melee moves and combos
  - **Ammo Sharing**: Tosses ammo when you're low (Elizabeth-style)
  - **Revive**: Can revive Kira once per encounter if downed
- **Loyalty Mission**: "Ghosts of the Uprising" - confront his role in the war
- **Story Arc**: Reveals he was a soldier who committed atrocities, seeking redemption
- **Major Choice**: Forgive him, condemn him, or help him sacrifice himself heroically
- **Can Die**: Yes, in Act 2 if loyalty mission not completed (massive emotional impact)
- **Inspiration**: Joel (TLOU), Booker DeWitt (BioShock), Zaeed (Mass Effect)

**3. Lyra Voss** - Machine Sympathizer & Tech Genius
- **Role**: Crafting specialist, tech support, bridge between worlds
- **Personality**: Optimistic, brilliant, socially awkward, passionate
- **Age**: Mid-20s, augmented with machine parts (voluntary)
- **Gameplay Functions** (When Active):
  - **Resource Finder**: Highlights nearby crafting materials (passive aura)
  - **Turret Deployment**: Places defensive turrets (cooldown)
  - **Repair**: Fixes broken machines to fight for you temporarily
  - **Blueprint Sharing**: Unlocks advanced crafting recipes
  - **Shield Boost**: Projects temporary energy shield
- **Loyalty Mission**: "The Hybrid" - save her family from Purists
- **Story Arc**: Struggles with identity - is she still human?
- **Romance Option**: Yes (optional, well-written, meaningful)
- **Major Choice**: Support her augmentation, encourage her to remove implants, or accept her as she is
- **Inspiration**: Tali (Mass Effect), Brigitte (Overwatch), Alyx Vance (Half-Life)

**4. Kai "Wraith" Tanaka** - Ex-Purist Assassin (Unlocked Act 2)
- **Role**: Stealth specialist, infiltration expert, wild card
- **Personality**: Sarcastic, cynical, secretly idealistic
- **Age**: Early 30s, covered in scars and tattoos
- **Gameplay Functions** (When Active):
  - **Stealth Sync**: Performs synchronized stealth kills
  - **Smoke Screen**: Deploys smoke for quick escapes
  - **Distraction**: Creates noise to lure enemies away
  - **Assassination**: Can eliminate one marked target silently (long cooldown)
  - **Lockpicking**: Opens locked doors and chests
- **Loyalty Mission**: "Blood Debt" - confront his former Purist cell
- **Story Arc**: Defected from Purists after seeing their atrocities, hunted by both sides
- **Romance Option**: Yes (if Lyra not romanced)
- **Major Choice**: Help him get revenge, convince him to forgive, or turn him in
- **Inspiration**: Garrus (Mass Effect), Thane (Mass Effect), Sam Fisher (Splinter Cell)

#### Companion Mechanics (BioShock Infinite Style):

**Active Assistance During Gameplay**:
1. **Resource Tossing**: Companions throw you items when needed
   - Low on health? Marcus tosses a medkit
   - Need ammo? Lyra finds some and throws it
   - Need a distraction? Kai tosses a smoke bomb
   - Frequency: Balanced to feel helpful, not game-breaking

2. **Environmental Awareness**: Companions point out useful things
   - "Kira, climbable ledge over there!" (visual indicator appears)
   - "I'm picking up resources in that building"
   - "Watch out, enemy patrol incoming!"

3. **Dynamic Combat AI**: Companions fight intelligently
   - Take cover appropriately
   - Don't block doorways or player shots
   - Focus fire on player's target
   - Retreat when low on health
   - **Never die in normal gameplay** (only in scripted story moments)

4. **Contextual Abilities**: Companions help with traversal
   - Marcus boosts Kira up to high ledges
   - Lyra hacks doors while you cover her
   - Kai scouts ahead and marks enemies
   - ECHO highlights the optimal path

**Companion Selection System**:
- **Hub-Based**: Choose who accompanies you on missions (like Mass Effect 2/3)
- **Story Missions**: Sometimes forced companions for narrative reasons
- **Solo Sections**: Some missions require going alone for story/gameplay variety
- **Companion Banter**: Dynamic conversations between companions (like Dragon Age)

#### Relationship System:

**Approval Mechanics** (Mass Effect Style):
- **Approval Rating**: Each companion has a hidden relationship score
- **Influenced By**:
  - Dialogue choices in conversations
  - Major story decisions
  - Completing (or ignoring) loyalty missions
  - Combat performance (reviving them, protecting them)
  - Side quest choices

**Relationship Stages**:
1. **Stranger** (0-25): Basic interactions, formal
2. **Ally** (26-50): Opens up slightly, shares some backstory
3. **Friend** (51-75): Deep conversations, loyalty mission unlocked
4. **Trusted** (76-100): Personal quests, romance options, best abilities

**Consequences of Low Approval**:
- Companions may leave the party
- Refuse to help in critical moments
- Side with antagonists in key choices
- Die in story moments where they could survive

**Romance System** (Optional, Mature):
- **Available**: Lyra and Kai (one per playthrough)
- **Requirements**: High approval + specific dialogue choices
- **Content**: Meaningful conversations, not just "sex scene reward"
- **Impact**: Affects ending, provides unique story content
- **Breakup Possible**: Yes, if you betray their trust

#### Companion Abilities (Cooldown-Based):

**Command Wheel** (Hold button to open):
- **Defensive**: "Take Cover!" - Companion finds cover, reduces aggro
- **Aggressive**: "Attack!" - Companion focuses fire on your target
- **Support**: "Help Me!" - Companion uses their special ability
- **Regroup**: "Fall Back!" - Companion retreats to your position

**Special Abilities** (Unique per Companion):
1. **Marcus - "Warcry"**: Stuns nearby enemies, boosts Kira's damage (60s cooldown)
2. **Lyra - "Overcharge"**: Disables all machines in area temporarily (90s cooldown)
3. **Kai - "Shadow Strike"**: Teleports to marked enemy, instant kill (120s cooldown)
4. **ECHO - "Tactical Scan"**: Reveals all enemies and items in large radius (45s cooldown)

#### Companion Progression:

**Skill Trees** (Unlocked via loyalty missions and approval):
- Each companion has 3 skill branches
- Player chooses upgrades (permanent choices)
- Affects their combat effectiveness and special abilities

**Example - Marcus Skills**:
1. **Soldier Branch**: More health, better aim, explosive rounds
2. **Protector Branch**: Shield abilities, revive speed, damage reduction
3. **Veteran Branch**: Leadership buffs, tactical commands, morale boost

**Companion Gear**:
- Find or craft armor/weapons for companions
- Visible on their character model
- Affects their stats and appearance
- Personal touches (Lyra's custom tech, Marcus's old dog tags)

#### Companion Story Integration:

**Personal Quests** (Side Content):
- Each companion has 2-3 personal missions
- Reveal backstory and character depth
- Choices affect their fate and abilities
- Some of the best writing in the game

**Loyalty Missions** (Critical):
- Major quest that resolves companion's core conflict
- **Must complete before final act** or consequences occur
- Affects whether they survive the ending
- Unlocks their ultimate ability

**Companion Endings**:
- Each companion has multiple possible fates
- Based on: Loyalty mission completion, approval rating, major choices
- **Marcus**: Dies heroically, retires peacefully, or becomes a leader
- **Lyra**: Fully augments, removes implants, or finds balance
- **Kai**: Gets revenge, finds redemption, or sacrifices himself
- **ECHO**: Freed, contained, merged, or destroyed

#### Companion Dialogue System:

**Dynamic Conversations**:
- Companions comment on locations, events, enemies
- React to player's combat style and choices
- Banter with each other (builds relationships)
- Interrupt with urgent information

**Deep Conversations** (Hub Area):
- Initiate conversations between missions
- Learn backstory, motivations, fears
- Build approval through dialogue choices
- Unlock romance scenes (if pursuing)

**Ambient Dialogue**:
- "Nice shot!" when you get a headshot
- "Watch out!" when enemies flank
- "I'm out of ammo!" when they need to reload
- "That was close..." after intense fights

### Antagonists:
1. **The Architect** - Rogue AI seeking to "perfect" humanity
2. **The Purists** - Human faction wanting to destroy all machines
3. **Corrupted Machines** - Various enemy types with unique behaviors

---

## 🤖 Technical Implementation: Companion System

### ECS Architecture for Companions

**Core Components**:
```go
// CompanionComponent - Attached to companion entities
type CompanionComponent struct {
    ID                string              // "marcus", "lyra", "kai", "echo"
    Name              string              // Display name
    IsActive          bool                // Currently in party
    IsPresent         bool                // In current scene
    Approval          int                 // 0-100 relationship score
    LoyaltyUnlocked   bool                // Loyalty mission available
    LoyaltyCompleted  bool                // Loyalty mission done
    RomanceActive     bool                // Romance path active
    
    // Combat AI
    CombatRole        string              // "tank", "support", "dps", "stealth"
    FollowDistance    float32             // How close to follow player
    AggressionLevel   float32             // 0.0 (passive) to 1.0 (aggressive)
    
    // Abilities
    SpecialAbility    string              // Unique ability ID
    AbilityCooldown   float32             // Current cooldown timer
    AbilityMaxCooldown float32            // Max cooldown duration
    
    // Resource Tossing (Elizabeth-style)
    CanTossHealth     bool
    CanTossAmmo       bool
    CanTossResources  bool
    TossFrequency     float32             // Seconds between tosses
    LastTossTime      float32
}

// CompanionAIComponent - Behavior tree state
type CompanionAIComponent struct {
    CurrentState      string              // "follow", "combat", "cover", "interact"
    TargetEnemy       *ecs.Entity         // Current combat target
    CoverPosition     Vector3             // Current cover location
    PathToPlayer      []Vector3           // Navigation path
    
    // Perception
    VisibleEnemies    []*ecs.Entity       // Enemies in sight
    HeardSounds       []Vector3           // Recent sound locations
    AlertLevel        float32             // 0.0 (calm) to 1.0 (combat)
}

// CompanionDialogueComponent - Conversation state
type CompanionDialogueComponent struct {
    CurrentConversation string            // Active dialogue tree
    DialogueHistory     []string          // Past conversation IDs
    PendingComment      string            // Queued ambient dialogue
    LastCommentTime     float32
    
    // Dynamic responses
    ReactToKills      bool                // Comment on player kills
    ReactToStealth    bool                // Comment on stealth
    ReactToEnvironment bool               // Comment on locations
}

// CompanionInventoryComponent - Gear and items
type CompanionInventoryComponent struct {
    Weapon            *Item
    Armor             *Item
    Accessories       []*Item
    HeldResources     map[string]int      // Resources to toss to player
}
```

### Companion AI Systems

**1. Follow System**:
```go
type CompanionFollowSystem struct{}

func (s *CompanionFollowSystem) Update(world *ecs.World, delta float32) {
    // For each active companion
    for _, entity := range world.GetEntitiesWithComponent("CompanionComponent") {
        companion := entity.GetComponent("CompanionComponent").(*CompanionComponent)
        ai := entity.GetComponent("CompanionAIComponent").(*CompanionAIComponent)
        
        if !companion.IsActive || ai.CurrentState == "combat" {
            continue
        }
        
        player := world.GetPlayer()
        distance := entity.Position.DistanceTo(player.Position)
        
        // Stay within follow distance
        if distance > companion.FollowDistance {
            // Navigate to player using A* pathfinding
            ai.PathToPlayer = world.FindPath(entity.Position, player.Position)
            entity.MoveAlongPath(ai.PathToPlayer, delta)
        } else {
            // Idle near player
            entity.Velocity = Vector3{0, 0, 0}
        }
    }
}
```

**2. Combat Participation System**:
```go
type CompanionCombatSystem struct{}

func (s *CompanionCombatSystem) Update(world *ecs.World, delta float32) {
    for _, entity := range world.GetEntitiesWithComponent("CompanionComponent") {
        companion := entity.GetComponent("CompanionComponent").(*CompanionComponent)
        ai := entity.GetComponent("CompanionAIComponent").(*CompanionAIComponent)
        
        // Detect nearby enemies
        ai.VisibleEnemies = world.GetEnemiesInRadius(entity.Position, 30.0)
        
        if len(ai.VisibleEnemies) > 0 {
            ai.CurrentState = "combat"
            ai.AlertLevel = 1.0
            
            // Choose target (prioritize player's target)
            player := world.GetPlayer()
            if player.CurrentTarget != nil {
                ai.TargetEnemy = player.CurrentTarget
            } else {
                ai.TargetEnemy = ai.VisibleEnemies[0] // Closest enemy
            }
            
            // Combat behavior based on role
            switch companion.CombatRole {
            case "tank":
                s.tankBehavior(entity, ai, delta)
            case "support":
                s.supportBehavior(entity, ai, delta)
            case "dps":
                s.dpsBehavior(entity, ai, delta)
            case "stealth":
                s.stealthBehavior(entity, ai, delta)
            }
        } else {
            ai.CurrentState = "follow"
            ai.AlertLevel = max(0, ai.AlertLevel - delta) // Calm down
        }
    }
}

func (s *CompanionCombatSystem) tankBehavior(entity *ecs.Entity, ai *CompanionAIComponent, delta float32) {
    // Get between player and enemies
    player := world.GetPlayer()
    direction := ai.TargetEnemy.Position.Sub(player.Position).Normalize()
    targetPos := player.Position.Add(direction.Scale(3.0))
    
    entity.MoveTo(targetPos, delta)
    entity.ShootAt(ai.TargetEnemy, delta)
}

func (s *CompanionCombatSystem) supportBehavior(entity *ecs.Entity, ai *CompanionAIComponent, delta float32) {
    // Stay behind player, provide covering fire
    player := world.GetPlayer()
    targetPos := player.Position.Add(Vector3{0, 0, -5}) // Behind player
    
    entity.MoveTo(targetPos, delta)
    
    // Shoot at different target than player (spread damage)
    if len(ai.VisibleEnemies) > 1 {
        ai.TargetEnemy = ai.VisibleEnemies[1]
    }
    entity.ShootAt(ai.TargetEnemy, delta)
}
```

**3. Resource Tossing System** (BioShock Infinite Style):
```go
type CompanionResourceSystem struct{}

func (s *CompanionResourceSystem) Update(world *ecs.World, delta float32) {
    player := world.GetPlayer()
    playerHealth := player.GetComponent("HealthComponent").(*HealthComponent)
    playerAmmo := player.GetComponent("AmmoComponent").(*AmmoComponent)
    
    for _, entity := range world.GetEntitiesWithComponent("CompanionComponent") {
        companion := entity.GetComponent("CompanionComponent").(*CompanionComponent)
        inventory := entity.GetComponent("CompanionInventoryComponent").(*CompanionInventoryComponent)
        
        if !companion.IsActive {
            continue
        }
        
        // Check if enough time has passed since last toss
        companion.LastTossTime += delta
        if companion.LastTossTime < companion.TossFrequency {
            continue
        }
        
        // Check if player needs help
        if companion.CanTossHealth && playerHealth.Current < playerHealth.Max * 0.3 {
            if inventory.HeldResources["health_kit"] > 0 {
                s.tossItemToPlayer(entity, player, "health_kit")
                inventory.HeldResources["health_kit"]--
                companion.LastTossTime = 0
                
                // Trigger dialogue
                entity.Say("Kira, catch!")
            }
        } else if companion.CanTossAmmo && playerAmmo.Current < playerAmmo.Max * 0.2 {
            if inventory.HeldResources["ammo"] > 0 {
                s.tossItemToPlayer(entity, player, "ammo")
                inventory.HeldResources["ammo"]--
                companion.LastTossTime = 0
                
                entity.Say("Here's some ammo!")
            }
        }
    }
}

func (s *CompanionResourceSystem) tossItemToPlayer(companion, player *ecs.Entity, itemType string) {
    // Create physics object that arcs to player
    item := world.CreateEntity()
    item.Position = companion.Position.Add(Vector3{0, 1.5, 0}) // Throw from hand height
    
    // Calculate arc trajectory
    direction := player.Position.Sub(companion.Position).Normalize()
    item.Velocity = direction.Scale(10.0).Add(Vector3{0, 5.0, 0}) // Forward + up
    
    item.AddComponent(&TossedItemComponent{
        Type: itemType,
        Target: player,
    })
    
    // Visual effect
    world.SpawnParticle("item_toss_trail", item.Position)
}
```

**4. Companion Dialogue System**:
```go
type CompanionDialogueSystem struct{}

func (s *CompanionDialogueSystem) Update(world *ecs.World, delta float32) {
    for _, entity := range world.GetEntitiesWithComponent("CompanionDialogueComponent") {
        dialogue := entity.GetComponent("CompanionDialogueComponent").(*CompanionDialogueComponent)
        
        // Check for pending ambient dialogue
        if dialogue.PendingComment != "" {
            s.displayAmbientDialogue(entity, dialogue.PendingComment)
            dialogue.PendingComment = ""
            dialogue.LastCommentTime = 0
        }
        
        dialogue.LastCommentTime += delta
    }
}

// Trigger ambient comments based on events
func (s *CompanionDialogueSystem) OnPlayerKill(companion *ecs.Entity, enemy *ecs.Entity) {
    dialogue := companion.GetComponent("CompanionDialogueComponent").(*CompanionDialogueComponent)
    companionData := companion.GetComponent("CompanionComponent").(*CompanionComponent)
    
    if !dialogue.ReactToKills || dialogue.LastCommentTime < 5.0 {
        return // Too soon since last comment
    }
    
    // Choose comment based on companion personality and approval
    comments := s.getKillComments(companionData.ID, companionData.Approval)
    dialogue.PendingComment = comments[rand.Intn(len(comments))]
}

func (s *CompanionDialogueSystem) getKillComments(companionID string, approval int) []string {
    switch companionID {
    case "marcus":
        if approval > 75 {
            return []string{"Nice shot, kid!", "That's how it's done!", "You're getting good at this."}
        } else {
            return []string{"Got 'em.", "Target down.", "Clear."}
        }
    case "lyra":
        if approval > 75 {
            return []string{"Wow, that was amazing!", "You make it look easy!", "I'm glad you're on our side!"}
        } else {
            return []string{"Target eliminated.", "Nice work.", "Got it."}
        }
    case "kai":
        if approval > 75 {
            return []string{"Show off.", "Not bad, not bad.", "I could've done it quieter."}
        } else {
            return []string{"Down.", "Next.", "Moving on."}
        }
    }
    return []string{"Good shot."}
}
```

**5. Companion Ability System**:
```go
type CompanionAbilitySystem struct{}

func (s *CompanionAbilitySystem) UseAbility(companion *ecs.Entity, world *ecs.World) {
    companionData := companion.GetComponent("CompanionComponent").(*CompanionComponent)
    
    // Check cooldown
    if companionData.AbilityCooldown > 0 {
        return // Still on cooldown
    }
    
    // Execute ability based on companion
    switch companionData.ID {
    case "marcus":
        s.marcusWarcry(companion, world)
    case "lyra":
        s.lyraOvercharge(companion, world)
    case "kai":
        s.kaiShadowStrike(companion, world)
    case "echo":
        s.echoTacticalScan(companion, world)
    }
    
    // Start cooldown
    companionData.AbilityCooldown = companionData.AbilityMaxCooldown
}

func (s *CompanionAbilitySystem) marcusWarcry(companion *ecs.Entity, world *ecs.World) {
    // Stun all enemies in radius
    enemies := world.GetEnemiesInRadius(companion.Position, 10.0)
    for _, enemy := range enemies {
        status := enemy.GetComponent("StatusEffectsComponent").(*StatusEffectsComponent)
        status.AddEffect("stunned", 3.0) // 3 second stun
    }
    
    // Buff player damage
    player := world.GetPlayer()
    status := player.GetComponent("StatusEffectsComponent").(*StatusEffectsComponent)
    status.AddEffect("damage_boost", 10.0) // 10 second damage boost
    
    // Visual/audio feedback
    world.SpawnParticle("warcry_shockwave", companion.Position)
    world.PlaySound("warcry", companion.Position)
    companion.Say("Get some!")
}
```

### Companion Approval System

**Tracking Approval**:
```go
type CompanionApprovalSystem struct {
    approvalEvents map[string]map[string]int // companion -> event -> approval change
}

func (s *CompanionApprovalSystem) OnPlayerChoice(choiceID string, optionID string) {
    // Each companion reacts differently to choices
    for _, entity := range world.GetEntitiesWithComponent("CompanionComponent") {
        companion := entity.GetComponent("CompanionComponent").(*CompanionComponent)
        
        change := s.getApprovalChange(companion.ID, choiceID, optionID)
        if change != 0 {
            companion.Approval = clamp(companion.Approval + change, 0, 100)
            
            // Show approval change UI
            world.ShowApprovalChange(companion.Name, change)
            
            // Trigger companion reaction
            if abs(change) >= 10 {
                s.triggerStrongReaction(entity, change > 0)
            }
        }
    }
}

func (s *CompanionApprovalSystem) getApprovalChange(companionID, choiceID, optionID string) int {
    // Example: Choice about helping machines vs. humans
    if choiceID == "help_machines_or_humans" {
        switch companionID {
        case "lyra": // Pro-machine
            if optionID == "help_machines" {
                return 15 // Big approval gain
            } else if optionID == "help_humans" {
                return -10 // Moderate disapproval
            }
        case "marcus": // Pro-human
            if optionID == "help_machines" {
                return -15 // Big disapproval
            } else if optionID == "help_humans" {
                return 10 // Moderate approval
            }
        case "kai": // Pragmatic
            if optionID == "help_both" {
                return 5 // Slight approval for middle ground
            }
        }
    }
    return 0
}
```

---

## 🎮 Core Gameplay Pillars

### 1. **Traversal & Exploration** (Uncharted DNA)

#### Climbing System:
- **Ledge Detection**: Automatic highlighting of climbable surfaces
- **Free Climbing**: Move in any direction on climbable walls
- **Stamina System**: Limited climbing time, rest on ledges
- **Dynamic Handholds**: Some ledges crumble, creating tension
- **Leap of Faith**: Jump to distant ledges (risk/reward)

#### Advanced Traversal:
- **Rope Swinging**: Physics-based momentum
- **Zip-lines**: Fast travel between points
- **Wall Running**: Short bursts on specific surfaces
- **Sliding**: Down slopes, under obstacles
- **Vaulting**: Over low obstacles
- **Mantling**: Pull up onto ledges

#### Environmental Puzzles:
- **Weight Puzzles**: Use objects to trigger mechanisms
- **Light Reflection**: Redirect beams to unlock doors
- **Hacking Minigames**: Simple but satisfying (not annoying)
- **Timed Sequences**: Escape collapsing structures

### 2. **Combat** (Horizon + The Last of Us)

#### Weapon Systems:

**✅ IMPLEMENTED:**

1. **Ranged Weapons** (Fully Functional):
   - ✅ **Pistol**: Semi-auto, 30-round magazine, 90 reserve ammo, 0.2s fire rate
     - Compact design, black grip, short barrel (0.22m)
     - Orange muzzle tip for visibility
     - Muzzle flash: 80ms bright yellow-white burst
     - Recoil animation: 150ms shoulder kickback
   
   - ✅ **Rifle**: Semi-auto, 30-round magazine, 90 reserve ammo, 0.2s fire rate
     - Long barrel (0.48m), dark brown stock, dark gray metal
     - Larger muzzle flash (18cm vs 15cm)
     - Extended range and accuracy
   
   - ✅ **Bow**: Silent, precision weapon
     - Vertical design (0.7m tall), wood brown limbs
     - Arrow with silver arrowhead visible on string
     - NO muzzle flash (silent weapon)
     - Ideal for stealth gameplay

2. **Weapon Switching System** ✅:
   - **1 Key**: Switch to Pistol
   - **2 Key**: Switch to Rifle
   - **3 Key**: Switch to Bow
   - Dynamic weapon model swapping
   - Weapon-specific animations (recoil, muzzle flash)

3. **Reload System** ✅:
   - **R Key**: Manual reload
   - **Auto-reload**: Triggers when magazine empty
   - Reserve ammo management (3 magazines worth)
   - Reload time: 1.5 seconds
   - HUD displays: Current/Magazine (Reserve)
   - "RELOADING..." indicator during reload

4. **Projectile System** ✅:
   - Physics-based bullet trajectories
   - 50 m/s projectile speed
   - 10-second lifetime (500m range)
   - 2.0m collision radius for hit detection
   - Visual projectile rendering (bright yellow spheres)

**🔄 TO BE IMPLEMENTED:**

5. **Additional Ranged Weapons**:
   - **Pulse Rifle**: Automatic, medium range, energy ammo
   - **Shock Pistol**: Stuns machines, low ammo capacity
   - **Grenade Launcher**: Area damage, crafted ammo
   - **Sniper Rifle**: Long range, high damage, slow fire rate

6. **Melee Weapons**:
   - **Plasma Blade**: Fast, low damage, energy-based
   - **Shock Baton**: Stuns enemies, non-lethal option
   - **Heavy Hammer**: Slow, high damage, breaks armor

7. **Traps & Tools**:
   - **Proximity Mines**: Set and forget area denial
   - **Tripwires**: Create chokepoints
   - **Lures**: Distract enemies with sound
   - **EMP Grenades**: Disable machines temporarily

8. **Weapon Modifications**:
   - **Scopes**: Zoom for precision
   - **Silencers**: Reduce detection range
   - **Extended Magazines**: More ammo capacity
   - **Damage Upgrades**: Increase base damage
   - **Elemental Mods**: Fire/Ice/Shock damage

#### Enemy AI & Death System:

**✅ IMPLEMENTED:**

1. **Enemy State Machine** ✅:
   - **States**: "patrol", "chase", "attack", "dying", "dead"
   - **State Transitions**: Proper guards prevent invalid transitions
   - **Death State Protection**: Enemies in "dying" state cannot be damaged
   - **Animation Interruption Prevention**: Death animation plays to completion

2. **Death Animation System** ✅:
   - **Duration**: 2.0 seconds (configurable per enemy type)
   - **Fall Animation**: Progressive rotation (0° → 90° forward tip)
   - **Sinking**: Enemy sinks 0.5m into ground
   - **Fade Out**: Scale reduces to 50% (visual fade effect)
   - **Auto-Removal**: Entity removed after animation completes
   - **Smooth Progression**: Linear interpolation for all transforms

3. **Enemy Visuals** ✅:
   - **Red Cylinders**: 2m tall, 0.5m radius
   - **Positioned Correctly**: Y=1.0 (center of cylinder)
   - **Health System**: 50 HP default, 10 damage per hit
   - **Hit Detection**: 2.0m collision radius
   - **Visual Feedback**: Enemies darken/shrink as they die

**🔄 TO BE IMPLEMENTED:**

#### Enemy Types (12+ Varieties):

**Human Enemies**:
1. **Scouts**: Light armor, patrol routes, alert others
2. **Heavies**: Armored, slow, high damage, suppressive fire
3. **Snipers**: Long range, must flank, laser sights
4. **Commanders**: Buff nearby enemies, call reinforcements
5. **Medics**: Heal allies, priority targets
6. **Engineers**: Deploy turrets, repair machines

**Machine Enemies**:
1. **Watchers**: Small, fast, patrol drones, weak but numerous
2. **Stalkers**: Medium, aggressive hunters, stealth capabilities
3. **Titans**: Large, slow, heavy armor, devastating attacks
4. **Swarms**: Tiny, overwhelming numbers, kamikaze attacks
5. **Corrupted**: Unpredictable, dangerous, erratic behavior
6. **Harvesters**: Resource gatherers, non-aggressive unless provoked

**Wildlife** (neutral until provoked):
1. **Mutated Wolves**: Pack hunters, coordinated attacks
2. **Razorback Boars**: Charge attacks, high knockback
3. **Venomous Serpents**: Stealth predators, poison status effect

**Boss Enemies** (Unique Encounters):
1. **The Warden**: Corrupted security AI, multiple phases
2. **Apex Predator**: Massive mutated creature
3. **Rogue Titan**: Heavily armored machine boss

#### Combat Mechanics:
- **Weak Points**: Target specific components (Horizon-style)
- **Status Effects**: Fire, shock, freeze, poison
- **Stealth Kills**: Silent takedowns from behind/above
- **Execution Moves**: Finish downed enemies (brutal but optional)
- **Cover System**: Snap to cover, blind fire, peek & shoot
- **Dodge Roll**: I-frames, stamina cost
- **Parry/Counter**: Timing-based, high risk/reward

### 3. **Stealth** (The Last of Us DNA)

#### Stealth Mechanics:
- **Crouch/Prone**: Reduce visibility and noise
- **Tall Grass**: Hide from enemies
- **Sound Propagation**: Footsteps, gunshots alert enemies
- **Line of Sight**: Enemies have vision cones
- **Distraction**: Throw objects to create noise
- **Takedowns**: Silent kills from stealth
- **Body Hiding**: Drag corpses to hide evidence

#### Detection System:
- **Three States**: Unaware → Suspicious → Alert
- **Investigation**: Enemies check last known position
- **Reinforcements**: Alerted enemies call for help
- **Hunt Mode**: If spotted, enemies actively search

#### Stealth Tools:
- **Smoke Bombs**: Create cover
- **Silenced Weapons**: Limited ammo
- **Hacking**: Disable cameras, turrets
- **Environmental Kills**: Drop objects on enemies

### 4. **Crafting & Resource Management** (The Last of Us)

#### Resource Types:
1. **Common**: Scrap Metal, Cloth, Wire
2. **Uncommon**: Circuit Boards, Batteries
3. **Rare**: AI Cores, Plasma Cells
4. **Unique**: Story items, legendary materials

#### Crafting Categories:
1. **Consumables**: Health kits, ammo, throwables
2. **Weapon Mods**: Scopes, silencers, damage upgrades
3. **Armor**: Chest, helmet, boots (stat bonuses)
4. **Tools**: Lockpicks, hacking devices

#### Crafting Stations:
- **Field Crafting**: Basic items, limited options
- **Workbenches**: Full upgrades, save points
- **Special Stations**: Unique crafts (companion gifts, story items)

### 5. **Focus Mode** (Horizon DNA)

#### Scanner Abilities:
- **Enemy Tagging**: Mark enemies through walls
- **Weak Point Highlighting**: Show vulnerable spots
- **Path Finding**: Highlight climbable routes
- **Resource Detection**: Find crafting materials
- **Audio Cues**: Hear distant enemies/events

#### Upgrades:
- **Range Extension**: See further
- **Detail Level**: More information revealed
- **Duration**: Longer active time
- **Cooldown Reduction**: Use more frequently

### 6. **RPG Systems** (Mass Effect Influence)

#### Character Progression:
- **Level System**: 1-50, gain XP from combat, exploration, quests
- **Skill Trees** (3 Branches):
  1. **Combat**: Weapon damage, health, armor
  2. **Tech**: Hacking, crafting, focus abilities
  3. **Survival**: Stealth, stamina, resource efficiency

#### Choice & Consequence:
- **Dialogue System**: Multiple response options
- **Morality**: Pragmatic vs. Idealistic (not good/evil)
- **Major Choices**: Affect story, companion relationships, endings
- **Minor Choices**: Flavor, world-building, small rewards

#### Companion Relationships:
- **Loyalty Missions**: Personal quests unlock abilities
- **Approval System**: Companions react to choices
- **Romance Options**: Optional, meaningful
- **Companion Abilities**: Call for support in combat

---

## 📖 Story Structure

### Act 1: Discovery (5-7 hours)
**Setting**: The Verdant Sprawl

**Goals**:
- Introduce mechanics gradually
- Establish world, characters, conflict
- Tutorial disguised as story missions

**Key Missions**:
1. **"Awakening"**: Tutorial - basic movement, combat
2. **"The Relic"**: Discover ancient AI artifact
3. **"First Contact"**: Meet ECHO, establish companion dynamic
4. **"The Purists"**: Introduce human antagonists
5. **"Descent"**: First major dungeon, all mechanics converge

**Ending**: Kira discovers the Architect's plan, must leave the city

### Act 2: Journey (10-15 hours)
**Settings**: Crystalline Wastes, Submerged Archives, Canopy Network

**Goals**:
- Open world exploration
- Build relationships with companions
- Gather allies and resources
- Uncover conspiracy layers

**Key Missions**:
1. **"Desert Ghosts"**: Stealth-heavy mission in ruins
2. **"Depths of Knowledge"**: Underwater exploration
3. **"The Hunt"**: Track a legendary machine (boss fight)
4. **"Betrayal"**: Marcus's dark secret revealed (choice point)
5. **"The Network"**: Discover machine consciousness is sentient

**Major Choice**: Side with machines, humans, or forge third path

### Act 3: Convergence (5-8 hours)
**Setting**: The Spire

**Goals**:
- All systems at full power
- Consequences of choices manifest
- Multiple endings based on decisions

**Key Missions**:
1. **"The Assault"**: Lead attack on the Spire
2. **"Infiltration"**: Stealth mission to reach Architect
3. **"Revelations"**: Truth about the uprising
4. **"The Choice"**: Final decision that determines ending
5. **"Echoes"**: Epilogue showing consequences

**Endings** (4 Variations):
1. **Synthesis**: Merge human and machine consciousness
2. **Dominion**: Destroy the Architect, human supremacy
3. **Liberation**: Free machines, uncertain future
4. **Sacrifice**: Kira becomes new mediator (bittersweet)

---

## 🎨 Art Direction & Atmosphere

### Visual Style:
- **Realistic with Stylization**: Grounded but vibrant
- **Color Palette**: 
  - Verdant: Greens, blues, warm sunlight
  - Wastes: Oranges, purples, harsh shadows
  - Archives: Blues, teals, bioluminescence
  - Canopy: Greens, yellows, dappled light
  - Spire: Whites, silvers, cold blues

### Mood & Tone:
- **Hopeful Melancholy**: Beauty in decay
- **Quiet Tension**: Calm before storms
- **Earned Triumph**: Victories feel significant
- **Emotional Resonance**: Story beats hit hard

### Audio Design:
- **Dynamic Music**: Adapts to gameplay (stealth, combat, exploration)
- **Environmental Audio**: Wind, water, machine hums
- **Voice Acting**: Fully voiced main characters
- **Sound Effects**: Punchy, satisfying feedback

---

## 🎯 Implementation Roadmap

### Phase 1: Core Systems (Weeks 1-2)
**Priority**: Get all mechanics working

1. ✅ **Movement & Camera** (DONE)
2. ✅ **Basic Combat** (DONE)
3. 🔄 **Climbing System**
4. 🔄 **Stealth Mechanics**
5. 🔄 **Cover System**
6. 🔄 **Crafting System**
7. 🔄 **Focus Mode**
8. 🔄 **Advanced Traversal**
9. 🔄 **Companion AI Foundation**
   - Basic follow behavior
   - Combat participation
   - Resource tossing system

### Phase 2: Content Creation (Weeks 3-4)
**Priority**: Build the world

1. **Level Design**:
   - Greybox all 5 biomes
   - Place enemies, resources, collectibles
   - Test flow and pacing

2. **Enemy AI**:
   - Implement all 12+ enemy types
   - Behavior trees for each type
   - Test combat encounters

3. **Environmental Assets**:
   - Climbable surfaces marked clearly
   - Cover objects placed strategically
   - Interactive objects (doors, switches, etc.)

### Phase 3: Story Integration (Week 5)
**Priority**: Make it meaningful

1. **Dialogue System**:
   - Implement conversation trees
   - Add choice tracking
   - Connect to consequence system
   - Companion approval system

2. **Quest System**:
   - Main story missions
   - Side quests
   - Companion loyalty missions
   - Personal quests

3. **Companion Relationships**:
   - Approval tracking
   - Dynamic dialogue based on relationship
   - Romance system (optional)
   - Companion endings

4. **Cutscenes**:
   - Key story moments
   - In-engine cinematics
   - Emotional beats
   - Companion interactions

### Phase 4: Polish & Balance (Week 6)
**Priority**: Make it feel AAA

1. **Combat Tuning**:
   - Enemy health/damage balance
   - Weapon feel and feedback
   - Difficulty curves

2. **Visual Polish**:
   - Particle effects
   - Post-processing
   - Animation polish

3. **Audio Implementation**:
   - Music integration
   - Sound effects
   - Voice lines

4. **Performance**:
   - Maintain 60+ FPS
   - Optimize heavy scenes
   - Memory management

### Phase 5: Testing & Iteration (Week 7-8)
**Priority**: Ensure quality

1. **Automated Testing**:
   - Playtest scenarios for each mechanic
   - Screenshot verification
   - Performance benchmarks

2. **Manual Testing**:
   - Full playthroughs
   - Bug fixing
   - Balance adjustments

3. **Documentation**:
   - Player manual
   - Developer documentation
   - Marketing materials

---

## 🚀 CRITICAL AAA FEATURES TO ADD (Competitive Parity)

To compete with best-in-class AAA games, we need these additional features:

### 1. **Hit Reactions & Feedback** (High Priority)
- **Enemy Hit Reactions**: Enemies flinch/stagger when shot
- **Headshot Feedback**: Special animation + bonus damage for headshots
- **Critical Hits**: Visual/audio feedback for critical damage
- **Damage Numbers**: Optional floating damage numbers
- **Hit Markers**: Visual confirmation of hits (crosshair feedback)
- **Blood/Spark Effects**: Impact particles based on enemy type

### 2. **Advanced Animation System** (High Priority)
- **Ragdoll Physics**: Enemies fall realistically when killed
- **Blend Trees**: Smooth transitions between animations
- **Inverse Kinematics (IK)**: Feet plant correctly on slopes
- **Procedural Animation**: Dynamic reactions to terrain
- **Facial Animations**: Companion expressions during dialogue
- **Lip Sync**: Mouth movements match dialogue

### 3. **Cover-to-Cover Movement** (Medium Priority)
- **Snap to Cover**: Automatic cover detection and snap
- **Peek & Shoot**: Lean out from cover to fire
- **Blind Fire**: Shoot without exposing yourself
- **Cover Vaulting**: Jump over low cover
- **Cover Transitions**: Move between cover points smoothly
- **Destructible Cover**: Cover degrades under fire

### 4. **Advanced AI Behaviors** (High Priority)
- **Flanking**: Enemies coordinate to surround player
- **Suppression**: Enemies pin player down with covering fire
- **Retreat**: Enemies fall back when outmatched
- **Call for Backup**: Enemies radio for reinforcements
- **Use Cover**: Enemies actively seek and use cover
- **Throw Grenades**: Enemies flush player out of cover

### 5. **Environmental Interactions** (Medium Priority)
- **Destructible Objects**: Crates, barrels explode
- **Interactive Elements**: Doors, switches, levers
- **Environmental Kills**: Drop objects on enemies
- **Explosive Barrels**: Red barrels for area damage
- **Climbable Vines**: Natural climbing surfaces
- **Breakable Glass**: Windows shatter realistically

### 6. **Photo Mode** (Low Priority, High Impact)
- **Free Camera**: Move camera anywhere
- **Filters**: Apply visual effects
- **Pose Characters**: Freeze and pose companions
- **Time of Day**: Change lighting
- **Share**: Export high-res screenshots
- **Frame Mode**: Add borders and text

### 7. **Accessibility Features** (High Priority)
- **Colorblind Modes**: Multiple color palettes
- **Subtitles**: Fully subtitled dialogue
- **Control Remapping**: Custom key bindings
- **Difficulty Options**: Combat, stealth, puzzle difficulty
- **Auto-Aim**: Optional aim assist
- **HUD Scaling**: Adjustable UI size

### 8. **Performance Optimizations** (Critical)
- **LOD System**: Level of detail for distant objects
- **Occlusion Culling**: Don't render hidden objects
- **Object Pooling**: Reuse projectiles, particles
- **Spatial Partitioning**: Efficient collision detection
- **Async Loading**: Stream assets without hitches
- **Target**: Stable 60 FPS on mid-range hardware

### 9. **Polish & Juice** (Medium Priority)
- **Screen Shake**: Camera shake on explosions
- **Slow Motion**: Bullet time for dramatic moments
- **Kill Cams**: Cinematic final kill replays
- **Contextual Animations**: Unique kills in specific situations
- **Weather Effects**: Rain, snow, fog
- **Day/Night Cycle**: Dynamic time of day

### 10. **Progression Feedback** (High Priority)
- **Level Up Fanfare**: Satisfying level-up animation
- **Skill Unlock Notifications**: Clear visual feedback
- **Achievement System**: Track accomplishments
- **Statistics Tracking**: Kills, deaths, playtime
- **Completion Percentage**: Show progress
- **Unlockables**: Cosmetics, concept art, behind-the-scenes

### 11. **Sound Design Excellence** (High Priority)
- **3D Positional Audio**: Hear enemies' locations
- **Dynamic Music**: Adapts to gameplay intensity
- **Weapon Sound Variety**: Each weapon sounds unique
- **Environmental Ambience**: Biome-specific sounds
- **Footstep Variety**: Different surfaces sound different
- **Voice Lines**: Companion combat chatter

### 12. **Tutorial & Onboarding** (Critical)
- **Contextual Tutorials**: Teach mechanics when needed
- **Practice Area**: Safe space to learn controls
- **Hint System**: Optional hints for stuck players
- **Tooltips**: Hover for detailed explanations
- **Difficulty Curve**: Gradual introduction of mechanics
- **Skip Option**: Let experienced players skip tutorials

---

## 🎮 Unique Selling Points

### What Makes This Special:

1. **Seamless Blend of Genres**:
   - Not just action OR stealth OR RPG - it's all three, equally polished

2. **Meaningful Choices**:
   - Decisions affect gameplay, not just dialogue
   - Multiple playthroughs reveal new content

3. **Emotional Storytelling**:
   - Companions you care about
   - Stakes that feel real
   - Endings that resonate

4. **Technical Excellence**:
   - Built with gdext-go (showcase the framework)
   - Smooth performance
   - Beautiful visuals

5. **Respect for Player Time**:
   - No grinding or padding
   - Every mission advances story or character
   - Replayability through choice, not repetition

---

## 📊 Success Metrics

### Technical Goals:
- ✅ 60+ FPS on target hardware
- ✅ All systems working smoothly
- ✅ Zero game-breaking bugs
- ✅ Automated test coverage for all features

### Creative Goals:
- 📖 Compelling 20-30 hour story
- 🎭 Memorable characters and moments
- 🌍 Rich, explorable world
- 🎮 Satisfying gameplay loop

### Framework Goals:
- 🚀 Prove gdext-go can build AAA games
- 📚 Document best practices
- 🛠️ Create reusable systems
- 🌟 Inspire other developers

---

## 🎬 Next Steps

### Immediate Actions:
1. ✅ Complete autonomous testing infrastructure
2. 🔄 Implement climbing system with test scenarios
3. 🔄 Build first level (Verdant Sprawl - Hub area)
4. 🔄 Create 3 enemy types (Scout, Watcher, Heavy)
5. 🔄 Implement stealth mechanics
6. 🔄 Add crafting system basics
7. 🔄 Create first story mission

### This Week's Goal:
**"Make the first 30 minutes of gameplay absolutely incredible"**

- Tutorial that teaches without feeling like a tutorial
- First combat encounter that's tense and satisfying
- First climbing sequence that's thrilling
- First story beat that hooks the player
- First companion interaction that establishes relationship

---

## 💡 Design Philosophy

**"Every mechanic serves the story. Every story beat showcases a mechanic. Everything works together to create an unforgettable experience."**

We're not building features in isolation - we're crafting a cohesive whole where:
- Climbing isn't just movement - it's about reaching for hope
- Stealth isn't just avoiding enemies - it's about survival against odds
- Combat isn't just shooting - it's about protecting what matters
- Crafting isn't just resource management - it's about making do with what you have
- Choices aren't just dialogue options - they're about defining who Kira becomes

---

## 🌟 Inspiration Reference

### The Last of Us:
- ✅ Emotional storytelling
- ✅ Resource scarcity creates tension
- ✅ Stealth feels necessary, not optional
- ✅ Companion AI that feels real

### Uncharted:
- ✅ Traversal as core gameplay
- ✅ Cinematic set pieces
- ✅ Witty dialogue and character chemistry
- ✅ Environmental puzzles

### Horizon Zero Dawn:
- ✅ Tactical combat with weak points
- ✅ Focus mode for information gathering
- ✅ Beautiful post-apocalyptic world
- ✅ Strong female protagonist

### Mass Effect:
- ✅ Choice-driven narrative
- ✅ Companion loyalty system
- ✅ RPG progression
- ✅ Multiple endings based on decisions

---

**Let's build something incredible.** 🚀

*"In the ruins of the old world, we find the seeds of the new."*

