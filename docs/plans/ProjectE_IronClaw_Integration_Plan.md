# Project E × IronClaw Integration Plan

**Status:** In Progress
**Branch:** `claude/implement-ironclaw-integration-AqSlD`
**Date:** 2026-03-13

---

## Practical Integration Strategy

The original draft assumed Project E should be initialized inside `AppBuilder` and
stored back into `AppComponents`. After reviewing IronClaw's actual startup path,
that is not the best seam.

The recommended integration is:

1. Keep `AppBuilder::build_all()` responsible only for constructing reusable core
   components (`llm`, `workspace`, `tools`, `hooks`, `cost_guard`, etc.).
2. Initialize Project E in `src/main.rs` immediately after `build_all()` returns,
   because Project E owns long-lived background tasks rather than reusable
   components.
3. Keep Project E shutdown in `src/main.rs` alongside the existing background
   task shutdown path so task lifetimes remain explicit.
4. Use the existing `Workspace`, `CostGuard`, `LlmProvider`, and tracing pipeline
   as the first integration surface. Avoid adding new persistence or registration
   layers until the runtime loop is stable.

Why this is the better fit:

- It avoids creating a chicken-and-egg dependency where `init_project_e()`
  needs a fully-built `AppComponents` in order to populate `AppComponents`.
- It keeps tests that use `AppBuilder` lightweight and unaffected by optional
  background runtimes.
- It matches how IronClaw already manages other long-lived runtime concerns in
  `main.rs` such as channels, tunnel lifecycle, reapers, and shutdown signaling.

## Implementation Status

- `Config` bridge: implemented
- `project_e` feature/module wiring: implemented
- Runtime startup/shutdown integration in `main.rs`: implemented
- Cognitive cycle/event subscriber background tasks: implemented behind
  `--features project_e` and `PROJECT_E_ENABLED=true`
- Tool registry auto-registration for Project E modules: deferred
- Persistent registry/store integration: deferred
- Full supervisor-driven adaptive loop: deferred

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        IronClaw Binary (src/)                            │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │              src/project_e/  (integration layer)                 │   │
│  │                                                                  │   │
│  │  mod.rs              -- feature-gated module root                │   │
│  │  config.rs           -- ProjectEIronclawConfig                   │   │
│  │  llm_bridge.rs       -- IronclawLlmBridge (LLM adapter)          │   │
│  │  module_adapter.rs   -- ModuleToolAdapter<M: Module> impl Tool   │   │
│  │  hook_bridge.rs      -- CognitiveHook + EventBusSubscriber        │   │
│  │  init.rs             -- init_project_e() + ProjectEComponents    │   │
│  │  knowledge_bridge.rs -- WorkspaceKnowledgeStore adapter          │   │
│  │  cycle_runner.rs     -- CognitiveCycleRunner                     │   │
│  │  forge_runner.rs     -- ForgeSandboxRunner                       │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Config        ──► ProjectEIronclawConfig          (Phase 1)            │
│  LlmProvider   ──► IronclawLlmBridge               (Phase 2)            │
│  ToolRegistry  ◄── ModuleToolAdapter               (Phase 3)            │
│  HookRegistry  ◄─► CognitiveHook / EventBusSubscriber (Phase 4)        │
│  main.rs       ◄── ProjectEHandle                  (Phase 5)            │
│  Workspace     ◄─► WorkspaceKnowledgeStore         (Phase 6)            │
│  Runtime       ← CognitiveCycleRunner              (Phase 7)            │
│  SandboxManager ◄─► ForgeSandboxRunner             (Phase 8)            │
└─────────────────────────────────────────────────────────────────────────┘
         │ imports (no circular deps; crates stay standalone)
         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                 Project E Crates (crates/project_e_*)                    │
