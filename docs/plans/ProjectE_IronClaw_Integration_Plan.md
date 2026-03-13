# Project E × IronClaw Integration Plan

**Objective:** Build Project E by transplanting its six-phase cognitive architecture onto the IronClaw codebase, keeping IronClaw's battle-tested Rust infrastructure and replacing or augmenting it with Project E's system logic.

**Strategy in one sentence:** IronClaw provides the engine room (sandbox, hybrid search, tool trait, LLM provider, job scheduler, circuit breaker); Project E provides the brain (cognitive cycle, module trait, forge pipeline, metacognition, meta-adaptation, sector config).

---

## Table of Contents

- [1. Guiding Philosophy](#1-guiding-philosophy)
- [2. Component Decision Map](#2-component-decision-map)
- [3. Workspace Anatomy After Integration](#3-workspace-anatomy-after-integration)
- [4. Phase 0 — Repository Foundation](#4-phase-0--repository-foundation)
- [5. Phase 1 — Module Trait and Event Bus Protocol](#5-phase-1--module-trait-and-event-bus-protocol)
- [6. Phase 2 — Perceive (Web Perception Layer)](#6-phase-2--perceive-web-perception-layer)
- [7. Phase 3 — Interpret (Analysis Engine)](#7-phase-3--interpret-analysis-engine)
- [8. Phase 4 — Deliberate (Metacognition Layer)](#8-phase-4--deliberate-metacognition-layer)
- [9. Phase 5 — Act (The Forge Pipeline)](#9-phase-5--act-the-forge-pipeline)
- [10. Phase 6 — Evaluate (Evaluation Engine)](#10-phase-6--evaluate-evaluation-engine)
- [11. Phase 7 — Meta-Adapt (Self-Improvement Loop)](#11-phase-7--meta-adapt-self-improvement-loop)
- [12. Phase 8 — Orchestrate (Daemon and Cycle Loop)](#12-phase-8--orchestrate-daemon-and-cycle-loop)
- [13. Phase 9 — Interface and Deployment](#13-phase-9--interface-and-deployment)
- [14. Migration Decision per Existing Project E Crate](#14-migration-decision-per-existing-project-e-crate)
- [15. Dependency Order and Critical Path](#15-dependency-order-and-critical-path)
- [16. Success Criteria per Phase](#16-success-criteria-per-phase)
- [17. Risk Register](#17-risk-register)

---

## 1. Guiding Philosophy

### Why IronClaw as the Base?

IronClaw is a working Rust agent system with production-quality implementations of the exact infrastructure pieces that Project E's architectural audit identified as either missing or fragile:

| IronClaw Strength | Project E Gap It Fills |
|---|---|
| Wasmtime per-invocation sandbox with `capabilities.json` | Process sandbox is crude; forge needs sandboxed execution |
| Hybrid FTS + pgvector + RRF search | Knowledge base uses cosine-only retrieval; no BM25/FTS layer |
| Tool trait with hot-reload registry | Tool registry and module registry are separate, not unified |
| Self-repair + heartbeat | Evaluation Engine (Phase 5) is the biggest audit gap |
| Tokio job scheduler with state machine | Daemon CycleOrchestrator loop is a placeholder |
| LLM provider protocol with fallback chain | No LLM circuit breaker; cascade failures on provider timeout |
| AGENTS.md / SKILL.md / HEARTBEAT.md patterns | Project E uses the same pattern — direct structural match |
| BM25 deferred tool loading | Module registry query floods LLM context on large catalogs |

### What to Keep from Each System

**Keep from IronClaw (infrastructure):**
- `src/tools/wasm/` Wasmtime sandbox + `capabilities.json` sidecar
- `src/workspace/` hybrid search (FTS + pgvector + RRF fusion)
- `src/llm/` provider protocol with `ModelCapabilities` and fallback chain
- `src/agent/` job scheduler and state machine
- `src/agent/repair.rs` self-repair + heartbeat
- `src/safety/` injection defense and secret redaction
- `src/tools/builder/` LLM-to-artifact build loop pattern
- `src/extensions/` runtime discovery pattern (becomes skill discovery)
- PostgreSQL/pgvector schema and migration runner
- Axum MCP server pattern

**Keep from Project E (cognitive architecture):**
- Six-phase cognitive cycle definition
- `Module` trait contract (seven methods)
- NATS JetStream event bus as control plane with typed topics
- Sector-agnostic TOML configuration model
- Module lifecycle state machine (Draft → Compiling → Testing → Active → Deprecated → Removed)
- `GapReport` and `ActionPlan` typed structs
- Six forge validation gates
- Metacognition self-model (five health dimensions)
- Meta-adaptation adaptive TOML overlay persisting learned thresholds
- `SYSTEM_SOUL.md` / `HEARTBEAT.md` / `BOOT.md` constitutional files
- Skills system (SKILL.md, three-tier progressive disclosure, six bundled skills)
- Supervisor agent with seven anomaly patterns and intervention types
- Homeostatic regulation (pruning, redundancy detection, auto-deprecation)

### What to Discard

**From IronClaw:**
- NEAR AI authentication / blockchain-specific auth
- `SOUL.md` (personality file for interactive chat) — replaced by `SYSTEM_SOUL.md`
- libSQL dual-backend — Project E uses PostgreSQL only
- Telegram / Slack channel handlers — Project E is headless autonomous
- Interactive approval workflows (human-in-the-loop pauses) — replaced by `deliberation.risk_threshold`
- WASM as the *published module format* — Project E modules are native Rust binaries

**From Project E (replaced by IronClaw equivalents):**
- Process-level sandbox → IronClaw Wasmtime sandbox (for forge environment only)
- Cosine-only knowledge base retrieval → IronClaw RRF hybrid search
- Ad-hoc tool trait → unified with IronClaw's typed tool trait
- Placeholder daemon loop → IronClaw job scheduler

---

## 2. Component Decision Map

| Component | Source | Action | Notes |
|---|---|---|---|
| Tool trait + registry | IronClaw | **Extend** into Module trait | Add 7 Module methods on top |
| Wasmtime sandbox | IronClaw | **Keep, repurpose** | For forge build environment, not published module runtime |
| Hybrid search (FTS+pgvector+RRF) | IronClaw | **Keep as-is** | Powers knowledge base retrieval in Phase 1 and Phase 2 |
| LLM provider protocol + capabilities | IronClaw | **Keep, extend** | Add per-phase model routing |
| Circuit breaker / `BackendHealthMonitor` | IronClaw (via Turnstone pattern) | **Port to Rust** | Critical missing piece — add to `project-e-core` |
| Job scheduler + state machine | IronClaw | **Adapt** | Becomes `CycleOrchestrator`; states map to cognitive phases |
| Self-repair + heartbeat | IronClaw | **Adapt** | Becomes MVP Evaluation Engine; extend with causal metrics |
| BM25 deferred loading | IronClaw | **Port** | For module discovery in Phase 2 and Phase 3 |
| `capabilities.json` sidecar | IronClaw | **Port** | Per-module security manifest in forge pipeline |
| AGENTS.md / SKILL.md | IronClaw | **Align** | Project E uses same pattern; merge file sets |
| Six-phase cognitive cycle | Project E | **Keep, wire to IronClaw** | IronClaw provides the execution engine |
| `Module` trait (7 methods) | Project E | **Keep, implement** | On top of IronClaw's extended tool base |
| NATS JetStream event bus | Project E | **Keep** | Add typed `CognitiveEvent` protocol (Turnstone pattern) |
| TOML sector config | Project E | **Keep, add TOML keys** | Add IronClaw's LLM circuit breaker params |
| Module lifecycle FSM | Project E | **Keep, enforce** | Wire state transitions to IronClaw's state machine pattern |
| `GapReport` + `ActionPlan` types | Project E | **Keep, enrich** | Add Cynefin/Wardley fields |
| Six forge validation gates | Project E | **Keep** | IronClaw's builder pattern wraps them |
| Metacognition self-model | Project E | **Keep** | Wire to IronClaw's heartbeat for live health data |
| Adaptive TOML overlay (meta-adapt) | Project E | **Keep** | Extend to cover IronClaw-added config params |
| `SYSTEM_SOUL.md` / `HEARTBEAT.md` | Project E | **Keep** | Replace IronClaw's `SOUL.md` and `HEARTBEAT.md` |
| Skills system (SKILL.md + 6 core skills) | Project E | **Keep** | Directly compatible with IronClaw's extension pattern |
| Supervisor agent | Project E | **Keep** | Wire to IronClaw's circuit breaker and model escalation |
| Homeostatic regulation | Project E | **Keep** | Integrate with IronClaw's eviction pattern |
| Python scrapers (ERSE, OMIE) | Project E | **Keep** | Publish to NATS; IronClaw has no equivalent |
| MCP server (Axum + rmcp) | Both (similar) | **Merge** | Use IronClaw's Axum base, Project E's tool definitions |
| Registry (PostgreSQL) | Project E | **Unify** | Fix dual-state bug; use IronClaw's single source-of-truth pattern |
| NEAR AI auth | IronClaw | **Remove** | Not relevant |
| Redis | Project E (unused) | **Remove** | Dead infrastructure |

---

## 3. Workspace Anatomy After Integration

The resulting Cargo workspace uses IronClaw's binary-centric structure but reorganizes crates into Project E's cognitive layer model. Crate names follow Project E's convention.

```
project-e/                             ← forked from IronClaw, renamed
├── Cargo.toml                         ← workspace manifest
│
├── crates/
│   │
│   ├── ── FOUNDATION (from IronClaw, kept/extended) ──
│   ├── core/                          ← project-e-core
│   │   └── src/
│   │       ├── types.rs               ← shared types, errors
│   │       ├── config.rs              ← TOML loader (layered: default → instance → env → adaptive)
│   │       ├── sandbox.rs             ← Wasmtime sandbox API (from IronClaw src/tools/wasm/)
│   │       ├── llm_health.rs          ← NEW: LlmHealthMonitor circuit breaker
│   │       ├── secret_redactor.rs     ← from IronClaw src/safety/
│   │       └── middleware.rs          ← from IronClaw
│   │
│   ├── event-bus/                     ← project-e-event-bus
│   │   └── src/
│   │       ├── lib.rs                 ← NATS JetStream abstraction
│   │       ├── topics.rs              ← typed topic taxonomy
│   │       └── protocol.rs            ← NEW: CognitiveEvent enum (typed protocol)
│   │
│   ├── ── COGNITIVE (Project E logic, IronClaw execution) ──
│   ├── perception/                    ← project-e-perception
│   │   └── src/
│   │       ├── collector.rs           ← DataCollector (Firecrawl/Tavily)
│   │       ├── relevance.rs           ← RelevanceScorer: novelty × coverage × magnitude
│   │       └── hybrid_search.rs       ← IronClaw RRF search adapted for scraped_data
│   │
│   ├── analysis/                      ← project-e-analysis
│   │   └── src/
│   │       ├── engine.rs              ← LLM-as-Judge gap detection
│   │       ├── gap_report.rs          ← GapReport + Cynefin + Wardley fields
│   │       └── bm25.rs                ← NEW: BM25 index for module discovery
│   │
│   ├── metacognition/                 ← project-e-metacognition
│   │   └── src/
│   │       ├── self_model.rs          ← SelfModelBuilder: 5 health dimensions
│   │       ├── deliberation.rs        ← DeliberationEngine: Recovery/Cautious/Nominal
│   │       ├── policy.rs              ← NEW: glob-pattern tool policy engine
│   │       └── evaluation.rs          ← EvaluationEngine (wraps IronClaw self-repair)
│   │
│   ├── forge/                         ← project-e-forge
│   │   └── src/
│   │       ├── pipeline.rs            ← 6 validation gates orchestration
│   │       ├── generator.rs           ← LLM code generation with skill injection
│   │       ├── gates/
│   │       │   ├── unsafe_check.rs
│   │       │   ├── forbidden.rs
│   │       │   ├── compile.rs         ← cargo build inside Wasmtime sandbox
│   │       │   ├── clippy.rs
│   │       │   ├── audit.rs
│   │       │   └── test.rs
│   │       ├── retry.rs               ← best-candidate tracking across retries
│   │       └── capabilities.rs        ← capabilities.json sidecar generation
│   │
│   ├── registry/                      ← project-e-registry
│   │   └── src/
│   │       ├── store.rs               ← PostgreSQL as single source-of-truth
│   │       ├── cache.rs               ← in-memory read-through cache
│   │       └── lifecycle.rs           ← state machine: Draft→Active→Removed
│   │
│   ├── knowledge-base/                ← project-e-knowledge-base
│   │   └── src/
│   │       ├── ingestion.rs           ← chunk → embed → upsert
│   │       ├── retrieval.rs           ← RRF hybrid: FTS + pgvector cosine
│   │       └── generation_history.rs
│   │
│   ├── skills/                        ← project-e-skills
│   │   └── src/
│   │       ├── catalog.rs             ← SKILL.md discovery and indexing
│   │       ├── activation.rs          ← 3-tier progressive disclosure
│   │       └── author.rs              ← SkillAuthor: stable module → SKILL.md
│   │
│   ├── ── ORCHESTRATION ──
│   ├── orchestrator/                  ← project-e-orchestrator
│   │   └── src/
│   │       ├── cycle.rs               ← CycleOrchestrator: IronClaw job scheduler → 6 phases
│   │       ├── phases.rs              ← phase dispatch: perceive/interpret/.../meta-adapt
│   │       └── supervisor.rs          ← Supervisor agent: 7 anomaly patterns
│   │
│   ├── ── INTERFACE ──
│   ├── cli/                           ← project-e-cli
│   ├── mcp-server/                    ← project-e-mcp-server
│   └── tui/                           ← project-e-tui
│
├── config/
│   ├── default.toml                   ← base: infra, limits, defaults, circuit breaker
│   ├── energy-pt.toml                 ← instance: sector=energy, region=portugal
│   └── runtime/                       ← written by meta-adaptation at runtime
│       └── <instance>.adaptive.toml
│
├── migrations/                        ← PostgreSQL schema (Project E's)
├── prompts/                           ← versioned LLM prompt templates
├── scrapers/                          ← Python ERSE/OMIE producers (unchanged)
├── skills/                            ← 6 bundled SKILL.md files (unchanged)
├── agents/                            ← agent profiles (YAML + markdown)
├── docker/
│
├── SYSTEM_SOUL.md                     ← Project E constitutional identity
├── HEARTBEAT.md                       ← maintenance policy
├── BOOT.md                            ← startup checklist
├── AGENTS.md                          ← agent index and pipeline workflow
└── CLAUDE.md                          ← development guidance for Claude Code
```

---

## 4. Phase 0 — Repository Foundation

**Duration:** Week 1  
**Goal:** Fork IronClaw, establish the workspace structure, and remove components that are being replaced.

### 4.1 Fork and Rename

```bash
git clone https://github.com/nearai/ironclaw project-e
cd project-e
# Update Cargo.toml workspace name
# Rename src/ → crates/ following Project E's layout
# Update all internal crate references
```

### 4.2 Remove IronClaw Components Not Needed

Delete or empty the following from IronClaw:

| IronClaw Path | Reason |
|---|---|
| `src/auth/near_ai.rs` | NEAR AI blockchain auth — not needed |
| `src/channels/` (Telegram, Slack, Email handlers) | Project E is headless autonomous |
| `SOUL.md` | Replaced by `SYSTEM_SOUL.md` |
| Any `libSQL` backend code | PostgreSQL only |
| Human-in-the-loop approval middleware | Replaced by `deliberation.risk_threshold` |

### 4.3 Add Constitutional Files from Project E

Copy the following files to the repository root:

```
SYSTEM_SOUL.md      ← behavioral identity (immutable)
HEARTBEAT.md        ← maintenance policy
BOOT.md             ← startup checklist
AGENTS.md           ← agent index
CLAUDE.md           ← development guidance
```

### 4.4 Establish Layered TOML Configuration

Create `config/default.toml` with all base configuration domains. Add new IronClaw-sourced sections that Project E was missing:

```toml
# --- NEW: LLM circuit breaker (from IronClaw BackendHealthMonitor) ---
[llm.circuit_breaker]
enabled = true
probe_interval = "30s"
failure_threshold = 5
cooldown = "60s"
probe_timeout = "5s"

# --- NEW: Model capabilities registry ---
[llm.capabilities]
auto_detect = true          # probe provider on startup

# --- NEW: Per-phase model routing ---
[llm.phase_routing]
perception = "model_for_analysis"       # cheap
interpretation = "model_for_analysis"   # structured JSON output
deliberation = "model_for_analysis"     # reasoning
action = "model_for_generation"         # code generation (most capable)
evaluation = "model_for_analysis"       # judgment
meta_adaptation = "model_for_analysis"  # introspection
```

### 4.5 Add PostgreSQL Migrations from Project E

The IronClaw schema and Project E's schema cover different tables. Merge migrations:
- Run Project E migrations `001` through `005` on top of IronClaw's schema
- IronClaw's tool/workspace tables become supplementary
- `modules`, `module_state_transitions`, `test_results`, `gap_reports`, `knowledge_embeddings`, `skills`, `skill_activations` are all Project E's domain

### 4.6 Success Criteria

- [ ] `cargo build --workspace` compiles with zero errors after removal of NEAR AI and channel code
- [ ] PostgreSQL migrations run clean on a fresh database
- [ ] Constitutional files present at repo root
- [ ] `config/default.toml` loads with new circuit breaker keys

---

## 5. Phase 1 — Module Trait and Event Bus Protocol

**Duration:** Week 1–2  
**Goal:** Establish the two foundational contracts: the `Module` trait and the typed NATS event protocol.

### 5.1 Extend IronClaw's Tool Trait into the Module Trait

IronClaw's tool system uses a `Tool` trait with `execute()`, `description()`, and schema methods. The Module trait is a superset of this. The integration approach: make `Module` a supertrait that extends the existing tool infrastructure, so all modules are automatically tools.

**File:** `crates/core/src/module_trait.rs`

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The seven-method contract every Project E module must implement.
/// Extends IronClaw's Tool trait so modules are automatically
/// discoverable by the existing tool registry.
#[async_trait]
pub trait Module: Send + Sync {
    /// Identity: unique ID, name, version, category, tags, UI hints,
    /// runtime metadata (topics, resource budget, health SLA).
    fn metadata(&self) -> ModuleMetadata;

    /// One-time initialization. Must succeed before ACTIVE state.
    async fn init(&mut self, config: &InstanceConfig) -> Result<()>;

    /// Main business logic. Returns structured output with type hint.
    async fn execute(&self, ctx: &ExecutionContext) -> Result<ModuleOutput>;

    /// Declares which NATS topics this module subscribes to.
    fn subscriptions(&self) -> Vec<EventTopic>;

    /// Processes an incoming event; may emit zero or more new events.
    async fn handle_event(&self, event: &Event) -> Result<Vec<Event>>;

    /// Reports current health: Healthy / Degraded(reason) / Unhealthy(reason).
    fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }

    /// Graceful cleanup before shutdown.
    async fn teardown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Forge pipeline rejects modules failing these constraints:
/// - No `unsafe` blocks (AST-checked, not regex)
/// - No `unwrap()`, `panic!()`, `todo!()`, `unimplemented!()`
/// - Only whitelisted crate dependencies
/// - All inter-module communication via NATS events (no direct calls)
```

**Constraint enforcement at forge time (not in the trait itself):**

```rust
// crates/forge/src/gates/unsafe_check.rs
pub fn check_no_unsafe(source: &str) -> Result<(), GateError> {
    // Use syn to parse the AST; regex is insufficient
    let syntax = syn::parse_file(source)?;
    if has_unsafe_block(&syntax) {
        return Err(GateError::UnsafeBlock);
    }
    Ok(())
}
```

### 5.2 Define the Typed NATS Event Protocol

Currently Project E's event bus uses string topics without typed message structs. This is the highest-leverage fix from the Turnstone analysis. Define a Rust enum covering every cognitive-phase transition.

**File:** `crates/event-bus/src/protocol.rs`

```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Every event published on NATS carries this envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<P> {
    pub event_id: Uuid,
    pub cycle_id: Uuid,          // stable ID linking all events of one cognitive cycle
    pub correlation_id: Uuid,    // links request/response pairs
    pub topic: String,
    pub schema_version: u8,
    pub producer: String,
    pub timestamp: DateTime<Utc>,
    pub payload: P,
}

/// All cognitive-phase transition events as a typed enum.
/// Serialized as JSON to NATS; deserialized by each phase consumer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CognitiveEvent {
    // Phase 1 — Perceive
    CycleStarted       { cycle_id: Uuid, instance: String },
    PerceptionComplete { cycle_id: Uuid, data_count: usize, avg_relevance: f32 },
    CrawlComplete      { cycle_id: Uuid, url: String, items: usize },

    // Phase 2 — Interpret
    GapReportReady     { cycle_id: Uuid, report_id: Uuid, gap_count: usize,
                         conflict_count: usize, confirmation_count: usize },
    GapDetected        { cycle_id: Uuid, gap_id: Uuid, priority: Priority },

    // Phase 3 — Deliberate
    SelfModelUpdated   { cycle_id: Uuid, mode: OperatingMode, efficiency: f32 },
    ActionPlanReady    { cycle_id: Uuid, plan_id: Uuid, actions_count: usize },
    ActionDeferred     { cycle_id: Uuid, gap_id: Uuid, reason: String },

    // Phase 4 — Act
    ForgeStarted       { cycle_id: Uuid, gap_id: Uuid, module_name: String },
    GatePass           { cycle_id: Uuid, module_id: Uuid, gate: GateName },
    GateFail           { cycle_id: Uuid, module_id: Uuid, gate: GateName, error: String },
    ModulePublished    { cycle_id: Uuid, module_id: Uuid, module_name: String },
    ModuleRejected     { cycle_id: Uuid, gap_id: Uuid, reason: String, attempts: u8 },

    // Phase 5 — Evaluate
    EvaluationComplete { cycle_id: Uuid, published: usize, rejected: usize,
                         efficiency: f32, skill_scores: Vec<(String, f32)> },
    ModuleDeprecated   { cycle_id: Uuid, module_id: Uuid, reason: String },

    // Phase 6 — Meta-Adapt
    ParameterAdjusted  { cycle_id: Uuid, param: String,
                         old_value: f32, new_value: f32, justification: String },
    ConstitutionalLesson { cycle_id: Uuid, lesson: String },
    CycleClosed        { cycle_id: Uuid, duration_secs: u64 },

    // Cross-cutting
    HealthAlert        { indicator: HealthIndicator, value: f32, threshold: f32 },
    SupervisorIntervention { pattern: AnomalyPattern, intervention: InterventionType },
    CircuitBreakerOpen { provider: String, failures: u32 },
    CircuitBreakerClosed { provider: String },
}
```

**Topic naming follows Project E's existing taxonomy:**

```rust
impl CognitiveEvent {
    pub fn topic(&self) -> &'static str {
        match self {
            Self::CycleStarted { .. }         => "project_e.system.cycle_started",
            Self::PerceptionComplete { .. }   => "project_e.perception.complete",
            Self::GapReportReady { .. }       => "project_e.analysis.gap_report_ready",
            Self::ActionPlanReady { .. }      => "project_e.meta.action_plan_created",
            Self::ModulePublished { .. }      => "project_e.forge.module_published",
            Self::EvaluationComplete { .. }   => "project_e.meta.cycle_evaluated",
            Self::ParameterAdjusted { .. }    => "project_e.meta.parameter_adjusted",
            Self::CircuitBreakerOpen { .. }   => "project_e.system.circuit_open",
            // ...
        }
    }
}
```

### 5.3 Add LLM Circuit Breaker to Core

Port IronClaw's `BackendHealthMonitor` to Project E's `project-e-core`. This fills the missing LLM resilience gap identified in the architectural audit.

**File:** `crates/core/src/llm_health.rs`

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,                 // healthy, requests pass through
    Open { opened_at: Instant },    // failing, requests rejected
    HalfOpen,               // testing recovery with single probe
}

pub struct LlmHealthMonitor {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    config: CircuitBreakerConfig,
}

impl LlmHealthMonitor {
    /// Called by the CycleOrchestrator before each LLM phase.
    /// Returns Err if circuit is Open (caller should pause cycle).
    pub async fn check_before_call(&self) -> Result<(), CircuitOpen> { ... }

    /// Called after each LLM response. Records success or failure.
    pub async fn record_outcome(&self, success: bool) { ... }

    /// Background task: periodic probe of the OpenRouter endpoint.
    pub async fn run_probe_loop(&self, event_bus: Arc<EventBus>) { ... }
}
```

**Wire to CycleOrchestrator:** Before each phase that calls LLM (Interpret, Deliberate, Act, Evaluate, Meta-Adapt), the orchestrator calls `health_monitor.check_before_call()`. On `CircuitOpen`, the orchestrator pauses the cycle, emits `CircuitBreakerOpen` event, and retries after `cooldown`.

### 5.4 Success Criteria

- [ ] `Module` trait compiles and is implemented by the example-tariff crate
- [ ] `CognitiveEvent` enum serializes to/from JSON with round-trip fidelity
- [ ] Circuit breaker transitions Closed → Open → HalfOpen → Closed under simulated failures
- [ ] All typed events publishable/consumable via NATS JetStream

---

## 6. Phase 2 — Perceive (Web Perception Layer)

**Duration:** Week 2–4  
**Goal:** Build the sector-aware data collection layer on top of IronClaw's web infrastructure, with tri-dimensional relevance scoring and hybrid-search knowledge ingestion.

### 6.1 DataCollector on IronClaw's Web Tools

IronClaw's `web_fetch` and `web_search` tools are implemented as first-class Rust structs using `reqwest` and external APIs. Project E's `DataCollector` orchestrates multiple strategies (Firecrawl, Tavily, scraper NATS events). The integration wraps IronClaw's tools inside the collector.

**File:** `crates/perception/src/collector.rs`

```rust
pub struct DataCollector {
    firecrawl: FirecrawlClient,        // IronClaw's web_fetch under the hood
    tavily: Option<TavilyClient>,      // IronClaw's web_search under the hood
    config: PerceptionConfig,
    event_bus: Arc<EventBus>,
}

impl DataCollector {
    pub async fn collect_cycle(&self, cycle_id: Uuid) -> Result<Vec<ScoredDataItem>> {
        // 1. Firecrawl seed URLs (primary)
        // 2. Tavily freshness queries (secondary)
        // 3. Consume NATS events from Python scrapers (tertiary)
        // 4. Score each item via RelevanceScorer
        // 5. Persist to scraped_data table
        // 6. Ingest high-relevance items into knowledge base (via hybrid indexer)
        // 7. Emit PerceptionComplete event
    }
}
```

### 6.2 Tri-Dimensional RelevanceScorer

The existing TF-IDF scorer in Project E uses a single score. Upgrade it to the three-dimension model from the cognitive framework.

**File:** `crates/perception/src/relevance.rs`

```rust
pub struct RelevanceScorer {
    keywords: HashMap<String, f32>,  // from TOML perception.keywords
    existing_topics: HashSet<String>, // populated from module registry at cycle start
    weights: RelevanceWeights,        // novelty_weight, coverage_weight, magnitude_weight
}

impl RelevanceScorer {
    /// Score = novelty_weight * novelty(item)
    ///       + coverage_weight * (1 - coverage(item))
    ///       + magnitude_weight * magnitude(item)
    ///
    /// novelty:   TF-IDF score of item tokens vs. existing knowledge base corpus
    /// coverage:  cosine similarity to nearest active module's description (IronClaw pgvector)
    ///            → low similarity = high coverage gap = important
    /// magnitude: keyword density weighted by configured urgency (e.g., "lei" > "nota")
    pub async fn score(&self, item: &RawDataItem) -> f32 { ... }
}
```

**Key difference from existing implementation:**
- `coverage` dimension now uses IronClaw's pgvector similarity against the module registry embeddings — items furthest from existing capabilities score highest
- All three dimension weights are TOML parameters adjustable by meta-adaptation

### 6.3 Hybrid-Search Knowledge Ingestion

After scoring, high-relevance items (above `perception.min_relevance_score`) are chunked and ingested into the knowledge base using IronClaw's hybrid FTS + pgvector approach.

**File:** `crates/knowledge-base/src/ingestion.rs`

```rust
pub struct KnowledgeIngester {
    db: Arc<PgPool>,
    embedder: Arc<EmbedderClient>,  // jina-embeddings-v3 via OpenRouter
}

impl KnowledgeIngester {
    pub async fn ingest_scraped_item(&self, item: &ScoredDataItem) -> Result<()> {
        let chunks = self.chunk(item.content(), CHUNK_SIZE, OVERLAP);
        for chunk in chunks {
            let embedding = self.embedder.embed(&chunk).await?;
            // Insert into knowledge_embeddings (pgvector)
            // Also update tsvector column for FTS
            sqlx::query!(
                "INSERT INTO knowledge_embeddings
                 (content, embedding, search_vector, source_type, relevance_score)
                 VALUES ($1, $2, to_tsvector('portuguese', $1), $3, $4)",
                chunk, embedding.as_slice(), item.source_type, item.relevance_score
            ).execute(&self.db).await?;
        }
        Ok(())
    }
}
```

### 6.4 Python Scraper Bridge

The Python scrapers (ERSE, OMIE) publish events to NATS independently. The DataCollector already consumes these. No changes needed — but make the NATS topic schema explicit:

```
project_e.perception.scraper.erse   → ERSE regulatory data
project_e.perception.scraper.omie   → OMIE market prices
```

The DataCollector subscribes to `project_e.perception.scraper.*` and processes all scraper events in the same pipeline as Firecrawl/Tavily items.

### 6.5 Success Criteria

- [ ] Full perception cycle collects data, scores it, and persists to `scraped_data` table
- [ ] RelevanceScorer produces distinct scores across novelty/coverage/magnitude dimensions
- [ ] High-relevance items appear in `knowledge_embeddings` with both vector and `tsvector`
- [ ] Python scraper events are consumed and scored alongside Firecrawl items
- [ ] `PerceptionComplete` event emitted to NATS with correct cycle_id

---

## 7. Phase 3 — Interpret (Analysis Engine)

**Duration:** Week 3–5  
**Goal:** Implement the LLM-as-Judge gap detection with BM25 module discovery and enriched GapReport typing.

### 7.1 BM25 Index for Module Discovery

With dozens or hundreds of active modules, passing the full registry to the LLM context is wasteful. Port IronClaw's BM25 deferred tool loading pattern to module discovery.

**File:** `crates/analysis/src/bm25.rs`

```rust
/// Lightweight BM25 index over module metadata.
/// Built at the start of each Interpret phase from the live registry.
/// Allows the LLM-as-Judge to find relevant modules by query
/// without receiving the full catalog in its context window.
pub struct ModuleBm25Index {
    /// (module_id, tokens, doc_length)
    docs: Vec<(Uuid, Vec<String>, usize)>,
    idf_cache: HashMap<String, f32>,
    k1: f32,    // typically 1.5
    b: f32,     // typically 0.75
}

impl ModuleBm25Index {
    /// Build index from current registry state.
    pub fn build(modules: &[ModuleMetadata]) -> Self {
        // Tokenize name + description + tags by whitespace, underscores, hyphens
        // Compute IDF over corpus
    }

    /// Return top-k module IDs most relevant to a query string.
    /// Used by the Analyzer to pre-filter before embedding cosine search.
    pub fn query(&self, q: &str, k: usize) -> Vec<(Uuid, f32)> { ... }
}
```

**Two-stage retrieval pipeline for Phase 2:**
1. BM25 query → top-20 module candidates (fast, zero API cost)
2. IronClaw pgvector cosine similarity → rerank to top-5 (semantic, 1 embedding call)
3. These 5 modules form the "existing capability context" for the LLM-as-Judge

This replaces the current pattern of passing the entire module list to the LLM.

### 7.2 Enriched GapReport

The current `GapReport` struct lacks the Cynefin/Wardley classification required by the deliberation engine. Add these fields:

**File:** `crates/analysis/src/gap_report.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapReport {
    pub id: Uuid,
    pub cycle_id: Uuid,
    pub classification: GapClassification,   // GAP | CONFLICT | CONFIRMATION
    pub confidence: f32,                     // 0.0-1.0; only gaps above threshold proceed

    // NEW: Cynefin domain determines forge strategy in Phase 3
    pub cynefin_domain: CynefinDomain,

    // NEW: Wardley evolution stage determines fault tolerance
    pub wardley_stage: WardleyStage,

    pub priority: Priority,                  // Critical | High | Medium | Low
    pub suggested_action: LifecycleAction,   // Create | Update | Merge | Split | Deprecate
    pub justification: String,
    pub affected_modules: Vec<Uuid>,
    pub evidence: Vec<String>,               // source URLs / data items

    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CynefinDomain {
    Simple,      // → apply best-practice template, high confidence, low risk
    Complicated, // → generate 2-3 variants, test, select best
    Complex,     // → generate experimental module with "experimental" flag, monitor closely
    Chaotic,     // → rapid prototype, temporary module, iterate fast
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WardleyStage {
    Genesis,     // High fault tolerance; many retry attempts allowed
    Custom,      // Medium tolerance; evaluate generalizability
    Product,     // Low tolerance; incremental changes only
    Commodity,   // Candidate for simplification or merger
}
```

**LLM prompt for gap analysis** must be updated to elicit Cynefin and Wardley fields as part of the JSON output schema. The existing `gap-analysis` SKILL.md already covers the classification; add a section for domain/stage guidance.

### 7.3 Analysis Engine Wire-Up

**File:** `crates/analysis/src/engine.rs`

```rust
pub struct AnalysisEngine {
    llm: Arc<LlmProvider>,              // IronClaw's typed provider
    knowledge_base: Arc<KnowledgeBase>,
    registry: Arc<ModuleRegistry>,
    bm25_index: RwLock<ModuleBm25Index>, // rebuilt each cycle
    skills: Arc<SkillCatalog>,
    event_bus: Arc<EventBus>,
}

impl AnalysisEngine {
    pub async fn analyze(&self, cycle_id: Uuid, scored_data: &[ScoredDataItem])
        -> Result<Vec<GapReport>>
    {
        // 1. Rebuild BM25 index from current registry
        // 2. For each high-relevance data item:
        //    a. BM25 query → top-20 module candidates
        //    b. pgvector cosine rerank → top-5
        //    c. RAG: retrieve similar past gap reports from knowledge base
        //    d. Activate gap-analysis skill (Tier 2)
        //    e. LLM-as-Judge: structured JSON output including cynefin + wardley
        //    f. Filter by confidence threshold
        // 3. Persist gap reports
        // 4. Ingest gap reports into knowledge base
        // 5. Emit GapReportReady event
    }
}
```

### 7.4 Success Criteria

- [ ] BM25 index builds from module registry in < 100ms for 100 modules
- [ ] Two-stage retrieval (BM25 → pgvector) reduces LLM context by >50% vs. full catalog
- [ ] GapReport JSON includes `cynefin_domain` and `wardley_stage` fields
- [ ] `GapReportReady` event emitted with gap/conflict/confirmation counts
- [ ] Malformed LLM output handled: code fences stripped, partial JSON recovered

---

## 8. Phase 4 — Deliberate (Metacognition Layer)

**Duration:** Week 4–6  
**Goal:** Build the self-model, deliberation engine with operating modes, and glob-pattern tool policy engine.

### 8.1 SelfModelBuilder with Five Health Dimensions

Wire IronClaw's heartbeat data (module health states from self-repair) into the SelfModelBuilder. This means Phase 5 data feeds directly into Phase 3 decision-making.

**File:** `crates/metacognition/src/self_model.rs`

```rust
#[derive(Debug, Clone)]
pub struct SystemSelfModel {
    pub coverage_score: f32,    // fraction of identified gaps with active modules
    pub stability_score: f32,   // module success rate over sliding window
    pub efficiency_score: f32,  // forge success rate: successful / total attempts
    pub utilization_score: f32, // fraction of active modules receiving NATS events
    pub freshness_signal: f32,  // recency of data feeding active modules (0=stale, 1=fresh)
    pub operating_mode: OperatingMode,
    pub reflection_window: Vec<CycleMetrics>, // last N cycles
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperatingMode {
    Recovery,   // efficiency very low, recent failures high → minimal action
    Cautious,   // mixed signals → selective action, defer uncertain gaps
    Nominal,    // good efficiency, stable system → normal operation
}

impl SelfModelBuilder {
    /// Called at the start of each Deliberation phase.
    /// Reads from: module_state_transitions, test_results, knowledge_embeddings,
    /// event bus utilization counts, and scraped_data timestamps.
    pub async fn build(&self, cycle_id: Uuid) -> Result<SystemSelfModel> {
        // IronClaw heartbeat data feeds utilization_score directly
        // Forge test_results table feeds efficiency_score
        // Module active count vs. known gap count feeds coverage_score
    }
}
```

### 8.2 DeliberationEngine with Operating Modes

**File:** `crates/metacognition/src/deliberation.rs`

```rust
impl DeliberationEngine {
    pub async fn deliberate(
        &self,
        gap_reports: Vec<GapReport>,
        self_model: &SystemSelfModel,
    ) -> Result<ActionPlan> {
        // 1. Filter gaps by confidence threshold (from self_model/config)
        // 2. Cynefin-based strategy assignment:
        //    Simple → CreateFromTemplate
        //    Complicated → CreateWithVariants(n=2)
        //    Complex → CreateExperimental
        //    Chaotic → CreateRapidPrototype
        // 3. Wardley-based fault tolerance:
        //    Genesis → max_retries=5, experimental=true
        //    Product → max_retries=2, experimental=false
        // 4. Operating mode gates:
        //    Recovery → skip all; emit ActionDeferred for each gap
        //    Cautious → only Simple/Confirmed gaps; max 2 actions
        //    Nominal → full action plan up to limits.max_actions_per_cycle
        // 5. Cost estimation using ModelCapabilities.cost_per_million_tokens
        // 6. Emit ActionPlanReady event
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionPlan {
    pub id: Uuid,
    pub cycle_id: Uuid,
    pub actions: Vec<PlannedAction>,     // ordered by priority
    pub deferred: Vec<DeferredGap>,      // gaps not acted on this cycle + reason
    pub estimated_llm_cost: f32,         // in USD
    pub max_concurrent: usize,           // from deliberation.max_concurrent_actions
}
```

### 8.3 Glob-Pattern Tool Policy Engine

Port the Turnstone tool policy engine (as analyzed) into Project E's deliberation layer. This replaces the single `deliberation.risk_threshold` float with granular rules.

**File:** `crates/metacognition/src/policy.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub pattern: String,    // glob, e.g. "module.create.*"
    pub action: PolicyAction,
    pub condition: Option<PolicyCondition>, // optional confidence/risk condition
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    Allow,  // proceed autonomously
    Ask,    // require human confirmation (pause cycle, emit event)
    Deny,   // never allowed without explicit config change
}

pub struct PolicyEngine {
    rules: Vec<PolicyRule>,  // loaded from TOML [deliberation.policies]
}

impl PolicyEngine {
    /// Evaluate a planned action against policy rules.
    /// First-match-wins. Falls back to Allow if no rule matches.
    pub fn evaluate(&self, action: &PlannedAction) -> PolicyAction { ... }

    /// Batch evaluation for the full ActionPlan.
    pub fn filter_plan(&self, plan: &mut ActionPlan) {
        plan.actions.retain(|a| {
            match self.evaluate(a) {
                PolicyAction::Allow => true,
                PolicyAction::Deny  => { plan.deferred.push(DeferredGap::from(a)); false }
                PolicyAction::Ask   => { /* emit event, await external signal */ false }
            }
        });
    }
}
```

**Default policy rules in `config/default.toml`:**

```toml
[deliberation.policies]
rules = [
    { pattern = "module.create.simple",    action = "allow" },
    { pattern = "module.create.complex",   action = "allow",
      condition = { min_confidence = 0.8 } },
    { pattern = "module.deprecate.*",      action = "ask" },
    { pattern = "module.remove.*",         action = "deny" },
    { pattern = "module.merge.*",          action = "ask" },
]
```

### 8.4 Success Criteria

- [ ] SelfModelBuilder computes all five health dimensions from real database data
- [ ] Operating mode transitions tested: inject simulated failures → system enters Recovery
- [ ] ActionPlan respects operating mode gate (Recovery → empty plan)
- [ ] PolicyEngine correctly filters: `module.remove.*` → Deny regardless of confidence
- [ ] `ActionPlanReady` event emitted with correct action count

---

## 9. Phase 5 — Act (The Forge Pipeline)

**Duration:** Week 5–8  
**Goal:** Build the six-gate forge pipeline using IronClaw's Wasmtime sandbox for the build environment and persistent shell session for sequential cargo commands.

### 9.1 Wasmtime Sandbox for the Forge Build Environment

IronClaw uses Wasmtime to sandbox LLM-generated WASM modules. In Project E, the published modules are native Rust binaries — but the forge *compilation environment* benefits from sandbox isolation. The Wasmtime sandbox wraps the cargo commands so generated code cannot make unauthorized network calls or write outside the forge workspace.

**File:** `crates/forge/src/pipeline.rs`

```rust
pub struct ForgePipeline {
    sandbox: Arc<WasmSandbox>,          // IronClaw Wasmtime sandbox
    llm: Arc<LlmProvider>,
    skills: Arc<SkillCatalog>,
    event_bus: Arc<EventBus>,
    config: ForgeConfig,
}

impl ForgePipeline {
    pub async fn forge_module(
        &self,
        gap: &GapReport,
        action: &PlannedAction,
        cycle_id: Uuid,
    ) -> Result<ForgedModule> {
        // 1. Pre-generation: requirements doc → design doc (LLM)
        // 2. Code generation with skill injection
        // 3. Write to forge workspace
        // 4. Run six gates (all inside Wasmtime sandbox)
        // 5. On gate failure: retry loop with best-candidate tracking
        // 6. On success: register module, emit ModulePublished
    }
}
```

**`capabilities.json` sidecar** — generated per module by the forge, constraining what the module can do at runtime:

```rust
// crates/forge/src/capabilities.rs
pub fn generate_capabilities_manifest(gap: &GapReport) -> Capabilities {
    Capabilities {
        allow_http: gap.requires_web_access(),
        allow_domains: gap.required_domains(),
        allow_secrets: gap.required_secrets(),
        max_http_requests: 50,
        max_memory_bytes: 128 * 1024 * 1024,  // 128MB
        timeout_secs: 30,
        network_policy: if gap.requires_web_access() {
            NetworkPolicy::AllowListed
        } else {
            NetworkPolicy::DenyAll
        },
    }
}
```

### 9.2 Six Validation Gates in Order

**Gate 1: Unsafe Check (AST-based)**
```rust
// crates/forge/src/gates/unsafe_check.rs
// Parse with `syn`, walk the AST, reject any unsafe block/fn/impl
```

**Gate 2: Forbidden Pattern Check with Auto-Sanitize**
```rust
// crates/forge/src/gates/forbidden.rs
// Detect: unwrap(), panic!(), todo!(), unimplemented!()
// Auto-sanitize: replace unwrap() with ? where context allows
// Reject if sanitization is insufficient
```

**Gate 3: Compilation via Persistent Shell Session**

Adopt IronClaw's persistent shell session pattern (from OpenCode analysis) for sequential cargo commands. Eliminates subprocess-per-command overhead:

```rust
// crates/forge/src/gates/compile.rs
pub struct ForgeShellSession {
    session: Arc<PersistentShell>,  // IronClaw pattern: persistent stdin/stdout
    workdir: PathBuf,
}

impl ForgeShellSession {
    pub async fn compile(&self, module_path: &Path) -> Result<CompileResult> {
        // Session persists across: cd → cargo fmt → cargo build --release
        // ~2-3 second saving per module vs. new subprocess each time
        self.session.run("cargo build --release", COMPILE_TIMEOUT).await
    }
}
```

**Gate 4: Clippy**
```rust
self.session.run("cargo clippy -- -D warnings", CLIPPY_TIMEOUT).await
```

**Gate 5: Security Audit**
```rust
self.session.run("cargo audit", AUDIT_TIMEOUT).await
```

**Gate 6: Tests**
```rust
self.session.run("cargo test", TEST_TIMEOUT).await
```

### 9.3 Retry Loop with Best-Candidate Tracking

```rust
// crates/forge/src/retry.rs
pub struct RetryState {
    pub attempt: u8,
    pub max_attempts: u8,           // from config, adjusted by WardleyStage
    pub best_candidate: Option<String>,  // most complete code seen so far
    pub error_history: Vec<GateError>,   // fed back to LLM on retry
    pub escalated: bool,            // true when switched to model_for_retry
}

impl RetryState {
    /// On each attempt: if this code is more complete than best_candidate, update it.
    /// "More complete" = compiles (gate 3 passes) but fails later gates
    pub fn update_best(&mut self, code: &str, gate_reached: GateName) {
        let current_progress = gate_reached.ordinal();
        let best_progress = self.best_gate().ordinal();
        if current_progress >= best_progress {
            self.best_candidate = Some(code.to_owned());
        }
    }

    /// After max_attempts, return best_candidate (not latest) for RejectionReport
    pub fn best_for_report(&self) -> Option<&str> {
        self.best_candidate.as_deref()
    }
}
```

### 9.4 Skill-Enhanced Code Generation

The skills system activates relevant SKILL.md files as part of the generation prompt. This is already implemented in Project E — connect it to the IronClaw LLM provider:

```rust
// crates/forge/src/generator.rs
pub async fn generate_module(
    &self,
    gap: &GapReport,
    requirements_doc: &str,
    design_doc: &str,
    retry_state: &RetryState,
) -> Result<String> {
    // Always activate:
    //   - rust-module-generation
    //   - module-lifecycle
    //   - nats-event-integration (if event bus configured)
    // Conditionally activate:
    //   - web-scraping (if gap relates to data collection)
    //   - domain-specific skills (e.g., energy-tariff)

    let skill_context = self.skills.activate_for_generation(gap).await?;

    // Build token-budgeted prompt:
    // system: SYSTEM_SOUL.md + agent profile
    // user:   Tier1 skill catalog + requirements + design + skill instructions
    //         + (on retry) error history from retry_state.error_history
    //         + (on retry) best_candidate for reference

    // Use IronClaw LlmProvider with ModelCapabilities routing:
    // → model_for_generation on attempt 1-2
    // → model_for_retry (escalated) on attempt 3+
}
```

### 9.5 Success Criteria

- [ ] Full pipeline runs: gap → requirements → design → code → gates → active module
- [ ] Gate failures feed error message back to LLM for retry
- [ ] Persistent shell session reused across all six gates for one module (no subprocess-per-gate)
- [ ] `capabilities.json` generated alongside each module and checked on registration
- [ ] `ModulePublished` event emitted; module appears in registry at ACTIVE state
- [ ] Retry escalates to `model_for_retry` after two failures
- [ ] Model escalation recorded as `ParameterAdjusted` event with justification

---

## 10. Phase 6 — Evaluate (Evaluation Engine)

**Duration:** Week 7–10  
**Goal:** Build the Evaluation Engine by adapting IronClaw's self-repair mechanism into a full post-cycle assessment system with homeostatic regulation.

### 10.1 IronClaw Self-Repair → Project E Evaluation Engine

IronClaw's `src/agent/repair.rs` monitors running tools for stuck states and triggers recovery. This is the structural MVP for Project E's missing Evaluation Engine.

**Mapping:**

| IronClaw Self-Repair | Project E Evaluation Engine |
|---|---|
| `broken_tools` table | `module_health` time series |
| `stuck_threshold` (tool inactive > 300s) | Module with 0 NATS events > `evaluation.monitoring_window` |
| `SELF_REPAIR_MAX_ATTEMPTS = 3` | `max_forge_retries` (config param) |
| Tool recovery → re-initialize | Module degraded → trigger homeostatic action |
| Heartbeat probe interval | `evaluation.monitoring_window` TOML param |

**File:** `crates/metacognition/src/evaluation.rs`

```rust
pub struct EvaluationEngine {
    db: Arc<PgPool>,
    registry: Arc<ModuleRegistry>,
    event_bus: Arc<EventBus>,
    config: EvaluationConfig,
}

impl EvaluationEngine {
    /// Called at the start of Phase 5 in each cycle.
    pub async fn evaluate_cycle(
        &self,
        cycle_id: Uuid,
        published_modules: &[Uuid],
        rejected_count: usize,
        skill_activations: &[SkillActivation],
    ) -> Result<CycleEvaluation> {

        // 1. Module quality: did published modules address their gaps?
        //    → Join module_id to gap_report; check gap is no longer flagged
        //    → Compute coverage delta: coverage_after - coverage_before

        // 2. Homeostatic regulation (IronClaw eviction pattern):
        //    → Prune scraped_data older than freshness threshold
        //    → Archive addressed gap reports
        //    → Prune short-lived knowledge embeddings
        //    → Trim failed generation logs
        //    → Detect redundant modules (pgvector similarity > 0.95)
        //    → Auto-deprecate low-health modules (test failure rate > threshold)
        //    → Remove long-deprecated modules (deprecated > 30 days with 0 utilization)

        // 3. Skill effectiveness: which skills correlated with successful generation?
        //    → skill_activations → join to test_results → compute per-skill success rate

        // 4. Build CycleEvaluation struct with all metrics
        // 5. Emit EvaluationComplete event
    }
}
```

### 10.2 Five Homeostatic Health Indicators

Wire the five health indicators (from the Framework Cognitivo spec) to real database queries. These feed the SelfModelBuilder in Phase 3 of the *next* cycle.

**File:** `crates/metacognition/src/health.rs`

```rust
pub struct HomeostaticIndicators {
    pub coverage: f32,     // active_modules / identified_needs (from gap_reports)
    pub stability: f32,    // modules_no_regression / total_active (test_results)
    pub efficiency: f32,   // successful_generations / total_attempts (generation_history)
    pub utilization: f32,  // modules_with_events / total_active (NATS event counts per module)
    pub freshness: f32,    // 1 - avg_data_age_days / max_acceptable_age
}

impl HomeostaticIndicators {
    pub async fn compute(db: &PgPool, config: &HealthConfig) -> Result<Self> { ... }

    pub fn all_above_thresholds(&self, config: &HealthConfig) -> bool {
        self.coverage    >= config.min_coverage    &&
        self.stability   >= config.min_stability   &&
        self.efficiency  >= config.min_efficiency  &&
        self.utilization >= config.min_utilization &&
        self.freshness   >= config.min_freshness
    }
}
```

### 10.3 Redundancy Detection with pgvector

When two modules have embedding similarity > 0.95, the weaker one (lower health score) is auto-deprecated.

```rust
// crates/metacognition/src/evaluation.rs
async fn detect_redundant_modules(&self) -> Result<Vec<RedundancyPair>> {
    // SQL: self-join on knowledge_embeddings WHERE source_type = 'module_source'
    // ORDER BY cosine_distance(e1.embedding, e2.embedding) ASC
    // Filter cosine_similarity > 0.95
    // For each pair, flag the lower-health module for deprecation
    let pairs = sqlx::query_as!(
        RedundancyPair,
        "SELECT a.module_id AS module_a, b.module_id AS module_b,
                1 - (a.embedding <=> b.embedding) AS similarity
         FROM knowledge_embeddings a
         JOIN knowledge_embeddings b ON a.module_id < b.module_id
         WHERE a.source_type = 'module_source'
           AND (1 - (a.embedding <=> b.embedding)) > 0.95"
    ).fetch_all(&self.db).await?;
    Ok(pairs)
}
```

### 10.4 Success Criteria

- [ ] `CycleEvaluation` struct produced at end of each cycle with all metrics populated
- [ ] Homeostatic regulation removes stale data, old gap reports, and duplicate embeddings
- [ ] Redundancy detection flags pairs of similar modules correctly
- [ ] Auto-deprecation fires for low-health modules below threshold
- [ ] `EvaluationComplete` event emitted with efficiency and skill scores
- [ ] Health indicators fed back to SelfModelBuilder for subsequent cycle

---

## 11. Phase 7 — Meta-Adapt (Self-Improvement Loop)

**Duration:** Week 9–12  
**Goal:** Close the feedback loop by rewriting the system's own decision parameters based on observed performance trends.

### 11.1 Adaptive TOML Overlay

The meta-adaptation engine reads the evaluation history and writes adjusted thresholds to `config/runtime/<instance>.adaptive.toml`. These are loaded at the *next* cycle's start, making learned behaviors persistent.

**File:** `crates/metacognition/src/meta_adapt.rs`

```rust
pub struct MetaAdaptationEngine {
    db: Arc<PgPool>,
    config: MetacognitionConfig,
}

impl MetaAdaptationEngine {
    /// Called once per `meta.adaptation_frequency_cycles` (default every 5 cycles).
    pub async fn adapt(&self, instance: &str, evaluation_history: &[CycleEvaluation])
        -> Result<AdaptiveConfig>
    {
        // Only adapt if enough history (>= reflection_window cycles)
        let window = evaluation_history
            .iter().rev().take(self.config.reflection_window).collect::<Vec<_>>();

        // Bounded adjustments (max ±adjustment_max_step per cycle = 10%)
        let adjusted = AdaptiveConfig {
            metacognition: self.adapt_metacognition_thresholds(&window),
            perception: self.adapt_perception_weights(&window),
            forge: self.adapt_forge_params(&window),
        };

        // Write to config/runtime/<instance>.adaptive.toml
        let path = format!("config/runtime/{}.adaptive.toml", instance);
        std::fs::write(&path, toml::to_string_pretty(&adjusted)?)?;

        // Append cycle checkpoint to MEMORY.md
        self.append_memory_checkpoint(instance, &adjusted).await?;

        // Generate constitutional lesson if a pattern repeated 3+ cycles
        if let Some(lesson) = self.extract_lesson(&window) {
            self.append_soul_overlay(instance, &lesson).await?;
            self.event_bus.publish(ConstitutionalLesson { lesson }).await?;
        }

        Ok(adjusted)
    }

    fn adapt_metacognition_thresholds(&self, window: &[&CycleEvaluation]) -> MetacognitionParams {
        // If forge efficiency declining → tighten min_decision_confidence
        // If forge efficiency improving → loosen min_decision_confidence
        // If system in Recovery for 3+ cycles → reduce max_actions_per_cycle
        // All adjustments bounded by ±adjustment_max_step
    }
}
```

### 11.2 Constitutional Evolution

When a failure pattern repeats for 3+ cycles, the meta-adaptation engine encodes it as a behavioral lesson appended to `SYSTEM_SOUL.auto.md`:

```rust
async fn append_soul_overlay(&self, instance: &str, lesson: &str) -> Result<()> {
    let path = format!(".project_e/homeostasis/{}/SYSTEM_SOUL.auto.md", instance);
    let entry = format!(
        "\n\n---\n<!-- Auto-generated overlay: {} -->\n{}\n",
        Utc::now().format("%Y-%m-%d"),
        lesson
    );
    // Append (never overwrite original SYSTEM_SOUL.md)
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(entry.as_bytes())?;
    Ok(())
}
```

### 11.3 ModelCapabilities-Aware Phase Routing

Use IronClaw's `ModelCapabilities` struct to route each cognitive phase to the optimal model tier, and estimate cost in the ActionPlan:

```rust
// crates/core/src/llm_health.rs (extends IronClaw ModelCapabilities)
pub struct PhaseModelRouter {
    capabilities: HashMap<String, ModelCapabilities>,
    routing: PhaseRoutingConfig,
}

impl PhaseModelRouter {
    pub fn model_for_phase(&self, phase: CognitivePhase) -> &str {
        match phase {
            Perception | Interpretation | Evaluation | MetaAdaptation =>
                &self.routing.analysis_model,   // cheaper, fast
            Deliberation =>
                &self.routing.analysis_model,   // reasoning but not code gen
            Action =>
                &self.routing.generation_model, // most capable, code gen
        }
    }

    pub fn estimate_cost(&self, phase: CognitivePhase, input_tokens: usize) -> f32 {
        let model = self.model_for_phase(phase);
        let caps = &self.capabilities[model];
        (input_tokens as f32 / 1_000_000.0) * caps.cost_per_million_input
    }
}
```

### 11.4 Success Criteria

- [ ] `config/runtime/energy-pt.adaptive.toml` written after `adaptation_frequency_cycles` cycles
- [ ] Adjusted thresholds loaded at next cycle start and affect SelfModelBuilder
- [ ] Constitutional lesson extracted and appended to `SYSTEM_SOUL.auto.md` on repeated failure
- [ ] `MEMORY.md` checkpoint appended each adaptation cycle
- [ ] Cost estimation in ActionPlan uses real ModelCapabilities data
- [ ] `ParameterAdjusted` event emitted for each threshold change with before/after values

---

## 12. Phase 8 — Orchestrate (Daemon and Cycle Loop)

**Duration:** Week 10–14  
**Goal:** Wire all six phases into a continuous autonomous cycle using IronClaw's job scheduler as the CycleOrchestrator, with the event bus as the control plane.

### 12.1 CycleOrchestrator Using IronClaw's Job Scheduler

IronClaw's job scheduler manages concurrent jobs with a state machine (Pending → InProgress → Completed → Failed → Stuck). This maps directly to Project E's cognitive cycle phases.

**File:** `crates/orchestrator/src/cycle.rs`

```rust
pub struct CycleOrchestrator {
    scheduler: Arc<JobScheduler>,       // IronClaw job scheduler
    health_monitor: Arc<LlmHealthMonitor>,
    perception: Arc<DataCollector>,
    analysis: Arc<AnalysisEngine>,
    metacognition: Arc<MetacognitionEngine>,
    forge: Arc<ForgePipeline>,
    evaluation: Arc<EvaluationEngine>,
    meta_adapt: Arc<MetaAdaptationEngine>,
    event_bus: Arc<EventBus>,
    config: Arc<Config>,
}

impl CycleOrchestrator {
    /// Single autonomous cycle execution.
    pub async fn run_cycle(&self) -> Result<CycleResult> {
        let cycle_id = Uuid::new_v4();
        self.event_bus.emit(CognitiveEvent::CycleStarted { cycle_id, instance: self.config.instance.name.clone() }).await?;

        // Circuit breaker check before any LLM phase
        self.health_monitor.check_before_call().await?;

        // Phase 1: Perceive
        let scored_data = self.perception.collect_cycle(cycle_id).await?;

        // Phase 2: Interpret (LLM)
        self.health_monitor.check_before_call().await?;
        let gap_reports = self.analysis.analyze(cycle_id, &scored_data).await?;

        // Phase 3: Deliberate (LLM)
        self.health_monitor.check_before_call().await?;
        let self_model = self.metacognition.build_self_model(cycle_id).await?;
        let action_plan = self.metacognition.deliberate(gap_reports, &self_model).await?;

        // Phase 4: Act (LLM — parallel by default, sequential in Cautious/Recovery)
        self.health_monitor.check_before_call().await?;
        let forge_results = self.forge_from_plan(cycle_id, &action_plan).await?;

        // Phase 5: Evaluate
        let evaluation = self.evaluation.evaluate_cycle(
            cycle_id, &forge_results.published, forge_results.rejected_count,
            &forge_results.skill_activations,
        ).await?;

        // Phase 6: Meta-Adapt (every N cycles)
        if self.should_adapt() {
            self.meta_adapt.adapt(&self.config.instance.name, &self.evaluation_history).await?;
        }

        self.event_bus.emit(CognitiveEvent::CycleClosed {
            cycle_id,
            duration_secs: cycle_start.elapsed().as_secs(),
        }).await?;

        Ok(CycleResult { cycle_id, evaluation })
    }

    /// Daemon loop: run cycles at configured crawl_frequency with IronClaw scheduler.
    pub async fn run_daemon(&self) -> Result<()> {
        loop {
            // IronClaw scheduler: check HEARTBEAT.md trigger conditions
            if self.should_trigger_cycle().await? {
                match self.run_cycle().await {
                    Ok(result) => tracing::info!("Cycle {} complete", result.cycle_id),
                    Err(e) => {
                        tracing::error!("Cycle failed: {}", e);
                        self.supervisor.observe_failure(e).await?;
                        // IronClaw scheduler: backoff before retry
                        tokio::time::sleep(FAILURE_BACKOFF).await;
                    }
                }
            }
            tokio::time::sleep(SCHEDULER_CHECK_INTERVAL).await;
        }
    }

    /// Context compaction (IronClaw pattern) when context window exceeded.
    async fn compact_context(&self, agent: &str) -> Result<()> {
        // Summarize recent cycle history to daily/<date>.md
        // Truncate in-memory context
        // Move long-term facts to knowledge base
    }
}
```

### 12.2 Supervisor Agent

The Supervisor monitors for seven anomaly patterns and applies interventions:

**File:** `crates/orchestrator/src/supervisor.rs`

```rust
pub struct Supervisor {
    anomaly_counters: HashMap<AnomalyPattern, u32>,
    intervention_history: Vec<Intervention>,
    config: SupervisorConfig,
    llm: Arc<LlmProvider>,
    event_bus: Arc<EventBus>,
}

impl Supervisor {
    pub async fn observe_failure(&mut self, error: &CycleError) -> Result<()> {
        let pattern = self.classify_anomaly(error);
        *self.anomaly_counters.entry(pattern).or_insert(0) += 1;

        if self.anomaly_counters[&pattern] >= self.config.intervention_threshold {
            let intervention = self.choose_intervention(pattern).await?;
            self.apply_intervention(&intervention).await?;
            self.event_bus.emit(SupervisorIntervention { pattern, intervention }).await?;
            self.reset_counter(pattern);
        }
        Ok(())
    }

    fn choose_intervention(&self, pattern: AnomalyPattern) -> InterventionType {
        match pattern {
            AnomalyPattern::RepeatedCompilationErrors =>
                InterventionType::PromptInjection("Focus on simpler module structure".into()),
            AnomalyPattern::EfficiencyCollapse =>
                InterventionType::ModelEscalation { to: self.config.escalation_model.clone() },
            AnomalyPattern::RecoveryLoop =>
                InterventionType::TightenConfidenceThreshold(0.05),
            AnomalyPattern::ErrorStorm =>
                InterventionType::PauseCycle { duration: Duration::from_secs(300) },
            // ...
        }
    }
}
```

### 12.3 HEARTBEAT.md Trigger Conditions

The daemon checks HEARTBEAT.md conditions before each cycle decision:

```
Trigger conditions (from HEARTBEAT.md):
  - scraped_data.max_age > sources.crawl_frequency  → stale data, trigger perception
  - gap_reports.unaddressed_count > 0                → pending gaps, run full cycle
  - homeo.coverage < health.min_coverage             → coverage drift, emergency cycle
  - No cycle run in > 2 × crawl_frequency            → scheduled cycle
```

### 12.4 Success Criteria

- [ ] `project-e run` executes a full six-phase cycle end-to-end without manual intervention
- [ ] Daemon loop respects `crawl_frequency` and HEARTBEAT.md trigger conditions
- [ ] Supervisor detects 3 consecutive compilation failures and applies intervention
- [ ] Circuit breaker pauses daemon on provider failure; resumes automatically on recovery
- [ ] Context compaction runs when LLM context window is exceeded
- [ ] `CycleClosed` event emitted at end of each cycle with correct duration

---

## 13. Phase 9 — Interface and Deployment

**Duration:** Week 13–16  
**Goal:** Unify the module registry, update the MCP server, wire the Python scrapers, and finalize the Docker deployment stack.

### 13.1 Registry Unification (Critical Bug Fix)

The architectural audit identified this as the most critical integrity issue: CLI in-memory registry ≠ PostgreSQL ≠ MCP server registry. Fix using IronClaw's single-source-of-truth pattern.

**File:** `crates/registry/src/store.rs`

```rust
pub struct ModuleRegistry {
    db: Arc<PgPool>,
    cache: DashMap<Uuid, ModuleMetadata>,  // in-memory read-through cache
    event_bus: Arc<EventBus>,
}

impl ModuleRegistry {
    /// PostgreSQL is the single source of truth.
    /// Cache is populated on read and invalidated via NATS events.
    pub async fn get(&self, id: Uuid) -> Result<Option<ModuleMetadata>> {
        if let Some(entry) = self.cache.get(&id) {
            return Ok(Some(entry.clone()));
        }
        let row = sqlx::query_as!(ModuleMetadata,
            "SELECT * FROM modules WHERE id = $1", id
        ).fetch_optional(&self.db).await?;
        if let Some(ref m) = row {
            self.cache.insert(id, m.clone());
        }
        Ok(row)
    }

    /// Transition module state; enforces valid transitions.
    pub async fn transition(&self, id: Uuid, new_state: ModuleState, reason: &str)
        -> Result<()>
    {
        // Validate transition is allowed
        // Update modules table
        // Insert into module_state_transitions audit trail
        // Invalidate cache entry
        // Emit registry event on NATS → MCP server receives and updates its view
        self.event_bus.emit(
            CognitiveEvent::ModuleStateChanged { module_id: id, new_state }
        ).await?;
        Ok(())
    }
}
```

**MCP server subscribes to module registry events** at startup, keeping its view consistent without polling:

```rust
// crates/mcp-server/src/main.rs
// At startup, subscribe to project_e.registry.* wildcard
// On ModulePublished: add module tool dynamically
// On ModuleDeprecated: remove module tool
// No more stale view between CLI and MCP
```

### 13.2 Updated MCP Server Tools

Add three new MCP tools reflecting the new cognitive architecture:

| New Tool | Function |
|---|---|
| `project_e_cycle_status` | Real-time status of current/last cycle with phase and health indicators |
| `project_e_gap_reports` | Query current gap reports with Cynefin/Wardley classification |
| `project_e_self_model` | Exposes the current SystemSelfModel (coverage, stability, efficiency, etc.) |

### 13.3 Docker Stack Cleanup

Remove Redis (dead infrastructure):

```yaml
# docker/docker-compose.yml
# REMOVE the redis service block entirely
# REMOVE redis from depends_on lists
# NOTE: Re-add only when Redis Streams becomes a NATS fallback in production
```

Final service set:
```yaml
services:
  nats:        # event bus backbone
  postgres:    # primary storage + pgvector
  searxng:     # self-hosted web search
  core:        # project-e run (CycleOrchestrator daemon)
  mcp-server:  # MCP tool server
  scrapers:    # Python perception producers
```

### 13.4 CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
jobs:
  check:
    - cargo fmt --check
    - cargo clippy --workspace -- -D warnings
    - cargo build --release --workspace

  test:
    services: [nats, postgres]
    - cargo test --workspace

  audit:
    - cargo audit

  integration:
    services: [nats, postgres]
    - run a single full cognitive cycle in test mode
    - assert: module registry not empty after forge
    - assert: evaluation metrics computed
    - assert: no leaked secrets in logs (SecretRedactor validation)
```

### 13.5 Success Criteria

- [ ] Registry: module generated via CLI appears immediately in MCP server (no restart needed)
- [ ] MCP server dynamically adds/removes tools as modules are published/deprecated
- [ ] Redis removed from Docker stack with no compilation errors
- [ ] `docker compose up` starts all six services and full cycle runs in container
- [ ] CI pipeline passes all three jobs (check, test, audit)
- [ ] Integration test: single cycle produces at least one attempted forge

---

## 14. Migration Decision per Existing Project E Crate

| Project E Crate | Action | What Changes |
|---|---|---|
| `project-e-core` | **Extend** | Add `LlmHealthMonitor`, `Module` trait, `capabilities.rs` from IronClaw |
| `project-e-protocol` | **Replace** | Old op/event protocol → new `CognitiveEvent` typed enum |
| `project-e-event-bus` | **Keep** | Add `protocol.rs` with `CognitiveEvent`; wire to all phase outputs |
| `project-e-perception` | **Upgrade** | Add IronClaw hybrid search to knowledge ingestion; tri-dim scorer |
| `project-e-analysis` | **Upgrade** | Add BM25 module discovery; enrich `GapReport` with Cynefin/Wardley |
| `project-e-metacognition` | **Upgrade** | Wire SelfModelBuilder to IronClaw heartbeat data; add policy engine |
| `project-e-forge` | **Upgrade** | Wrap gates in IronClaw Wasmtime sandbox; add persistent shell session; add `capabilities.json` generation |
| `project-e-registry` | **Fix** | Unify dual-state bug; PostgreSQL as single source-of-truth; IronClaw cache-read-through |
| `project-e-knowledge-base` | **Upgrade** | Add FTS `tsvector` column; RRF fusion retrieval (IronClaw hybrid search) |
| `project-e-skills` | **Keep** | Already aligned with IronClaw's extension pattern; minor wiring |
| `project-e-orchestrator` | **Rebuild on IronClaw** | CycleOrchestrator uses IronClaw job scheduler + state machine |
| `project-e-supervisor` | **Keep, wire** | Connect to IronClaw circuit breaker and model escalation |
| `project-e-capabilities` | **Merge into core** | Fold into `project-e-core`; use IronClaw's `ModelCapabilities` |
| `project-e-cli` | **Keep, extend** | Add circuit breaker status command; add health indicator command |
| `project-e-mcp-server` | **Upgrade** | Subscribe to registry events for live sync; add new cognitive tools |
| `project-e-tui` | **Keep** | Wire to new `HomeostaticIndicators` for live health display |
| `project-e-example-tariff` | **Keep** | Reference implementation; validate it passes all forge gates |

---

## 15. Dependency Order and Critical Path

Phases are not fully sequential — some can parallelize. The critical path is:

```
Phase 0 (Foundation)
    │
Phase 1 (Module Trait + Event Bus)
    │
    ├── Phase 2 (Perceive) ──────────────────────────┐
    │                                                 │
    ├── Phase 3 (Interpret) ← depends on Phase 2     │
    │                                                 │
    ├── Phase 4 (Deliberate) ← depends on Phase 3    │
    │                                                 │
    ├── Phase 5 (Act/Forge) ← depends on Phase 4     │
    │                                                 │
    ├── Phase 6 (Evaluate) ← depends on Phase 5 ─────┘
    │
    ├── Phase 7 (Meta-Adapt) ← depends on Phase 6
    │
Phase 8 (Orchestrate) ← depends on ALL phases 2–7
    │
Phase 9 (Interface & Deploy) ← parallelizable from Phase 8
```

**Non-blocking parallelism opportunities:**
- Phase 2 (Perceive) and Phase 1 extensions can be built concurrently
- Phase 9 registry unification can start during Phase 8
- `HEARTBEAT.md`/`SYSTEM_SOUL.md`/constitutional files (Phase 0 subtask) are documentation-only and take hours, not days

**Minimum viable checkpoint (end of week 8):**
At the end of Phase 5 (Act/Forge), you have a system that can: collect data, detect gaps, deliberate, and attempt to forge a module. This is the first "full loop" even without Evaluation and Meta-Adapt. Ship this as an internal milestone.

---

## 16. Success Criteria per Phase

| Phase | Milestone Gate |
|---|---|
| **Phase 0** | `cargo build --workspace` clean; DB migrations run; constitutional files present |
| **Phase 1** | `Module` trait compiles; `CognitiveEvent` round-trips; circuit breaker state machine works |
| **Phase 2** | Full perception cycle: data collected, scored, ingested to knowledge base, event emitted |
| **Phase 3** | GapReport includes Cynefin/Wardley; BM25 index reduces LLM context by >50% |
| **Phase 4** | ActionPlan respects operating mode; policy engine enforces Deny on removal actions |
| **Phase 5** | Module generated, passes all 6 gates, appears at ACTIVE in registry, event emitted |
| **Phase 6** | `CycleEvaluation` produced; homeostatic pruning runs; redundancy detection works |
| **Phase 7** | Adaptive TOML written after 5 cycles; `SYSTEM_SOUL.auto.md` appended on repeated failure |
| **Phase 8** | `project-e run` executes full 6-phase cycle autonomously; daemon loop stable for 24h |
| **Phase 9** | Docker stack starts cleanly; MCP server stays in sync with registry; CI green |

---

## 17. Risk Register

| Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|
| IronClaw Wasmtime version incompatible with cargo build commands for Rust modules | High | Low | Use OS-level Docker sandbox as fallback; Wasmtime is for env isolation only |
| BM25 index degrades with >500 modules (collision, context explosion) | Medium | Medium | Add pgvector cosine as primary at >200 modules; BM25 as pre-filter only below that |
| Adaptive TOML oscillation (parameters swing back and forth each cycle) | Medium | Medium | `adjustment_max_step = 0.1` is already bounded; add momentum term if oscillation detected |
| LLM provider rate limits during parallel forge (multiple gaps in one cycle) | High | High | `limits.max_llm_calls_per_day` is enforced in ActionPlan cost estimation; `max_concurrent_actions` limits parallelism |
| PostgreSQL pgvector index slow on large `knowledge_embeddings` table | Medium | Medium | Add `HNSW` index (`CREATE INDEX ON knowledge_embeddings USING hnsw (embedding vector_cosine_ops)`) in migration |
| Module registry dual-state bug causes test failures before Phase 9 fix | High | High | Apply registry unification (Phase 9, Section 13.1) as the **first** task in Phase 0 if a running Project E codebase exists |
| Constitutional lesson injection into `SYSTEM_SOUL.auto.md` produces harmful behavioral drift | Medium | Low | Lessons are LLM-generated with bounded scope (only addressing the failure pattern); Supervisor reviews lessons before append |
| IronClaw fork diverges from upstream (security patches) | Low | Medium | Monitor IronClaw releases; apply security patches from upstream to infrastructure crates only |
| Python scrapers lose sync with Rust perception layer on NATS restart | Medium | Medium | Scrapers use NATS JetStream (persistent) not Core NATS; messages are buffered and replayed |

---

*This plan is organized so each phase produces a working increment. Phase 5 (the forge) is the minimum viable demonstration of Project E's unique value proposition. Every phase before it is in service of getting to that first autonomous module generation. Every phase after it makes the system genuinely self-improving.*
