use mypass::{app::App, error::Error};

fn main() {
    let block = || {
        env_logger::init();

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
