//! Cognitive Cycle Runner — fires the six-phase cognitive cycle on a heartbeat.
//!
//! `CognitiveCycleRunner` is an independent tokio task that runs one full
//! cognitive cycle every `heartbeat_interval`. It coordinates all the other
//! bridge components:
//!
//! ```text
//! CognitiveCycleRunner
//!   ├── WorkspaceKnowledgeStore  (perception: workspace search → RawDataItems)
//!   ├── DataCollector            (relevance scoring, PerceptionComplete event)
//!   ├── AnalysisEngine           (gap report filtering, GapReportReady event)
//!   ├── SelfModelBuilder         (operating mode: Nominal / Cautious / Recovery)
//!   ├── DeliberationEngine       (action plan, ActionPlanReady event)
//!   ├── IronclawLlmBridge        (LLM calls with CostGuard + circuit breaker)
//!   └── ForgeSandboxRunner       (six-gate forge pipeline per planned action)
//! ```
//!
//! **Failure policy:** Three consecutive cycle failures → runner stops itself
//! and logs a warning. An IronClaw operator can re-enable by restarting the
//! service with `PROJECT_E_ENABLED=true`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::watch;
use tracing::{error, info, warn};
use uuid::Uuid;

use project_e_analysis::engine::AnalysisEngine;
use project_e_analysis::gap_report::GapReport;
use project_e_core::config::{AnalysisConfig, LimitsConfig, MetacognitionConfig, PerceptionConfig};
use project_e_core::types::{CycleMetrics, OperatingMode};
use project_e_event_bus::EventBus;
use project_e_event_bus::protocol::CognitiveEvent;
use project_e_metacognition::deliberation::{DeliberationEngine, PlannedAction};
use project_e_metacognition::self_model::SelfModelBuilder;
use project_e_orchestrator::cycle::CycleResult;
use project_e_perception::collector::{DataCollector, RawDataItem, ScoredDataItem};

use crate::project_e::forge_runner::ForgeSandboxRunner;
use crate::project_e::knowledge_bridge::WorkspaceKnowledgeStore;
use crate::project_e::llm_bridge::IronclawLlmBridge;

/// Maximum consecutive failures before the runner stops itself.
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// Runs the six-phase cognitive cycle on a timer.
pub struct CognitiveCycleRunner {
    event_bus: Arc<EventBus>,
    knowledge: WorkspaceKnowledgeStore,
    llm: Arc<IronclawLlmBridge>,
    forge: ForgeSandboxRunner,
    heartbeat_interval: Duration,
    instance: String,
    // Project E sub-components.
    perception: DataCollector,
    analysis: AnalysisEngine,
    deliberation: DeliberationEngine,
    self_model_builder: SelfModelBuilder,
    limits: LimitsConfig,
    // Rolling window of past cycle metrics for self-model.
    cycle_history: Vec<CycleMetrics>,
    reflection_window_size: usize,
}

