# CLUAIZ ARCHER vNext: Sovereign Skill System Design (Spec-First + Native Runtime)

Date: 2026-05-05
Owner: Cluaiz Archer Design
Status: Proposed System Design (Implementation Ready)

## 1. Problem Statement

Current skill authoring requires multiple artifacts (`manifest.json`, `connector.mcp`, `logic.wasm`, `state.kv-cache`) to be manually created and synchronized. This causes:

1. Authoring friction and slow iteration.
2. Drift between intent and runtime artifacts.
3. Inconsistent security declarations.
4. Harder onboarding for contributors.

At the same time, Cluaiz must preserve its core advantage:

1. Native Rust execution.
2. WASM sandboxed deterministic logic.
3. MCP external connectivity.
4. KV-cache neural injection.

## 2. Design Goal

Adopt **Spec-First authoring** without losing native power.

Single source for humans: `SKILL.md`

System-generated artifacts for runtime:

1. `manifest.json` (passport)
2. `connector.mcp` (hands)
3. `logic.wasm` (body)
4. `state.kv-cache` (soul)

## 3. Core Principle

`SKILL.md` is not a prompt-only template.

It is a strict compile contract consumed by Rust builders.

## 4. New Skill Package Contract

```text
skills/
  spec-first/
    packs/
      <skill-id>/
        SKILL.md               # only human-authored canonical source
        src/                   # optional rust source for wasm
        assets/                # optional prompts, fixtures, eval data
        artifacts/             # generated runtime outputs
          manifest.json
          connector.mcp
          logic.wasm
          state.kv-cache
          build-output.json
```

## 5. Runtime Modes (Strict)

`runtime_mode` controls required artifacts and validator behavior.

1. `prompt_only`:
   - Required: `manifest.json`
   - Optional: others
   - Use only for lightweight behavior packs.

2. `mcp_native`:
   - Required: `manifest.json`, `connector.mcp`
   - Optional: wasm/kv

3. `wasm_native`:
   - Required: `manifest.json`, `logic.wasm`
   - Optional: mcp/kv

4. `neural_native`:
   - Required: `manifest.json`, `state.kv-cache`
   - Optional: wasm/mcp

5. `sovereign_hybrid`:
   - Required: all 4 artifacts
   - Recommended for high-value production skills.

## 6. SKILL.md Schema

Frontmatter required keys:

1. `skill_id`
2. `version`
3. `runtime_mode`
4. `category`
5. `capabilities`
6. `triggers`
7. `permissions`
8. `soul_type`
9. `security_level`

Required sections:

1. `Intent Contract`
2. `Router Contract`
3. `Permission Contract`
4. `MCP Contract`
5. `WASM Contract`
6. `Neural Contract`
7. `Build Directives`
8. `Acceptance Tests`
9. `Security Guards`

## 7. Build Pipeline

Command targets:

1. `cluaiz skill validate <SKILL.md>`
2. `cluaiz skill build <SKILL.md>`
3. `cluaiz skill install <SKILL.md>`

Build stages:

1. Parse `SKILL.md` frontmatter + contracts.
2. Validate schema and mode constraints.
3. Generate `manifest.json`.
4. Generate `connector.mcp` if enabled.
5. Compile Rust -> `logic.wasm` if enabled.
6. Generate/attach `state.kv-cache` if enabled.
7. Compute checksums + optional signatures.
8. Emit `build-output.json`.
9. Fail closed on any required stage failure.

## 8. Runtime Execution Flow (Neural Foundry)

1. Registry indexes generated `manifest.json`.
2. SkillRouter resolves intent via semantic triggers + thresholds.
3. PermissionGuard verifies declared scopes.
4. NeuralFoundry runtime activates per mode:
   - KV mmap stitching
   - WASM module load
   - MCP connector handshake
5. LLM inference runs with injected context + deterministic outputs.
6. Audit subsystem logs declared actions and tool calls.

## 9. Security Model

### 9.1 Fail-Closed Validation

If `runtime_mode` says required artifact must exist and generation fails, install must abort.

### 9.2 Permission Binding

