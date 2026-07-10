# 🧿 Cluaiz Neural Hub: Cluaiz-DNA CLI (Architecture & Roadmap)

This document serves as the **Single Source of Truth** and the **Architectural Brain** for the Cluaiz CLI (The "Neural OS" Dashboard). It defines the deep logic, user experience sequences, and hardware-level governance that power the Cluaiz ecosystem.

---

## 🏗️ 1. Core Philosophy: The "First-Boot" Handshake
Every top-tier neural engine (like Claw or OpenClaw) must establish a "Neural Handshake" with the user. Cluaiz's onboarding is not a static setup screen; it is a sequential, animated journey that defines the **Structural DNA** of the local node.

### **Phase A: Neural Ignition (The Intro)**
- **Logic**: A 60-frame ASCII animation sequence.
- **Implementation**: We use a `frame_counter` inside the Ratatui render loop.
    - **Buffer Glow**: The `CLUAIZ` logo starts in `Color::DarkGray` and pulses into `Color::Cyan` over 1 second (30-60 frames).
    - **Purpose**: This creates a "boot-up" feel, establishing Cluaiz as an 'active' system rather than a passive script.

### **Phase B: Cluaiz Identity (User Extraction)**
Cluaiz must know who it is serving to optimize the neural weights.
1.  **Identity Entry**: Requesting the User's Name.
2.  **Utility Logic**: Determining the Purpose:
    *   `RESEARCH`: Optimizes for high precision, lower token speed.
    *   `PRODUCTION`: Optimizes for maximum throughput and concurrent threads.
    *   `CREATIVE`: Optimizes for long-context retention.
3.  **Logic Guardrail**: These fields are persisted into `system_control.json` under the `user_identity` object.

### **Phase C: Hardware DNA Audit**
## 🧬 2. Cluaiz-DNA Patterns (Research-Driven)
Cluaiz adopts high-tier architectural patterns from elite agents (OpenClaw) to ensure a stable and premium "Day 0" experience:

- **The Neural Ritual (Bootstrapping) 🕯️**: Cluaiz no longer just boots; it undergoes a ritual. On the first run, the engine seeds the workspace (~/.cluaiz/workspace) with fundamental identity files: `IDENTITY.md`, `USER.md`, and `SOUL.md`. This "Neural Cradle" serves as the persistent memory of the Cluaiz node.
- **The Sentinel Mechanism 🛡️**: To handle "Auto-Landing" reliably, Cluaiz creates a temporary `.ignition_lock` file during onboarding. If the file exists, the ritual resumes at the last point. Once complete, the sentinel is purged, and the system permanently bypasses setup on subsequent boots.
- **The Atomic Handshake 💬**: Instead of a monolithic setup form, Cluaiz uses a "One-Question-At-A-Time" interactive interview. This reduces cognitive load and ensures that the User's Identity (Name, Purpose, Logic) is "Burned" into the DNA properly.
- **Security Trust Model 🧱**: Cluaiz explicitly presents a "Privacy Handshake" upfront, detailing the local vs. cloud trust boundaries. This establishes the user as the sole Cluaiz operator of the hardware.

---

## 🧠 3. TUI Architecture (State & Scalability)

### **State Machine Design**
Cluaiz's state is divided into two primary dimensions:
1.  **OsState (Lifecycle)**:
    - `Setup(OnboardingStep)`: Sequential steps (Intro -> Name -> Purpose -> Audit).
    - `Dashboard`: The main "Neural Hub" once DNA is saved.
2.  **MenuApp (Context)**:
    - `Roster`: Model registry and local weights.
    - `Settings`: Hardware DNA management.
    - `Chat`: Real-time inference interaction.

### **Scalability (The 10M User Goal)**
To ensure Cluaiz scales to millions of users without central failure:
- **Local Persistence**: Zero cloud reliance for identity.
- **Thread Governance**: All inference happens in a separate `tokio` thread, communicating with the UI via `mpsc` channels. This ensures the TUI never flickers or freezes during heavy tensor computation.

---

## 🎨 3. Research-Led Design (Patterns from OpenClaw)
To achieve a "wow" first impression, Cluaiz adopts proven architectural patterns from top-tier agents:

- **Visual Precision (Rigid Padding) 📏**: Following OpenClaw's logic (e.g., `CRON_ID_PAD`), we implement fixed-width constants for the Neural Roster. This ensures that model names look pixel-perfect with ellipses (...) and no layout-shift occurs during long-string rendering.
- **Temporal Handshake (Relative Context) 🕰️**: Instead of raw epoch numbers or seconds, Cluaiz displays "Relative Time" (e.g., "in 5m", "30s ago"). This makes the Download progress and Generation ETAs intuitive and human-centric.
- **Semantic Feedback 🌈**: Automated color matching (Green for `Optimal`, Yellow for `Heavy`, Red for `Failing`). The TUI dynamically adjusts its glow based on the underlying health of the Neural Bridge.
- **Thin Client Architecture 🐚**: The CLI remains a lightweight shell. High-level logic (weights, hardware-locks) is handled by the **Cluaiz Neural Bridge**, with the CLI simply patching JSON values like an RPC client.

## 🌈 4. The "Wow" Onboarding Sequence (Cluaiz Lifecycle)
Cluaiz's boot sequence is divided into five distinct sub-states:
1.  **Phase 0: Greeting**: Animated ASCII pulse sequence.
2.  **Phase 1: Identity Extraction**: Capturing User Name.
3.  **Phase 2: Purpose Mapping**: Logic toggle (Research vs. Production).
4.  **Phase 3: Hardware Audit**: Live "Bare Metal" spec readout.
5.  **Phase 4: Registry Sync**: Initial model recommendations based on identity + hardware.

---

## 🛣️ 5. Immediate Development Roadmap

### **Step 1: The Multi-Step Setup Refactor**
1.  Modify `AppState` to support `OnboardingStep`.
2.  Refactor `setup/mod.rs` to render different screens for Identity vs. Hardware.
3.  Implement the **"Auto-Landing" Sentinel** in `main.rs` to bypass setup if DNA exists.

### **Step 2: Identity Persistence & UI Polish**
1.  Update `system_control_manager.rs` to support `save_identity(name, purpose)`.
2.  Implement **Sovereign Pad** (truncation logic) in the Roster view.
3.  Display "Welcome, [Name]" on the main dashboard.

---
**Cluaiz Neural Hub: Built for the Cluaiz-DNA Node. 🧿⚡🚀**