│                                                                          │
│  project_e_core           -- Module trait, types, config, LlmHealth     │
│  project_e_event_bus      -- EventBus, CognitiveEvent (21+3 variants)   │
│  project_e_perception     -- DataCollector, RelevanceScorer             │
│  project_e_analysis       -- BM25Index, GapReport                       │
│  project_e_metacognition  -- DeliberationEngine, PolicyEngine, Meta     │
│  project_e_forge          -- ForgePipeline (6 gates), RetryState        │
│  project_e_orchestrator   -- CycleResult, Supervisor                    │
│  project_e_registry       -- ModuleStore (state machine)                │
│  project_e_knowledge_base -- chunk_text(), RRF retrieval                │
│  project_e_skills         -- SkillCatalog, activation tiers             │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Dependency-Ordered File List

### Files to Modify
1. `Cargo.toml` — add `project_e` feature flag + 10 optional path deps
2. `src/lib.rs` — `#[cfg(feature = "project_e")] pub mod project_e;`
3. `src/config/mod.rs` — add `ProjectEIronclawConfig` field to `Config`, call `resolve()` in `build()`
4. `src/main.rs` — initialize and shut down `ProjectEHandle` around the agent runtime

### Files to Create (in dependency order)
1. `src/config/project_e.rs`
2. `src/project_e/mod.rs`
3. `src/project_e/llm_bridge.rs`
4. `src/project_e/module_adapter.rs`
5. `src/project_e/hook_bridge.rs`
6. `src/project_e/knowledge_bridge.rs`
7. `src/project_e/init.rs`
8. `src/project_e/cycle_runner.rs`
9. `src/project_e/forge_runner.rs`

---

## Phase 1: Config Bridge

**Goal:** Add `ProjectEConfig` to IronClaw's config system without touching existing config loading.

### New File: `src/config/project_e.rs`

```rust
pub struct ProjectEIronclawConfig {
    pub enabled: bool,                   // PROJECT_E_ENABLED, default false
    pub config_dir: PathBuf,             // PROJECT_E_CONFIG_DIR, default ~/.ironclaw/project_e/config
    pub instance: String,                // PROJECT_E_INSTANCE, default "default"
    pub event_bus_capacity: usize,       // PROJECT_E_BUS_CAPACITY, default 1024
    pub cycle_on_heartbeat: bool,        // PROJECT_E_CYCLE_ON_HEARTBEAT, default true
    pub forge_use_sandbox: bool,         // PROJECT_E_FORGE_SANDBOX, default true
    pub max_llm_calls_per_cycle: usize,  // PROJECT_E_MAX_LLM_CALLS, default 20
    pub inner: ProjectEConfig,           // loaded via ProjectEConfig::load_layered()
}
```

**`resolve()` logic:**
1. Read `PROJECT_E_ENABLED`; if false, return `Self { enabled: false, inner: ProjectEConfig::default(), .. }`
2. If true: read `PROJECT_E_CONFIG_DIR`, `PROJECT_E_INSTANCE`
3. Call `ProjectEConfig::load_layered(config_dir, instance)` — falls back to defaults if dir missing
4. Return full config

**`src/config/mod.rs` change:**
```rust
#[cfg(feature = "project_e")]
pub project_e: ProjectEIronclawConfig,
```
In `build()`:
```rust
#[cfg(feature = "project_e")]
project_e: ProjectEIronclawConfig::resolve()?,
```

**Cargo.toml feature flag:**
```toml
[features]
project_e = [
    "dep:project_e_core",   "dep:project_e_event_bus", "dep:project_e_perception",
    "dep:project_e_analysis", "dep:project_e_metacognition", "dep:project_e_forge",
    "dep:project_e_orchestrator", "dep:project_e_registry",
    "dep:project_e_knowledge_base", "dep:project_e_skills",
]

[dependencies]
project_e_core           = { path = "crates/project_e_core",           optional = true }
project_e_event_bus      = { path = "crates/project_e_event_bus",      optional = true }
project_e_perception     = { path = "crates/project_e_perception",     optional = true }
project_e_analysis       = { path = "crates/project_e_analysis",       optional = true }
project_e_metacognition  = { path = "crates/project_e_metacognition",  optional = true }
project_e_forge          = { path = "crates/project_e_forge",          optional = true }
project_e_orchestrator   = { path = "crates/project_e_orchestrator",   optional = true }
project_e_registry       = { path = "crates/project_e_registry",       optional = true }
project_e_knowledge_base = { path = "crates/project_e_knowledge_base", optional = true }
project_e_skills         = { path = "crates/project_e_skills",         optional = true }
```

