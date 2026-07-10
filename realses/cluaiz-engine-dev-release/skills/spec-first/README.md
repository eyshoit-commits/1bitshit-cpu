# Cluaiz Spec-First (Archer Native Standard)

This is not a prompt-only format.

`SKILL.md` is an authoring contract that compiles into the native Cluaiz runtime package:
- `manifest.json` (passport)
- `connector.mcp` (hands)
- `logic.wasm` (body)
- `state.kv-cache` (soul)

If any required runtime part is missing for declared `runtime_mode`, build must fail.

## Core Principle

1. Human writes one canonical file: `SKILL.md`.
2. Rust build pipeline validates and compiles it.
3. Runtime artifacts are generated and signed.
4. Neural Foundry loads artifacts, not raw markdown.

## Directory Contract

```text
skills/
  spec-first/
    packs/
      <skill-id>/
        SKILL.md               # only human-edited source
        artifacts/             # machine-generated
          manifest.json
          connector.mcp
          logic.wasm
          state.kv-cache
```

## Runtime Modes (strict)

- `prompt_only`: only for lightweight behavior steering (discouraged for production)
- `mcp_native`: requires `manifest.json + connector.mcp`
- `wasm_native`: requires `manifest.json + logic.wasm`
- `neural_native`: requires `manifest.json + state.kv-cache`
- `sovereign_hybrid`: requires all four artifacts

## Build Rules (must enforce)

1. Parse frontmatter from `SKILL.md`.
2. Validate required keys and permission scopes.
3. Generate `manifest.json` from metadata and policies.
4. Generate `connector.mcp` from declared tool contracts.
5. Compile Rust source (or template module) -> `logic.wasm` when wasm path enabled.
6. Generate/attach `state.kv-cache` profile when neural path enabled.
7. Run checksum/signing for artifacts.
8. Fail closed: no partial install on validation or compile failure.

## Required Frontmatter Keys

- `skill_id`
- `version`
- `runtime_mode`
- `category`
- `capabilities`
- `triggers`
- `permissions`
- `soul_type`
- `security_level`

## Installation Vision

Future command:

```bash
cluaiz skill install ./packs/<skill-id>/SKILL.md
```

Installer behavior:
1. Compile `SKILL.md` -> runtime artifacts.
2. Register in skill registry.
3. Hot-swap activate without engine restart.

## What this solves

- Keeps Archer-level native power (WASM + KV + MCP).
- Removes manual multi-file authoring pain.
- Preserves strict security and deterministic runtime.

