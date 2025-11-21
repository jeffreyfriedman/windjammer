# Reactive Re-rendering Implementation Plan

## 🎯 Goal
Make the UI automatically update when `Signal<T>` values change.

## 🏗️ Architecture Decision

We'll use a **simplified effect-based system** inspired by Solid.js, but simpler for our first iteration.

### Key Components

1. **Global App State** - Store the root VNode and DOM element
2. **Effect System** - Track which effects depend on which signals
3. **Signal Notifications** - Trigger effects when signals change
4. **DOM Updates** - Re-render and patch the DOM

## 📝 Implementation Steps

### Step 1: Global App State (30 min)
Store the app instance globally so signals can trigger re-renders.

### Step 2: Effect Registration (45 min)
Track which UI elements depend on which signals.

### Step 3: Signal Notifications (30 min)
Make `Signal::set()` notify registered effects.

### Step 4: DOM Re-rendering (45 min)
Implement efficient re-rendering of the UI.

### Step 5: Testing (30 min)
Test with interactive counter and button examples.

## Total Estimated Time: 3 hours

Let's begin!