Permissions in `SKILL.md` must be copied into generated `manifest.json`. Runtime cannot exceed declared scope.

### 9.3 Artifact Integrity

Generated artifacts should include checksums in `build-output.json`.

### 9.4 Runtime Isolation

1. WASM runs sandboxed.
2. MCP tools limited by contract.
3. KV payload only loaded when schema/model family compatible.

## 10. Engine Module Design (Rust)

Suggested module layout under engines crate:

```text
src/
  neural_foundry/
    spec_first/
      parser.rs
      schema.rs
      validator.rs
      generator_manifest.rs
      generator_mcp.rs
      generator_wasm.rs
      generator_kv.rs
      installer.rs
      build_report.rs
```

Responsibilities:

1. `parser.rs`: parse markdown + frontmatter.
2. `schema.rs`: typed structs + serde.
3. `validator.rs`: mode constraints + permission checks.
4. `generator_manifest.rs`: produce runtime manifest.
5. `generator_mcp.rs`: produce connector config.
6. `generator_wasm.rs`: invoke wasm build workflow.
7. `generator_kv.rs`: bridge to kv profile generation.
8. `installer.rs`: registry register + hot-swap activation.
9. `build_report.rs`: checksums, diagnostics, output status.

## 11. CI/CD Plan

Add workflow stages:

1. `spec-first-validate`:
   - validate all `SKILL.md` in `skills/spec-first/packs/**`

2. `spec-first-build-smoke`:
   - build 1-2 reference skills (`agentic-eval`, `docker-pilot`)

3. `spec-first-policy-check`:
   - ensure no skill declares unsupported permission values

## 12. Migration Plan

Phase 1 (Pilot):

1. Migrate `agentic-eval`
2. Migrate `docker-pilot`
3. Keep old artifacts side-by-side

Phase 2 (Dual-mode):

1. Read existing legacy folders
2. Prefer spec-first outputs when present

Phase 3 (Default):

1. New skills must use `SKILL.md`
2. Legacy manual artifact creation deprecated

## 13. Why This Is Better

1. Fast contributor onboarding.
2. Single canonical source of truth.
3. Stronger auditability and security consistency.
4. Preserves Archer native execution strengths.
5. Scales to large skill catalogs with predictable quality.

## 14. Non-Goals

1. Replacing MCP with prompts.
2. Replacing Rust/WASM with script-only execution.
3. Allowing partial installs for production runtime modes.

## 15. Acceptance Criteria

Design is considered complete when:

1. `cluaiz skill build <SKILL.md>` generates artifacts for 2 pilot skills.
2. `cluaiz skill install <SKILL.md>` hot-swaps at runtime without restart.
3. CI rejects invalid frontmatter, missing required sections, or permission drift.
4. Runtime mode enforcement proves fail-closed behavior.

## 16. Example Policy Matrix

| Runtime Mode | Manifest | MCP | WASM | KV Cache | Install on Missing Required Part |
| --- | --- | --- | --- | --- | --- |
| prompt_only | Required | Optional | Optional | Optional | Fail |
| mcp_native | Required | Required | Optional | Optional | Fail |
| wasm_native | Required | Optional | Required | Optional | Fail |
| neural_native | Required | Optional | Optional | Required | Fail |
| sovereign_hybrid | Required | Required | Required | Required | Fail |

## 17. Final Recommendation

Adopt spec-first as authoring standard immediately, but execute through native Rust build and runtime contracts. This gives Cluaiz both:

1. Developer speed.
2. Sovereign-grade performance and security.



 

---

## 🏛️ ARCHER vNext: THE FOUNDER'S FAQ

### 🧠 1. Archer Engine असल में क्या है?
आर्चर कोई साधारण प्लगइन सिस्टम नहीं है; यह **Cluaiz** का "Neural Kernel" है[cite: 7, 20]. इसका काम एआई की **सोचने की शक्ति (Reasoning)** और **काम करने की क्षमता (Execution)** को अलग-अलग मैनेज करना है[cite: 7, 11]. यह सीधे सिलिकॉन (GPU/NPU) से जुड़कर एआई को हार्डवेयर की रफ़्तार देता है[cite: 11, 20].