**Resilience:** If `PROJECT_E_ENABLED=false` (default), all downstream init is skipped — zero cost.

---

## Phase 2: LLM Provider Bridge

**Goal:** Let Project E modules call IronClaw's LLM chain without owning it or duplicating circuit-breaking.

### New File: `src/project_e/llm_bridge.rs`

```rust
pub struct IronclawLlmBridge {
    provider: Arc<dyn LlmProvider>,
    cost_guard: Arc<CostGuard>,
    llm_monitor: Arc<LlmHealthMonitor>,  // Project E's cognitive-cycle-level monitor
    model_name: String,
}

impl IronclawLlmBridge {
    /// Check BOTH IronClaw's cost guard AND Project E's LLM health monitor.
    pub async fn check_allowed(&self) -> Result<(), ProjectEBridgeError>;

    /// Run a completion via IronClaw's provider chain.
    pub async fn complete(&self, prompt: &str, system: &str) -> Result<String, ProjectEBridgeError>;

    /// Record outcome in Project E's monitor after an LLM call.
    pub async fn record_outcome(&self, success: bool);
}
```

**Double circuit-breaker strategy:**
- IronClaw's `CircuitBreakerProvider` (in provider chain): trips on provider-level transient errors
- Project E's `LlmHealthMonitor`: trips on cognitive-cycle-level failures (higher threshold)
- Configure Project E's monitor with `failure_threshold = ironclaw_threshold * 2` so it trips only on confirmed degradation

**LLM-aware modules** (opt-in, no change to `Module` trait):
```rust
pub trait LlmAware {
    fn set_llm_bridge(&mut self, bridge: Arc<IronclawLlmBridge>);
}
```
`ModuleToolAdapter` calls `set_llm_bridge()` after construction if `M: LlmAware`.

**Resilience:**
- `check_allowed()` returning `Err` → `ToolError::ExecutionFailed("cognitive cycle blocked: circuit open")`
- All `LlmError` variants → `ProjectEBridgeError::Llm(msg)` — no panics

---

## Phase 3: Module → Tool Adapter

**Goal:** Make every Project E `Module` discoverable by IronClaw's `ToolRegistry` as a first-class tool.

### New File: `src/project_e/module_adapter.rs`

```rust
pub struct ModuleToolAdapter<M: Module> {
    module: Arc<tokio::sync::Mutex<M>>,
    llm_bridge: Option<Arc<IronclawLlmBridge>>,
    schema_cache: serde_json::Value,  // built from ModuleMetadata at construction time
}

impl<M: Module + 'static> Tool for ModuleToolAdapter<M> {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value { self.schema_cache.clone() }
    async fn execute(&self, params: Value, ctx: &JobContext) -> Result<ToolOutput, ToolError> {
        // 1. Check module.health_check() → return Err on Unhealthy
        // 2. Map params + ctx → ExecutionContext
        // 3. module.execute(&exec_ctx).await
        // 4. Map ModuleOutput → ToolOutput
    }
}
```

**Context mapping:**
```
ExecutionContext {
    cycle_id: ctx.job_id.unwrap_or_else(Uuid::new_v4),
    module_id: module.metadata().id,
    phase: CognitivePhase::Action,
    parameters: params,
}
```

**Output mapping:**
```
ModuleOutput { data, events, .. }
  → ToolOutput::success(data, duration)
       .with_raw(serde_json::to_string(&events).ok())
```

**`ToolRegistry` extension** (`src/tools/registry.rs` under `#[cfg(feature = "project_e")]`):
```rust
pub async fn register_project_e_module<M: Module + 'static>(
    &self,
    module: M,
    llm_bridge: Option<Arc<IronclawLlmBridge>>,
) -> Result<(), RegistryError>
// Prefixes name with "pe_" on conflict with PROTECTED_TOOL_NAMES
```

