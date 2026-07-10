use color_eyre::Result;
use engines::{DownloadEvent, InferenceEvent};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::app_enums::Mode;
use crate::core::dashboard::DashboardEngine;
use crate::core::flow::FlowEngine;
use crate::core::state::{ActivityBlock, AppState, OsState, UserProfile};
use crate::theme::Theme;
use colored::Colorize;

pub struct App {
    pub state: AppState,
    pub tab: crate::app_enums::Tab,
    pub mode: Mode,
    pub theme: Theme,
    pub tx: mpsc::UnboundedSender<DownloadEvent>,
    pub rx: mpsc::UnboundedReceiver<DownloadEvent>,
    pub _inf_tx: mpsc::UnboundedSender<InferenceEvent>,
    pub _abort_handle: Option<Arc<AtomicBool>>,
    pub _last_frame_time: Instant,
    pub flow: FlowEngine,
}

impl App {
    pub fn new(
        profile_override: Option<UserProfile>,
        auto_start_model: Option<engines::models::registry::ModelManifest>,
    ) -> color_eyre::Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (inf_tx, _inf_rx) = mpsc::unbounded_channel();
        let flow = FlowEngine::new()?;
        let mut state = AppState::new(profile_override);

        if let Some(m) = auto_start_model {
            let model_id = m.id.clone();
            let model_name = m.name.clone();
            state._active_model_id = Some(model_id.clone());
            state
                .activity_stream
                .push(ActivityBlock::ModelMounted(model_name.clone()));

            // Fast-track the model to sorted_models at the top
            state.sorted_models.insert(
                0,
                engines::models::registry::ModelRecommendation {
                    manifest: m,
                    status: "Optimal".to_string(),
                    is_cached: true,
                },
            );
            state.roster_state.select(Some(0));
        }

        let app = Self {
            state,
            tab: crate::app_enums::Tab::All,
            mode: Mode::Running,
            theme: Theme::default(),
            tx,
            rx,
            _inf_tx: inf_tx,
            _abort_handle: None,
            _last_frame_time: Instant::now(),
            flow,
        };
        let _ = _inf_rx;
        Ok(app)
    }

    pub async fn run(mut self) -> Result<()> {
        while self.mode != Mode::Quit {
            match self.state.os_state {
                OsState::Onboarding(_) => {
                    crate::ui::apps::onboarding::native::run_native_flow()?;
                    self.state.os_state = OsState::Dashboard;
                }
                OsState::Dashboard => {
                    // ── 0. Auto-Pilot Linkage (Sovereign Mount) ──
                    if !self.state.auto_mount_triggered {
                        // 🔍 Auto-Selection: Pick first available model if none active
                        if self.state._active_model_id.is_none() {
                            if let Some(model) =
                                self.state.sorted_models.iter().find(|m| m.is_cached)
                            {
                                self.state._active_model_id = Some(model.manifest.id.clone());
                            }
                        }

                        if let Some(active_id) = &self.state._active_model_id {
                            if let Some(model) = self
                                .state
                                .sorted_models
                                .iter()
                                .find(|m| &m.manifest.id == active_id)
                            {
                                if let Some(local_path) =
                                    engines::models::fetch::ModelDownloader::get_cached_path(
                                        &model.manifest.category,
                                        &model.manifest.id,
                                        &model.manifest.huggingface_filename,
                                    )
                                {
                                    self.state.auto_mount_triggered = true;
                                    let engine = self.state.Core_engine.clone();
                                    println!(
                                        "  {} Mounting auto-selected model: {}",
                                        "⚙️".yellow(),
                                        model.manifest.name
                                    );
                                    tokio::spawn(async move {
                                        let _ = engine.load_model(local_path).await;
                                    });
                                }
                            }
                        }
                    }

                    // ── 1. Unified Interface Initialization ──
                    if !self.state.printed_logo {
                        let _ = crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::LeaveAlternateScreen
                        );
                        let _ = crossterm::terminal::disable_raw_mode();
                        print!("\x1B[2J\x1B[1;1H"); // Clear and home
                        crate::assets::logos::logo::print_native_logo(self.state.logo_index);
                        println!();
                        println!("  {} {}", "CLUAIZ".cyan().bold(), "v0.1.0".bright_black());
                        println!(
                            "  {} {}",
                            "API Gateway: ".dimmed(),
                            "http://0.0.0.0:8000 ↗".cyan().bold()
                        );
                        println!(
                            "  {} {}",
                            "Dashboard:   ".dimmed(),
                            "http://0.0.0.0:3030 ↗".yellow().bold()
                        );
                        self.state.printed_logo = true;
                    }

                    // ── 2. Background Event Processing ──
                    self.state.handle_events(&mut self.rx);
                    crate::ui::apps::stream::commit_to_stdout(&mut self.state);

                    // ── 3. Native Dashboard Interaction ──
                    DashboardEngine::run_native(
                        &mut self.state,
                        &self.tx,
                        &mut self.rx,
                        &mut self.mode,
                    )?;
                }
            }
        }
        Ok(())
    }
}
