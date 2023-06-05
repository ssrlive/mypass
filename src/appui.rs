use crate::{error::Error, keepass::KpDb};
use eframe::{
    egui::{self, Hyperlink, Label, RichText, ScrollArea, TopBottomPanel},
    emath::Align,
};
use keepass::db::NodeRef;

const PADDING: f32 = 1.0;
pub const APP_NAME: &str = "mypass";

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
struct Config {
    dark_mode: bool,
}

#[derive(Default, Debug)]
pub struct AppUI {
    kpdb: Option<KpDb>,
    file_path: Option<std::path::PathBuf>,
    password: String,
    allowed_to_close: bool,
    show_confirmation_dialog: bool,
    show_open_file_dialog: bool,
    config: Config,
}

impl AppUI {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = cc
            .storage
            .and_then(|storage| storage.get_string(APP_NAME))
            .and_then(|cfg| serde_json::from_str::<Config>(&cfg).ok())
            .unwrap_or_default();

        let block = || {
            let db_path = dotenvy::var("DB_PATH")?;
            let password = dotenvy::var("PASSWORD")?;
            // let key_file = dotenvy::var("KEY_FILE")?;

            let kpdb = KpDb::open(&db_path, Some(&password), None)?;
            Ok::<KpDb, Error>(kpdb)
        };
        Self {
            kpdb: block().ok(),
            config,
            ..Default::default()
        }
    }

    fn render_kp_items(&mut self, ui: &mut egui::Ui) {
        if let Some(kpdb) = &self.kpdb {
            if let Some(root) = kpdb.get_root() {
                for node in root {
                    self.render_kp_node(ui, node);
                }
            }
        }
    }

    fn render_kp_node(&self, ui: &mut egui::Ui, node: NodeRef) {
        ui.separator();
        ui.add_space(PADDING);
        match node {
            NodeRef::Group(g) => {
                ui.horizontal(|ui| {
                    ui.label("Group");
                    ui.label(g.uuid.to_string());
                });
                ui.label(g.name.clone());
            }
            NodeRef::Entry(entry) => {
                ui.horizontal(|ui| {
                    ui.label("Entry");
                    ui.label(entry.uuid.to_string());
                    ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                        ui.add(egui::Hyperlink::new("https://www.rust-lang.org/"));
                    });
                });
                ui.label(entry.get_title().unwrap_or("(no title)"));
                ui.label(entry.get_username().unwrap_or("(no username)"));
                ui.label(entry.get_password().unwrap_or("(no password)"));
                ui.horizontal(|ui| {
                    ui.label("URL");
                    if let Some(url) = entry.get_url() {
                        ui.add(egui::Hyperlink::new(url));
                    }
                });
            }
        }
        ui.add_space(PADDING);
    }

    fn render_top_panel(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(PADDING);
            egui::menu::bar(ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(Align::Max), |ui| {
                    let text = RichText::new("🗋").text_style(egui::TextStyle::Heading);
                    ui.add(egui::Label::new(text));

                    let text = RichText::new("🗁").text_style(egui::TextStyle::Heading);
                    if ui.add(egui::Button::new(text)).clicked() {
                        self.show_open_file_dialog = true;
                    }

                    let text = RichText::new("💾").text_style(egui::TextStyle::Heading);
                    ui.add(egui::Button::new(text));
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                    let text = RichText::new("❌").text_style(egui::TextStyle::Body);
                    if ui.add(egui::Button::new(text)).clicked() {
                        frame.close();
                    }

                    let text = RichText::new("🔄").text_style(egui::TextStyle::Body);
                    if ui.add(egui::Button::new(text)).clicked() {}

                    let text = if self.config.dark_mode { "🔆" } else { "🌙" };
                    let text = RichText::new(text).text_style(egui::TextStyle::Body);
                    if ui.add(egui::Button::new(text)).clicked() {
                        self.config.dark_mode = !self.config.dark_mode;
                    }
                });
            });
            ui.add_space(PADDING);
        });
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("KeePass items");
            ui.label("This is a list of KeePass items");
        });
        ui.add_space(PADDING);
        // ui.add(Separator::default().spacing(20.0));
    }

    fn render_footer(&self, ctx: &egui::Context) {
        TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.vertical_centered(|ui: &mut egui::Ui| {
                ui.add_space(PADDING);
                ui.add(Label::new("This is a footer").wrap(false));
                ui.add(Hyperlink::from_label_and_url(
                    "Made with egui",
                    "https://gihub.com/emilk/egui",
                ));
                ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                    ui.add(egui::Hyperlink::new("https://www.rust-lang.org/"));
                });
            });
        });
    }

    fn render_confirm_exit_dialog(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.show_confirmation_dialog {
            let size = frame.info().window_info.size;
            let pos = egui::Pos2::new(size.x / 3.0, size.y / 3.0);

            let title = format!("Do you want to quit {APP_NAME} really?");
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .default_pos(pos)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Quit").clicked() {
                            self.allowed_to_close = true;
                            frame.close();
                            log::info!("{APP_NAME} closed.");
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_confirmation_dialog = false;
                        }
                    });
                });
        }
    }

    fn render_open_file_dialog(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.show_open_file_dialog {
            let size = frame.info().window_info.size;
            let pos = egui::Pos2::new(size.x / 3.0, size.y / 3.0);

            let title = format!("Open keepass file");
            egui::Window::new(title)
                .collapsible(false)
                .resizable(true)
                .default_pos(pos)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Pick a file").clicked() {
                            self.file_path = rfd::FileDialog::new().pick_file();
                            log::info!("file path: {:?}", self.file_path);
                        }
                        if let Some(path) = &self.file_path {
                            let path = path.to_str().unwrap_or("invalid path");
                            ui.label(path);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Password:");
                        ui.add(egui::TextEdit::singleline(&mut self.password));
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Open").clicked() {
                            self.show_open_file_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_open_file_dialog = false;
                        }
                    });
                });
        }
    }
}

impl eframe::App for AppUI {
    fn on_close_event(&mut self) -> bool {
        self.show_confirmation_dialog = true;
        self.allowed_to_close
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(cfg) = serde_json::to_string(&self.config) {
            storage.set_string(APP_NAME, cfg);
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.config.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
        self.render_top_panel(ctx, frame);
        self.render_footer(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui);
            ScrollArea::vertical().show(ui, |ui| {
                self.render_kp_items(ui);
            });
        });
        self.render_confirm_exit_dialog(ctx, frame);
        self.render_open_file_dialog(ctx, frame);
    }
}
