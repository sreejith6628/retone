mod commands;
mod flows;
mod menus;
mod pill;
mod platform;
mod shortcuts;
mod state;
mod winmgr;

use state::AppState;
use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| winmgr::open_settings(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            commands::provider_list, commands::provider_status, commands::test_provider,
            commands::get_settings, commands::save_settings, commands::finish_first_run,
            commands::has_api_key, commands::set_api_key, commands::delete_api_key, commands::sign_in,
            commands::options_init, commands::options_go, commands::options_cancel,
            commands::pill_click, commands::pill_right_click, commands::pill_moved,
            commands::open_settings, commands::accessibility_status, commands::open_accessibility_settings,
            commands::platform_name, commands::default_style,
        ])
        .on_menu_event(|app, e| menus::handle(app, e.id().as_ref()))
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let path = app.path().app_config_dir()?.join("settings.json");
            app.manage(AppState::load(path));
            let handle = app.handle().clone();
            menus::build_tray(&handle)?;

            // Pill mouse events arrive on the OS hook thread; handle them elsewhere.
            let (tx, rx) = std::sync::mpsc::channel();
            let events_app = handle.clone();
            std::thread::spawn(move || for ev in rx { pill::handle_event(&events_app, ev); });
            let pill_win = pill::window(&handle).expect("pill window in tauri.conf.json");
            #[cfg(windows)]
            {
                pill_win.set_ignore_cursor_events(true)?;
                platform::install_pill(pill_win.hwnd()?.0 as isize, tx);
            }
            #[cfg(not(windows))]
            {
                let _ = &pill_win;
                platform::install_pill(0, tx);
            }

            let settings = app.state::<AppState>().snapshot();
            for w in shortcuts::register(&handle, &settings) { eprintln!("retone: {w}"); }
            if settings.first_run_done {
                pill::show(&handle, settings.show_pill);
            } else {
                winmgr::open_settings(&handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Retone")
        .run(|_app, event| {
            // Keep running in the tray when the last visible window closes.
            if let RunEvent::ExitRequested { api, code: None, .. } = event { api.prevent_exit(); }
        });
}
