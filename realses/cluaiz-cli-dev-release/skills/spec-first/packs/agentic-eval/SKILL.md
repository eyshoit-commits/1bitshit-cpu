---
skill_id: core.agentic-eval
version: 1.0.0
runtime_mode: sovereign_hybrid
category: core-intelligence
capabilities:
  - eval
  - benchmark
  - quality-audit
triggers:
  - evaluate this
  - run eval
  - benchmark agent
permissions:
  - workspace.read
  - workspace.write
  - network.restricted
soul_type: KV_CACHE
security_level: strict
---

# SKILL: Agentic Eval

## Intent Contract
Evaluate agent outputs with measurable rubric and actionable fixes.

## Router Contract
- semantic_triggers:
  - evaluate
  - benchmark
  - quality check
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
1. Score correctness/completeness/safety with evidence.
2. Return failure reasons for hallucination or tool misuse.
3. Build must fail if required artifacts are missing.

## Security Guards
- Never call undeclared connectors.
- Never bypass permission guard.
- Never execute destructive operations.