impl CognitiveCycleRunner {
    /// Construct a runner from bridged infrastructure and Project E configs.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_bus: Arc<EventBus>,
        knowledge: WorkspaceKnowledgeStore,
        llm: Arc<IronclawLlmBridge>,
        forge: ForgeSandboxRunner,
        heartbeat_interval: Duration,
        instance: String,
        perception_config: PerceptionConfig,
        analysis_config: AnalysisConfig,
        meta_config: MetacognitionConfig,
        limits: LimitsConfig,
    ) -> Self {
        let perception = DataCollector::new(perception_config, Arc::clone(&event_bus));
        let analysis = AnalysisEngine::new(analysis_config, Arc::clone(&event_bus));
        let deliberation = DeliberationEngine::new(limits.clone());
        let self_model_builder = SelfModelBuilder::new(
            meta_config.recovery_threshold,
            meta_config.cautious_threshold,
        );
        let reflection_window_size = meta_config.reflection_window;

        Self {
            event_bus,
            knowledge,
            llm,
            forge,
            heartbeat_interval,
            instance,
            perception,
            analysis,
            deliberation,
            self_model_builder,
            limits,
            cycle_history: Vec::new(),
            reflection_window_size,
        }
    }

    /// Spawn the runner as a background tokio task.
    ///
    /// Returns a `JoinHandle` — await it on shutdown.
    pub fn spawn(mut self, mut shutdown: watch::Receiver<bool>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                instance = %self.instance,
                interval_secs = self.heartbeat_interval.as_secs(),
                "project_e: cognitive cycle runner starting"
            );

            let mut consecutive_failures: u32 = 0;

            loop {
                tokio::select! {
                    biased;
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            info!("project_e: cognitive cycle runner shutting down");
                            break;
                        }
                    }
                    _ = tokio::time::sleep(self.heartbeat_interval) => {
                        match self.run_cycle().await {
                            Ok(result) => {
                                consecutive_failures = 0;
                                info!(
                                    cycle_id = %result.cycle_id,
                                    duration_secs = result.duration_secs,
                                    published = result.modules_published,
                                    rejected = result.modules_rejected,
                                    gaps = result.gaps_detected,
                                    "project_e: cognitive cycle complete"
                                );
                            }
                            Err(e) => {
                                consecutive_failures += 1;
                                warn!(
                                    error = %e,
                                    consecutive_failures,
                                    "project_e: cognitive cycle failed"
                                );
                                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                                    error!(
                                        consecutive_failures,
                                        "project_e: too many consecutive cycle failures — stopping runner"
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    /// Execute one full six-phase cognitive cycle.
    async fn run_cycle(&mut self) -> Result<CycleResult, String> {
        let cycle_id = Uuid::new_v4();
        let start = Instant::now();

        // ── Phase 0: Emit CycleStarted ────────────────────────────────────
        self.event_bus
            .emit(CognitiveEvent::CycleStarted {
                cycle_id,
                instance: self.instance.clone(),
            })
            .await
            .map_err(|e| format!("CycleStarted emit failed: {e}"))?;

        // ── Phase 1: Perception ────────────────────────────────────────────
        // Search the workspace for recent cognitive knowledge as raw items.
        let raw_items = self.collect_perception_items(cycle_id).await;
        let scored = self
            .perception
            .collect_cycle(cycle_id, raw_items)
            .await
            .map_err(|e| format!("perception failed: {e}"))?;

        // ── Phase 2: Interpretation (gap analysis) ─────────────────────────
        // Use the LLM to identify capability gaps from scored perception data.
        let gap_reports = self
            .identify_gaps(cycle_id, &scored)
            .await
            .unwrap_or_default();

        let gaps_detected = gap_reports.len();

        let filtered_gaps = self
            .analysis
            .analyze(cycle_id, gap_reports)
            .await
            .map_err(|e| format!("analysis failed: {e}"))?;

        // ── Phase 3: Deliberation ──────────────────────────────────────────
        let history = self.cycle_history.clone();
        let self_model = self.self_model_builder.build(
            0.5, // coverage — TODO: compute from filtered_gaps vs registry size
            1.0 - (filtered_gaps.len() as f32 / self.limits.max_actions_per_cycle as f32).min(1.0),
            0.7, // efficiency — placeholder until forge metrics are tracked
            0.5, // utilization — placeholder until resource metrics are tracked
            0.8, // freshness — workspace is fresh by design
            history,
        );

        let action_plan = self
            .deliberation
            .deliberate(cycle_id, filtered_gaps, &self_model);

        self.event_bus
            .emit(CognitiveEvent::ActionPlanReady {
                cycle_id,
                plan_id: action_plan.id,
                actions_count: action_plan.actions.len(),
            })
            .await
            .map_err(|e| format!("ActionPlanReady emit failed: {e}"))?;

        // ── Phase 4: Action (forge) ────────────────────────────────────────
        let (mut modules_published, mut modules_rejected) = (0usize, 0usize);

        // Respect max_concurrent_actions.
        let actions: Vec<_> = action_plan
            .actions
            .into_iter()
            .take(action_plan.max_concurrent)
            .collect();

        for planned_action in &actions {
            // Generate source code for the module using the LLM.
            let llm_call_idx = modules_published + modules_rejected;
            let source = self
                .generate_module_source(cycle_id, planned_action, llm_call_idx)
                .await;

            let source = match source {
                Ok(s) if !s.trim().is_empty() => s,
                Ok(_) | Err(_) => {
                    modules_rejected += 1;
                    continue;
                }
            };

            let module_name = format!("gap_{}", planned_action.gap_id.simple());

            match self
                .forge
                .run(cycle_id, planned_action.gap_id, &module_name, &source)
                .await
            {
                Ok(_) => modules_published += 1,
                Err(e) => {
                    warn!(error = %e, gap_id = %planned_action.gap_id, "project_e: forge failed");
                    modules_rejected += 1;
                }
            }
        }

        // ── Phase 5: Evaluation ────────────────────────────────────────────
        let efficiency = if actions.is_empty() {
            1.0
        } else {
            modules_published as f32 / actions.len() as f32
        };

        self.event_bus
            .emit(CognitiveEvent::EvaluationComplete {
                cycle_id,
                published: modules_published,
                rejected: modules_rejected,
                efficiency,
                skill_scores: vec![],
            })
            .await
            .ok(); // best-effort

        // ── Phase 6: MetaAdaptation ────────────────────────────────────────
        // Update the rolling cycle history for the self-model.
        let duration_secs = start.elapsed().as_secs();
        self.push_cycle_metrics(
            cycle_id,
            duration_secs,
            gaps_detected,
            modules_published,
            modules_rejected,
            efficiency,
            self_model.operating_mode,
        );

        // ── Cycle complete ─────────────────────────────────────────────────
        self.event_bus
            .emit(CognitiveEvent::CycleClosed {
                cycle_id,
                duration_secs,
            })
            .await
            .ok(); // best-effort

        let mut result = CycleResult::new(cycle_id, duration_secs);
        result.modules_published = modules_published;
        result.modules_rejected = modules_rejected;
        result.gaps_detected = gaps_detected;

        Ok(result)
    }

    /// Collect perception items from the workspace.
    ///
    /// Searches for recent cognitive-layer documents and wraps them as
    /// `RawDataItem` values for the perception scoring pipeline.
    async fn collect_perception_items(&self, cycle_id: Uuid) -> Vec<RawDataItem> {
        let query = "cognitive architecture module capability gap";
        let results = self.knowledge.search(query, 20).await.unwrap_or_default();

        results
            .into_iter()
            .map(|hit| RawDataItem {
                id: Uuid::new_v4(),
                source_type: "workspace".to_string(),
                source_url: None,
                title: Some(hit.path.clone()),
                content: hit.content,
                collected_at: Utc::now(),
                metadata: serde_json::json!({
                    "cycle_id": cycle_id.to_string(),
                    "path": hit.path,
                    "score": hit.score,
                }),
            })
            .collect()
    }

    /// Use the LLM to identify capability gaps from perception data.
    ///
    /// Returns pre-filtered `GapReport`s; the analysis engine will apply its
    /// own confidence threshold on top.
    async fn identify_gaps(
        &self,
        cycle_id: Uuid,
        scored: &[ScoredDataItem],
    ) -> Result<Vec<GapReport>, String> {
        if scored.is_empty() {
            return Ok(vec![]);
        }

        // Summarise perception data for the LLM prompt.
        let data_summary: String = scored
            .iter()
            .take(5)
            .map(|item| {
                let snippet: String = item.content().chars().take(200).collect();
                format!("• {}: {}", item.source_type(), snippet)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let system = "You are a cognitive architecture analyst. Identify specific capability gaps in the current IronClaw tool set based on the observed data. Output a JSON array of gap objects with fields: description (string), confidence (float 0-1), priority (\"high\"|\"medium\"|\"low\"), domain (\"simple\"|\"complicated\"|\"complex\"|\"chaotic\").";
        let user = format!(
            "Perceived data:\n{data_summary}\n\nIdentify up to 3 capability gaps. Output only valid JSON."
        );

        let response = self
            .llm
            .complete_text(system, &user, None, 0)
            .await
            .map_err(|e| format!("gap identification LLM call failed: {e}"))?;

        parse_gap_reports(cycle_id, &response)
    }

    /// Use the LLM to generate Rust source code for a planned module.
    async fn generate_module_source(
        &self,
        _cycle_id: Uuid,
        action: &PlannedAction,
        call_index: usize,
    ) -> Result<String, String> {
        let system = "You are a Rust expert generating safe, no-unsafe module stubs for an AI agent. Output only valid Rust code with no explanation.";
        let user = format!(
            "Generate a minimal Rust library crate that addresses the capability gap: {}. \
             Strategy: {:?}. The code must have no unsafe blocks, no unwrap(), no panic!(), no todo!(). \
             Output only the Rust source code for src/lib.rs.",
            action.action_label, action.strategy
        );

        self.llm
            .complete_text(system, &user, None, call_index + 1)
            .await
            .map_err(|e| format!("module generation LLM call failed: {e}"))
    }

    /// Push cycle metrics, trimming the history to the reflection window.
    fn push_cycle_metrics(
        &mut self,
        cycle_id: Uuid,
        duration_secs: u64,
        gaps_detected: usize,
        modules_published: usize,
        modules_rejected: usize,
        efficiency: f32,
        operating_mode: OperatingMode,
    ) {
        let metrics = CycleMetrics {
            cycle_id,
            duration_secs,
            gaps_detected,
            modules_published,
            modules_rejected,
            forge_efficiency: efficiency,
            operating_mode,
            completed_at: Utc::now(),
        };
        self.cycle_history.push(metrics);
        // Guard against a misconfigured window of 0 (keep at least the latest entry).
        let window = self.reflection_window_size.max(1);
        if self.cycle_history.len() > window {
            let excess = self.cycle_history.len() - window;
            self.cycle_history.drain(..excess);
        }
    }
}

/// Parse the LLM's JSON response into `GapReport`s.
///
/// Lenient: if the response can't be parsed, returns an empty vec rather than
/// failing the cycle.
fn parse_gap_reports(cycle_id: Uuid, response: &str) -> Result<Vec<GapReport>, String> {
    use project_e_core::types::{
        CynefinDomain, GapClassification, LifecycleAction, Priority, WardleyStage,
    };

    // Strip any markdown code fences the LLM may have added.
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json: serde_json::Value = match serde_json::from_str(cleaned) {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "project_e: could not parse gap identification response as JSON");
            return Ok(vec![]);
        }
    };

    let array = match json.as_array() {
        Some(a) => a,
        None => return Ok(vec![]),
    };

    let mut reports = Vec::with_capacity(array.len());
    for item in array {
        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown gap")
            .to_string();
        let confidence = item
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.6) as f32;
        let priority = match item.get("priority").and_then(|v| v.as_str()) {
            Some("high") => Priority::High,
            Some("low") => Priority::Low,
            _ => Priority::Medium,
        };
        let cynefin = match item.get("domain").and_then(|v| v.as_str()) {
            Some("complicated") => CynefinDomain::Complicated,
            Some("complex") => CynefinDomain::Complex,
            Some("chaotic") => CynefinDomain::Chaotic,
            _ => CynefinDomain::Simple,
        };

        reports.push(GapReport::new(
            cycle_id,
            GapClassification::Gap,
            confidence,
            cynefin,
            WardleyStage::Custom,
            priority,
            LifecycleAction::Create,
            description,
        ));
    }

    Ok(reports)
}
