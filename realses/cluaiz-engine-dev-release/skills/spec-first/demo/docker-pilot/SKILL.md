---
skill_id: dev.docker-pilot
version: 1.0.0
runtime_mode: wasm_native
category: dev-suite
capabilities:
  - dev-tools
  - container-automation
triggers:
  - docker
  - container
permissions:
  - workspace.read
  - workspace.write
  - shell.safe
soul_type: STEERING_VECTOR
security_level: strict
---

# SKILL: Docker Pilot

## Intent Contract
Assist users with deterministic Docker workflows for build, run, inspect, and cleanup.

## Router Contract
- semantic_triggers:
  - docker
  - container
- entropy_threshold: 0.25

## Permission Contract
- allow_network: false
- allow_filesystem_write: true
- deny_shell_destructive: true

## MCP Contract
- enabled: false

## WASM Contract
- language: rust
- entrypoint: run

## Neural Contract
- profile: docker-pilot-v1
- stitching_mode: optional

## Build Directives
```yaml
build:
  manifest: { enabled: true }
  mcp: { enabled: false }
  wasm: { enabled: true, crate_path: ./src }
  kv_cache: { enabled: false }
  signing: { enabled: true }
```
