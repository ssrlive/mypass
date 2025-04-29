#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use mypass::{
    appui::{APP_NAME, AppUI},
    error::Error,
};

fn main() {
    let block = || {
        dotenvy::dotenv().ok();

        let verbose = true;
        let log_level = if verbose { "trace" } else { "info" };
        let root = module_path!().split("::").next().unwrap();
        let filter_str = &format!("off,{root}={log_level}");
        let env = env_logger::Env::default().default_filter_or(filter_str);
        env_logger::Builder::from_env(env).init();

        let native_options = eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_drag_and_drop(true)
                .with_inner_size([960.0, 640.0]),
            ..Default::default()
        };

        eframe::run_native(APP_NAME, native_options, Box::new(|cc| Ok(Box::new(AppUI::new(cc)))))?;
        Ok::<(), Error>(())
    };
    if let Err(e) = block() {
        log::error!("{}", e);
    }
}