**Resilience:**
- `HealthStatus::Unhealthy(reason)` → return `ToolError::ExecutionFailed("module unhealthy: {reason}")`
- `Mutex` guards `init/teardown` (need `&mut self`); `execute` uses `&self` safely via the lock

---

## Phase 4: Event Bus → Hook Bridge

**Goal:** Connect Project E's `CognitiveEvent` stream to IronClaw's hook lifecycle bidirectionally.

### New File: `src/project_e/hook_bridge.rs`

**`CognitiveHook`** implements `Hook`:
```rust
pub struct CognitiveHook {
    event_bus: Arc<EventBus>,
    instance: String,
}
// hook_points() → [BeforeInbound, OnSessionStart]
// failure_mode() → FailOpen (never blocks the agent)
// Always returns HookOutcome::Continue { modified: None }
```
- `BeforeInbound` → emits `CognitiveEvent::CycleStarted { cycle_id: Uuid::new_v4(), instance }`
- `OnSessionStart` → same (lightweight perception trigger per new session)

**`EventBusSubscriber`** — background tokio task mapping events to IronClaw actions:

| CognitiveEvent | IronClaw Action |
|---|---|
| `SupervisorIntervention { EfficiencyCollapse }` | `tracing::warn!` + `cost_guard` notification |
| `SupervisorIntervention { ErrorStorm }` | pause new job creation via `cost_guard` |
| `CircuitBreakerOpen { provider, failures }` | `tracing::warn!` + optional channel broadcast |
| `CircuitBreakerClosed { provider }` | `tracing::info!` |
| `ModulePublished { module_id, .. }` | trigger auto-registration in `ToolRegistry` (Phase 8) |
| `HealthAlert { indicator, value }` | `tracing::warn!` |
| `CycleClosed { duration_secs }` | write `heartbeat/project_e_last_cycle.md` to workspace |

**Resilience:**
- `CognitiveHook::execute()` swallows EventBus emit errors — always returns `Continue`
- Subscriber uses `tokio::select!` with `shutdown_rx` — exits cleanly on shutdown
- `RecvError::Lagged(n)` logged and skipped; subscriber continues from latest message

---

## Phase 5: App Startup Integration

**Goal:** Add `init_project_e()` to IronClaw's runtime lifecycle without
turning `AppBuilder` into a long-lived task manager.

### New File: `src/project_e/init.rs`

```rust
pub struct ProjectEHandle {
    pub shutdown_tx: watch::Sender<bool>,
    pub runner_handle: tokio::task::JoinHandle<()>,
    pub event_subscriber_handle: tokio::task::JoinHandle<()>,
    pub event_bus: Arc<EventBus>,
}

pub fn init_project_e(
    components: &AppComponents,
) -> Option<ProjectEHandle>
```

**Initialization order (inside `init_project_e()`):**
1. `EventBus::in_memory(config.event_bus_capacity)`
2. `LlmHealthMonitor::new(llm.model_name(), config.inner.llm.circuit_breaker)`
3. `IronclawLlmBridge::new(llm, cost_guard, monitor)`
4. `ModuleStore::new()`
5. `Supervisor::new(config.inner.meta.lesson_repeat_threshold)`
6. `DeliberationEngine::new(config.inner.limits)`
7. `CognitiveHook::new(event_bus, config.instance)`
8. `hooks.register(hook.clone()).await`
9. `tokio::spawn(EventBusSubscriber::new(...).run())`

**`src/main.rs` integration:**
```rust
#[cfg(feature = "project_e")]
let project_e_handle = ironclaw::project_e::init::init_project_e(&components);
```

On shutdown:
```rust
#[cfg(feature = "project_e")]
if let Some(handle) = project_e_handle {
    let _ = handle.shutdown_tx.send(true);
    let _ = handle.runner_handle.await;
    let _ = handle.event_subscriber_handle.await;
}
```

**Resilience:** init remains optional and fail-open; when disabled or missing a
workspace, `init_project_e()` returns `None` and IronClaw continues normally.

---

## Phase 6: Knowledge Base ↔ Workspace Bridge

**Goal:** Route Project E knowledge ingestion and retrieval through IronClaw's `Workspace`.

