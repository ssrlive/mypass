#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use mypass::{
    appui::{AppUI, APP_NAME},
    error::Error,
};

fn main() {
    let block = || {
        _ = dotenvy::dotenv();

        let verbose = true;
        let log_level = if verbose { "trace" } else { "info" };
        let root = module_path!().split("::").next().unwrap();
        let filter_str = &format!("off,{root}={log_level}");
        let env = env_logger::Env::default().default_filter_or(filter_str);
        env_logger::Builder::from_env(env).init();

        let native_options = eframe::NativeOptions {
            drag_and_drop_support: true,
            initial_window_size: Some(eframe::egui::vec2(960.0, 640.0)),
            ..Default::default()
        };

        eframe::run_native(APP_NAME, native_options, Box::new(|cc| Box::new(AppUI::new(cc))))?;
        Ok::<(), Error>(())
    };
    if let Err(e) = block() {
        log::error!("{}", e);
    }
}
