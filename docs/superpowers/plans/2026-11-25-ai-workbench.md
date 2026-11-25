# AI 工作台 (AI Workbench) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the AI Workbench tab in the open island view (placed directly next to file-shelf) with real-time multi-agent activity monitoring, custom brand icons, status pills, and focus actions matching the design specification.

**Architecture:** A native tab button in `crates/nook/src/island.rs` navigates to a new shipped mod `crates/nook/mods/ai-workbench`. The mod reads active agent processes via `agents()`, `codexStatus()`, and `mapStatus()`, rendering dynamic cards with fallback preview states.

**Tech Stack:** Rust (Rune UI, openNook island layout), QuickJS / JS (`nook/ui`, `nook`), SVG icon assets.

---

### Task 1: Add Brand Icon Assets
**Files:**
- Create: `crates/nook/assets/icons/sparkle.svg`
- Create: `crates/nook/assets/icons/gem.svg`
- Test: `cargo test -p nook icons`

- [ ] **Step 1: Write sparkle.svg and gem.svg**
- [ ] **Step 2: Run cargo test -p nook icons to verify icon conversion**
- [ ] **Step 3: Commit icon assets**

---

### Task 2: Create the AI Workbench Mod
**Files:**
- Create: `crates/nook/mods/ai-workbench/mod.json`
- Create: `crates/nook/mods/ai-workbench/main.js`
- Test: `/Applications/openNook.app/Contents/MacOS/openNook check crates/nook/mods/ai-workbench/`

- [ ] **Step 1: Create mod.json**
- [ ] **Step 2: Implement main.js with 1:1 active AI task list card layout, status pills, and jump action**
- [ ] **Step 3: Run opennook check crates/nook/mods/ai-workbench/ to verify 0 syntax/runtime errors**
- [ ] **Step 4: Commit mod files**

---

### Task 3: Wire Native Island Tab in island.rs
**Files:**
- Modify: `crates/nook/src/island.rs`
- Modify: `crates/nook/home/layout.json`

- [ ] **Step 1: Add ai-workbench tab button right next to file-shelf tab**
- [ ] **Step 2: Update layout.json expanded list**
- [ ] **Step 3: Verify with cargo test -p nook**
- [ ] **Step 4: Commit island.rs and layout.json changes**

---

### Task 4: Compile, Deploy & Verify
**Files:**
- Run: `./scripts/bundle.sh release`
- Deploy: Replace `/Applications/openNook.app` and launch

- [ ] **Step 1: Compile release app with bundle.sh**
- [ ] **Step 2: Restart openNook app**
- [ ] **Step 3: Verify running process and logs**
