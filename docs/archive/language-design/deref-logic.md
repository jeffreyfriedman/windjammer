# String Comparison Deref Logic - Proper Design

## The Problem

When comparing strings in Windjammer, we need to handle multiple cases:

### Case 1: Both Borrowed (&String == &String)
```windjammer
fn matches(self, entity_components: &Vec<String>) -> bool {
    for req in self.required.iter() {  // req: &String
        for comp in entity_components.iter() {  // comp: &String
            if comp == req {  // &String == &String ✅
```

**Correct Rust:** `comp == req` (PartialEq<&String> works!)  
**Old Compiler:** `comp == *req` ❌ (causes E0277: &String == String)  
**New Compiler:** `comp == req` ✅ (FIXED!)

### Case 2: Owned vs Borrowed (String == &String)
```windjammer
fn find_npc(self, npc_id: &String) -> bool {
    for i in 0..self.members.len() {
        if self.members[i].npc_id == npc_id {  // String == &String
```

**Correct Rust:** `&self.members[i].npc_id == npc_id` OR `self.members[i].npc_id.as_str() == npc_id.as_str()`  
**Old Compiler:** `self.members[i].npc_id == *npc_id` ✅ (works via deref!)  
**New Compiler:** `self.members[i].npc_id == npc_id` ❌ (causes E0277: String == &String)

### Case 3: Both Owned (String == String)
```windjammer
fn compare_names(a: String, b: String) -> bool {
    a == b  // String == String ✅
```

**Correct Rust:** `a == b` (PartialEq<String> works!)  
**All Compilers:** `a == b` ✅

## Root Cause Analysis

The original auto-deref logic was:
```rust
if is_comparison {
    if let Expression::Identifier { name, .. } = right {
        if self.inferred_borrowed_params.contains(name.as_str()) 
           || self.borrowed_iterator_vars.contains(name) {
            right_str = format!("*{}", right_str);  // Add * to borrowed params
        }
    }
}
```

**Problems:**
1. ✅ **Correctly** added `*` when left=owned, right=borrowed (Case 2)
2. ❌ **Incorrectly** added `*` when left=borrowed, right=borrowed (Case 1)
3. ❌ Only checked RIGHT operand, never LEFT operand

## Proper Solution

### Algorithm:
1. Infer if LEFT is borrowed (from params/iterators)
2. Infer if RIGHT is borrowed (from params/iterators)
3. Apply rules:
   - **Both borrowed:** NO change (PartialEq<&T> works)
   - **Both owned:** NO change (PartialEq<T> works)
   - **Left owned, right borrowed:** Add `*` to right
   - **Left borrowed, right owned:** Add `*` to left

### Implementation:
```rust
if is_comparison {
    let left_is_borrowed = match left {
        Expression::Identifier { name, .. } => {
            self.inferred_borrowed_params.contains(name.as_str())
            || self.borrowed_iterator_vars.contains(name)
        }
        _ => false,
    };
    
    let right_is_borrowed = match right {
        Expression::Identifier { name, .. } => {
            self.inferred_borrowed_params.contains(name.as_str())
            || self.borrowed_iterator_vars.contains(name)
        }
        _ => false,
    };
    
    // Only add deref if exactly ONE side is borrowed (XOR)
    if left_is_borrowed && !right_is_borrowed {
        // &String == String → *&String == String (deref left)
        left_str = format!("*{}", left_str);
    } else if !left_is_borrowed && right_is_borrowed {
        // String == &String → String == *&String (deref right)
        right_str = format!("*{}", right_str);
    }
    // If both borrowed or both owned: NO deref needed
}
```

## Test Cases Required

### TDD Test 1: Both Borrowed (Already passing ✅)
```windjammer
fn check(name: &String, target: &String) -> bool {
    name == target  // &String == &String
}
```

**Expected:** `name == target` (no *)

### TDD Test 2: Owned vs Borrowed (Need to add)
```windjammer
struct Member {
    id: String,
}

fn find(members: &Vec<Member>, target_id: &String) -> bool {
    for m in members.iter() {
        if m.id == target_id {  // String == &String
            return true
        }
    }
    false
}
```

**Expected:** `m.id == *target_id` (add * to right)

### TDD Test 3: Iterator Both Borrowed (Already passing ✅)
```windjammer
fn find_match(items: &Vec<String>, target: &String) -> bool {
    for item in items.iter() {  // item: &String
        if item == target {  // &String == &String
            return true
        }
    }
    false
}
```

**Expected:** `item == target` (no *)

## Complexity: Field Access

The real challenge is field access:
- `m.id` where `m: &Member` → generates `m.id` but represents `&String` or `String`?
- Need to know if field access on borrowed produces owned or borrowed

**Current behavior:** Field access on struct produces owned value (needs `.clone()`)  
**For comparisons:** We need to detect this and add `*` to the other side

---

*Next: Implement smart XOR logic with proper type inference*