### New File: `src/project_e/knowledge_bridge.rs`

```rust
pub struct WorkspaceKnowledgeStore {
    workspace: Arc<Workspace>,
    chunk_size: usize,    // default 512 tokens
    chunk_overlap: usize, // default 64 tokens
}

impl WorkspaceKnowledgeStore {
    /// Chunks text and writes each chunk to workspace at knowledge/{type}/{id}/{n}.md
    pub async fn ingest(&self, source_id: Uuid, text: &str, source_type: &str);

    /// Delegates to workspace hybrid FTS+vector search (already uses RRF internally).
    pub async fn search(&self, query: &str, top_k: usize) -> Vec<(String, f32)>;
}
```

**Design decision:** `Workspace::search()` already implements RRF in `src/workspace/search.rs`.
Using the workspace's built-in version avoids duplicating `project_e_knowledge_base::retrieval::reciprocal_rank_fusion()` and benefits from IronClaw's full FTS + vector pipeline.

**Resilience:**
- `workspace.write()` failures logged and ignored — knowledge errors never abort the cycle
- If `workspace = None`, `WorkspaceKnowledgeStore` is `None` and the bridge is disabled gracefully

---

## Phase 7: Heartbeat / Cycle Trigger

**Goal:** Run the cognitive cycle on IronClaw's heartbeat interval with circuit-breaking and graceful degradation.

### New File: `src/project_e/cycle_runner.rs`

```rust
pub struct CognitiveCycleRunner {
    config: ProjectEIronclawConfig,
    event_bus: Arc<EventBus>,
    llm_bridge: Arc<IronclawLlmBridge>,
    llm_health_monitor: Arc<LlmHealthMonitor>,
    supervisor: Arc<Mutex<Supervisor>>,
    deliberation: Arc<DeliberationEngine>,
    module_store: Arc<ModuleStore>,
    knowledge_store: Option<Arc<WorkspaceKnowledgeStore>>,
    workspace: Option<Arc<Workspace>>,
    interval: Duration,
    consecutive_failures: u32,        // max 3 before runner stops
    paused_until: Option<Instant>,    // set by PauseCycle intervention
    shutdown_rx: watch::Receiver<bool>,
}
```

**`run()` main loop:**
```rust
loop {
    tokio::select! {
        _ = interval.tick() => {
            if !self.should_run_cycle() { continue; }
            match self.run_one_cycle().await {
                Ok(result) => { self.consecutive_failures = 0; self.write_cycle_summary(&result).await; }
                Err(e) => {
                    tracing::warn!("cognitive cycle error: {e}");
                    self.consecutive_failures += 1;
                    if self.consecutive_failures >= 3 { break; }
                }
            }
        }
        _ = self.shutdown_rx.changed() => break,
    }
}
```

**`should_run_cycle()` guards:**
1. `paused_until` active → skip
2. `llm_health_monitor.check_before_call().is_err()` → skip (circuit open)
3. `config.cycle_on_heartbeat == false` → skip

**`run_one_cycle()` scope (Phase 7):**
1. Emit `CycleStarted`
2. Run perception → collect data
3. Run deliberation → `ActionPlan`
4. Emit `ActionPlanReady` + `CycleClosed`
5. Return `CycleResult`

Full 6-phase execution (analysis, forge, evaluation, meta-adaptation) is Phase 8.

**`write_cycle_summary(result)`** writes to workspace `heartbeat/project_e_last_cycle.md`:
```markdown
## Last Cognitive Cycle
Completed at: {timestamp}
Gaps detected: {N}
Modules published: {M}
Operating mode: {Nominal|Cautious|Recovery}
```
The existing `HeartbeatRunner` includes this file naturally in its workspace scan.

**Supervisor intervention handling:**

| Intervention | Action in runner |
|---|---|
| `PauseCycle { duration_secs }` | `paused_until = Some(Instant::now() + duration)` |
| `ForceRecoveryMode` | flag forces `OperatingMode::Recovery` in all `deliberate()` calls |
| `SkipPhase { phase }` | skip that phase in next `run_one_cycle()` |
| `ModelEscalation` | log + pass hint to `IronclawLlmBridge` |

