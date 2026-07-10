# Spec-First Full Demo

This demo shows the complete flow for Cluaiz spec-first skills.

## What is included

- Authoring source: `SKILL.md`
- Generated artifacts:
  - `artifacts/manifest.json`
  - `artifacts/connector.mcp`
  - `artifacts/build-output.json`

## Demo skills

1. `agentic-eval` (sovereign_hybrid)
2. `docker-pilot` (wasm_native)

## Flow

1. Write `SKILL.md`
2. Run builder (future command):
   - `cluaiz skill build ./SKILL.md`
3. Builder validates and generates artifacts
4. Installer registers skill and hot-swaps runtime

## Notes

- In this demo, `logic.wasm` and `state.kv-cache` are represented as build outputs metadata, not binary payload.
- In production, builder generates or links actual binaries in artifacts.
