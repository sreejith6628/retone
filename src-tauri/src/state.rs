//! Shared app state.
use crate::platform::Target;
use retone_core::job::Busy;
use retone_core::settings::{self, KeyringStore, Settings};
use std::path::PathBuf;
use std::sync::Mutex;

/// Text captured for the options panel, waiting for the user to press Polish.
pub struct Pending { pub target: Option<Target>, pub draft: String }

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub secrets: KeyringStore,
    pub busy: Busy,
    pub pending: Mutex<Option<Pending>>,
    /// Set while the pill shows an error; clicking the pill then opens Settings.
    pub error_showing: Mutex<bool>,
}

impl AppState {
    pub fn load(settings_path: PathBuf) -> AppState {
        AppState {
            settings: Mutex::new(settings::load(&settings_path)),
            settings_path,
            secrets: KeyringStore,
            busy: Busy::default(),
            pending: Mutex::new(None),
            error_showing: Mutex::new(false),
        }
    }

    pub fn snapshot(&self) -> Settings { self.settings.lock().unwrap().clone() }

    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> std::io::Result<Settings> {
        let mut s = self.settings.lock().unwrap();
        f(&mut s);
        settings::save(&self.settings_path, &s)?;
        Ok(s.clone())
    }
}
