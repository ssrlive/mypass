#![windows_subsystem = "windows"]

use mypass::{app::App, error::Error};

fn main() {
    let block = || {
        dotenvy::dotenv()?;

        let verbose = true;
        let log_level = if verbose { "trace" } else { "info" };
        let root = module_path!().split("::").next().unwrap();
        let filter_str = &format!("off,{root}={log_level}");
        let env = env_logger::Env::default().default_filter_or(filter_str);
        env_logger::Builder::from_env(env).init();

        let native_options = eframe::NativeOptions::default();

        eframe::run_native(
            "MyPass",
            native_options,
            Box::new(|cc| Box::new(App::new(cc))),
        )?;
        Ok::<(), Error>(())
    };
    if let Err(e) = block() {
        log::error!("{}", e);
    }
}
