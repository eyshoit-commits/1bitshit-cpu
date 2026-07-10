---
skill_id: core.agentic-eval
version: 1.0.0
runtime_mode: sovereign_hybrid
category: core-intelligence
capabilities:
  - eval
  - benchmark
triggers:
  - evaluate this
  - run eval
permissions:
  - workspace.read
  - workspace.write
  - network.restricted
soul_type: KV_CACHE
security_level: strict
---

# SKILL: Agentic Eval (Sovereign Native)

## Intent Contract
Define the mission of the skill in one paragraph.

## Router Contract
- semantic_triggers:
  - evaluate
  - benchmark
- entropy_threshold: 0.35

## Permission Contract
- allow_network: true
- allow_filesystem_write: true
- deny_shell_destructive: true

## MCP Contract
- server_id: eval-dataset-server
- tools:
  - name: fetch_eval_cases
    input_schema: { "suite": "string" }

## WASM Contract
- language: rust
- entrypoint: run
- io:
  input: EvalRequest
  output: EvalReport

## Neural Contract
- profile: agentic-eval-v1
- stitching_mode: mmap
- rope_offset_policy: virtual_prefix

## Build Directives
```yaml
build:
  manifest:
    enabled: true
  mcp:
    enabled: true
  wasm:
    enabled: true
    crate_path: ./src
  kv_cache:
    enabled: true
    source_profile: agentic-eval-v1
  signing:
    enabled: true
```

## Acceptance Tests
1. Given a bad response, skill must produce rubric score + evidence.
2. If connector permission missing, install/build must fail.
3. If wasm compile fails, package must not be installable.

## Security Guards
- Never allow undeclared tool calls.
- Never bypass permission guard.
- Never execute destructive actions during evaluation.