### 📄 2. हम सिर्फ `SKILL.md` का इस्तेमाल क्यों कर रहे हैं?
*   **Source of Truth**: डेवलपर को 5 फाइलें नहीं लिखनी, सिर्फ एक साफ़-सुथरी Markdown फाइल लिखनी है[cite: 14, 16].
*   **Automation**: इस एक फाइल से हमारा **Rust-Engine** अपने आप `manifest.json`, `logic.wasm`, और `soul.kv-cache` जैसे टेक्निकल आर्टिफ़ैक्ट्स पैदा (Generate) कर देता है[cite: 14, 16].
*   **AI Friendly**: एआई के लिए Markdown लिखना और समझना JSON से कहीं ज्यादा आसान और सटीक है[cite: 16, 17].

### 🏎️ 3. क्या इस तरीके से Rust की "Power" कम हो जाएगी?
**बिल्कुल नहीं।** रफ़्तार पहले जैसी ही बिजली कड़क होगी[cite: 15, 20]. 
*   `SKILL.md` सिर्फ "नक्शा" है, लेकिन जो "इमारत" बनेगी वो नैटिव **Rust** से ही कंपाइल होगी[cite: 14, 15]. 
*   लॉजिक अभी भी `.wasm` में चलेगा और डेटा `mmap` के जरिए $O(1)$ रफ़्तार से मूव होगा[cite: 20].

### 🪄 4. "No Prompt / Zero Token" जादू कैसे काम करता है?
पारंपरिक एआई हर बार स्किल के निर्देश (Instructions) पढ़ता है, जिससे टोकन बर्बाद होते हैं[cite: 20]. 
*   आर्चर **KV-Cache Stitching** का उपयोग करता है[cite: 20]. 
*   हम स्किल की "याददाश्त" (`soul.kv-cache`) को सीधे GPU की VRAM में इंजेक्ट कर देते हैं[cite: 11, 20]. 
*   एआई को निर्देश पढ़ने नहीं पड़ते, उसे वो "नैटिवली" याद होते हैं[cite: 20].

### 📂 5. क्या हर स्किल में सारी फाइल्स (WASM, MCP, KV) होंगी?
**नहीं, यह "On-Demand" है**[cite: 16]. 
*   अगर स्किल को सिर्फ बाहरी API चाहिए, तो सिर्फ **MCP** होगा[cite: 14, 16]. 
*   अगर सिर्फ नैटिव लॉजिक चाहिए, तो सिर्फ **WASM** होगा[cite: 14, 16]. 
*   अगर सिर्फ नया ज्ञान चाहिए, तो सिर्फ **KV-Cache** होगा[cite: 14, 16]. 
*   **Sovereign Hybrid** में ही ये सब एक साथ होंगे[cite: 16].

### 🛠️ 6. `src/` फोल्डर में क्या होगा?
`src/` फोल्डर उस स्किल का **"Native Engine"** है[cite: 14, 17]. इसमें तुम्हारा असली **Rust Code** (`lib.rs`, `api.rs`) होगा, जिसे कंपाइल करके `logic.wasm` बनाया जाएगा[cite: 14].

### 🛡️ 7. यह सिस्टम कितना सुरक्षित है?
हम **"Defense-in-Depth"** का पालन करते हैं:
*   **Sandboxing**: हर स्किल एक सुरक्षित WASM सैंडबॉक्स में चलती है, जिसका तुम्हारे मुख्य OS से कोई सीधा संबंध नहीं होता[cite: 18].
*   **Permission Guard**: कोई भी स्किल बिना तुम्हारी इज़ाज़त के इंटरनेट या फाइल्स को हाथ नहीं लगा सकती[cite: 18].

### 🚦 8. स्किल्स की कैटेगरीज (Categories) कैसे काम करेंगी?
हमें फोल्डर्स बनाने की ज़रूरत नहीं है[cite: 17]. कैटेगरी की जानकारी `SKILL.md` के **Metadata** में होगी[cite: 14, 16]. इससे हम UI में "अनंत कैटेगरीज" बना सकते हैं और फ़िल्टर लगा सकते हैं[cite: 16, 17].

---


 