**Spawning in `src/main.rs`:**
The current implementation keeps spawning encapsulated inside
`init_project_e()`. `main.rs` owns only the resulting `ProjectEHandle` and the
shutdown signal.

**Resilience:** 3 consecutive failures → runner exits; main chat loop completely unaffected.

---

## Phase 8: Forge Pipeline ↔ Sandbox Bridge

**Goal:** Wire forge gates to IronClaw's subprocess/sandbox infrastructure; auto-register built modules.

### New File: `src/project_e/forge_runner.rs`

```rust
pub struct ForgeSandboxRunner {
    sandbox_manager: Option<Arc<SandboxManager>>,
    use_sandbox: bool,
    tool_registry: Arc<ToolRegistry>,
    module_store: Arc<ModuleStore>,
    event_bus: Arc<EventBus>,
}
```

**`run_pipeline(source, workdir, config, cycle_id, gap_id, module_name) -> ForgeOutcome`:**
1. Emit `ForgeStarted`
2. `run_gates(source, workdir, config).await` (direct subprocess; Docker deferred)
3. Gate pass → emit `GatePass`; gate fail → emit `GateFail` → return `Failure`
4. All gates pass:
   - Emit `ModulePublished`
   - `module_store.transition(id, Active).await`
   - If WASM artifact exists: `tool_registry.register_wasm_module(path).await`
5. After all retries exhausted with failure:
   - Emit `ModuleRejected`
   - `supervisor.record_anomaly(RepeatedCompilationErrors)` → if `Some(intervention)` → emit `SupervisorIntervention`

**Resilience:**
- Forge failures are `ForgeOutcome::Failure` — normal variant, never crashes cycle
- `cargo audit` gate skips silently if binary absent
- WASM registration failure logged and non-fatal

---

## Resilience Summary

| Risk | Mitigation |
|---|---|
| Project E init failure | `match init_project_e()` → on error `project_e: None`; IronClaw continues |
| LLM circuit open at cycle start | `should_run_cycle()` checks `llm_health_monitor` before starting |
| Cycle crash (repeated) | `consecutive_failures >= 3` → runner exits; agent loop unaffected |
| EventBus subscriber lag | `RecvError::Lagged` logged + skipped; subscriber continues |
| `ErrorStorm` intervention | `EventBusSubscriber` notifies `cost_guard` to pause new jobs |
| `PauseCycle` intervention | `CognitiveCycleRunner` sets `paused_until`; no locks held |
| `ForceRecoveryMode` | Deliberation defers all forge actions; normal chat continues |
| Forge gate timeout | `ForgeConfig.*_timeout_secs` enforced in existing gate code |
| Module tool unhealthy | `health_check()` before `execute()` → `ToolError::ExecutionFailed` |
| Double circuit-breaking | Project E monitor threshold = `ironclaw_threshold × 2` |
| Knowledge store write failure | Logged + ignored; cycle continues without persistence |

---

## Deferred Items (Runtime Dependencies)

| Item | Requires |
|---|---|
| NATS transport | External NATS broker; `InMemoryEventBus` used throughout |
| Persistent `ModuleStore` | New DB table + PostgreSQL/libSQL migrations |
| Docker sandbox for forge gates | `SandboxManager` + Docker at runtime |
| WASM compilation from forge output | `wasm32-wasi` toolchain in execution env |
| `cargo audit` gate | `cargo-audit` binary in PATH |
| Full 6-phase cognitive cycle | Phase 7 implements Perception→Deliberation only initially |
| Phase model routing | `PhaseModelRouter` plumbed but inactive |
| Vector embeddings for knowledge | `EmbeddingProvider` wired to `WorkspaceKnowledgeStore` |
| Web UI config for Project E | Frontend work |

---

## Build & Test

```bash
# Build with Project E feature
cargo build --features project_e

# Lint
cargo clippy --all --features project_e --benches --tests

# Test all including Project E crates
cargo test --features project_e

# Run with Project E enabled
PROJECT_E_ENABLED=true RUST_LOG=ironclaw=debug cargo run --features project_e
```
