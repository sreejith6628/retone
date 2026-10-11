//! User-facing flows: quick polish and polish-with-options.
use crate::pill::{set_status, PillSink};
use crate::platform::{self, PlatformDesktop, Target};
use crate::state::{AppState, Pending};
use crate::winmgr;
use retone_core::job::{self, JobRequest, PillStatus};
use retone_core::models;
use retone_core::prompt::{Length, Tone};
use retone_core::providers::registry::{build_provider, ProviderId};
use tauri::{AppHandle, Manager};

/// Which provider/model a job uses (defaults from settings unless overridden in the options panel).
pub struct Choice { pub provider: String, pub model: Option<String> }

pub fn default_choice(app: &AppHandle) -> Choice {
    let s = app.state::<AppState>().snapshot();
    let model = s.models.get(&s.default_provider).cloned().filter(|m| !m.is_empty())
        .or_else(|| models::default_model(&s.default_provider));
    Choice { provider: s.default_provider, model }
}

async fn copy(target: Option<Target>) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || platform::copy_from(target.as_ref())).await.ok().flatten()
}

const STILL_WORKING: &str = "Still working…";

fn options_open(app: &AppHandle) -> bool { app.get_webview_window(winmgr::OPTIONS).is_some() }

/// Select text → press shortcut / click pill → polished in place with the default tone.
pub fn quick_polish(app: AppHandle) {
    if options_open(&app) {
        winmgr::focus_options(&app);
        return;
    }
    let target = platform::current_target();
    tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        // Hold the busy guard from the copy onwards: two overlapping copies would
        // clobber each other's clipboard snapshot.
        let Some(guard) = st.busy.try_start() else { set_status(&app, PillStatus::Hint(STILL_WORKING.into())); return; };
        let Some(draft) = copy(target).await else {
            set_status(&app, PillStatus::Hint(job::HINT_NOTHING_SELECTED.into()));
            return;
        };
        let tone = Tone::from_name(&st.snapshot().default_tone).unwrap_or(Tone::Standard);
        run_job(&app, target, draft, tone, Length::Same, String::new(), default_choice(&app)).await;
        drop(guard);
    });
}

/// Copy first (while the user's app still has focus), then show the options panel.
pub fn open_options(app: AppHandle) {
    if options_open(&app) {
        winmgr::focus_options(&app);
        return;
    }
    let target = platform::current_target();
    tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        let Some(guard) = st.busy.try_start() else { set_status(&app, PillStatus::Hint(STILL_WORKING.into())); return; };
        let draft = copy(target).await.unwrap_or_default();
        drop(guard);
        *st.pending.lock().unwrap() = Some(Pending { target, draft });
        winmgr::open_options(&app);
    });
}

pub fn options_go(app: AppHandle, tone: Tone, length: Length, context: String, choice: Choice) {
    let pending = app.state::<AppState>().pending.lock().unwrap().take();
    winmgr::close(&app, winmgr::OPTIONS);
    let Some(Pending { target, draft }) = pending else { return };
    tauri::async_runtime::spawn(async move {
        let st = app.state::<AppState>();
        let Some(guard) = st.busy.try_start() else { set_status(&app, PillStatus::Hint(STILL_WORKING.into())); return; };
        // Give the options window time to close so the target can take focus back.
        tokio_sleep(250).await;
        run_job(&app, target, draft, tone, length, context, choice).await;
        drop(guard);
    });
}

async fn tokio_sleep(ms: u64) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(std::time::Duration::from_millis(ms))).await;
}

/// Runs one job. The caller holds the busy guard.
async fn run_job(app: &AppHandle, target: Option<Target>, draft: String, tone: Tone, length: Length,
                 context: String, choice: Choice) {
    let st = app.state::<AppState>();
    let settings = st.snapshot();
    let Some(id) = ProviderId::from_key(&choice.provider) else {
        set_status(app, PillStatus::Error("Unknown AI provider — click to open Settings".into()));
        return;
    };
    let provider = build_provider(id, &settings, &st.secrets);
    let req = JobRequest { draft, tone, length, context, style: settings.style.clone(), model: choice.model };
    let mut desktop = PlatformDesktop { target };
    let _ = job::run(req, provider.as_ref(), &mut desktop, &PillSink(app.clone())).await;
}
