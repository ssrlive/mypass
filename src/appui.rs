use crate::{error::Error, keepass::KpDb};
use eframe::{
    egui::{self, Hyperlink, Label, RichText, ScrollArea, TopBottomPanel},
    emath::Align,
};
use keepass::db::NodeRef;
use std::path::PathBuf;

const PADDING: f32 = 1.0;
pub const APP_NAME: &str = "mypass";

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
struct Config {
    dark_mode: bool,
}

#[derive(Default, Debug)]
struct UiState {
    show_open_file_dialog: bool,
    file_path: Option<PathBuf>,
    password: String,
    keyfile: Option<PathBuf>,

    show_confirm_quit_dialog: bool,
    allowed_to_quit: bool,

    config: Config,
}

impl UiState {
    fn on_show_open_file_dialog(&mut self) {
        self.show_open_file_dialog = true;
        self.show_confirm_quit_dialog = false;
        self.file_path = None;
        self.password.clear();
        self.keyfile = None;
    }

    fn is_open_file_dialog_visible(&self) -> bool {
        self.show_open_file_dialog
    }

    fn on_open_file_dialog_confirm(&mut self) -> (Option<PathBuf>, String, Option<PathBuf>) {
        self.show_open_file_dialog = false;
        let password = std::mem::take(&mut self.password);
        (self.file_path.take(), password, self.keyfile.take())
    }

    fn on_open_file_dialog_cancel(&mut self) {
        self.show_open_file_dialog = false;
        self.password.clear();
        self.file_path = None;
        self.keyfile = None;
    }

    fn on_show_confirm_quit_dialog(&mut self) {
        self.show_open_file_dialog = false;
        self.show_confirm_quit_dialog = true;
    }

    fn on_confirm_quit_dialog_quit(&mut self) {
        self.allowed_to_quit = true;
        self.show_confirm_quit_dialog = false;
    }

    fn on_confirm_quit_dialog_cancel(&mut self) {
        self.show_confirm_quit_dialog = false;
    }

    fn is_confirm_quit_dialog_visible(&self) -> bool {
        self.show_confirm_quit_dialog
    }

    fn is_allowed_to_quit(&self) -> bool {
        self.allowed_to_quit
    }
}

#[derive(Default, Debug)]
pub struct AppUI {
    kpdb: Option<KpDb>,
    state: UiState,
}

impl AppUI {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = cc
            .storage
            .and_then(|storage| storage.get_string(APP_NAME))
            .and_then(|cfg| serde_json::from_str::<Config>(&cfg).ok())
            .unwrap_or_default();
        let mut state = UiState::default();
        state.config = config;

        let block = || {
            let db_path = dotenvy::var("DB_PATH")?;
            let password = dotenvy::var("PASSWORD").ok();
            let key_file = dotenvy::var("KEY_FILE").ok();

            let kpdb = KpDb::open(&db_path, password.as_deref(), key_file.as_deref())?;
            Ok::<KpDb, Error>(kpdb)
        };

        Self {
            kpdb: block().ok(),
            state,
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
                        self.state.on_show_open_file_dialog();
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

                    let text = if self.state.config.dark_mode { "🔆" } else { "🌙" };
                    let text = RichText::new(text).text_style(egui::TextStyle::Body);
                    if ui.add(egui::Button::new(text)).clicked() {
                        self.state.config.dark_mode = !self.state.config.dark_mode;
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
        if self.state.is_confirm_quit_dialog_visible() {
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
                            self.state.on_confirm_quit_dialog_quit();
                            frame.close();
                            log::info!("{APP_NAME} closed.");
                        }
                        if ui.button("Cancel").clicked() {
                            self.state.on_confirm_quit_dialog_cancel();
                        }
                    });
                });
        }
    }

    fn render_open_file_dialog(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.state.is_open_file_dialog_visible() {
            let size = frame.info().window_info.size;
            let pos = egui::Pos2::new(size.x / 3.0, size.y / 3.0);

            let title = format!("Open keepass file");
            egui::Window::new(title)
                .collapsible(false)
                .resizable(true)
                .default_pos(pos)
                .show(ctx, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Pick keepass file").clicked() {
                            let path = rfd::FileDialog::new().pick_file();
                            if path.is_some() {
                                self.state.file_path = path;
                            }
                        }
                        let text = if let Some(path) = &self.state.file_path {
                            path.to_str().unwrap_or("Invalid path")
                        } else {
                            "Please pick a keepass database file"
                        };
                        ui.label(text);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Password");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state.password)
                                .password(true)
                                .desired_width(size.x),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Pick key file").clicked() {
                            let path = rfd::FileDialog::new().pick_file();
                            if path.is_some() {
                                self.state.keyfile = path;
                            }
                        }
                        let text = if let Some(path) = &self.state.keyfile {
                            path.to_str().unwrap_or("Invalid key file path")
                        } else {
                            "Pick a key file for the keepass database (optional)"
                        };
                        ui.label(text);
                    });
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Open").clicked() {
                            let (path, password, keyfile) = self.state.on_open_file_dialog_confirm();
                            if let Some(path) = path {
                                let path = path.to_str().unwrap_or("Invalid path");
                                let password = if password.is_empty() { None } else { Some(password) };
                                let keyfile = keyfile.as_ref().and_then(|p| p.to_str().and_then(|p| Some(p)));
                                self.kpdb = KpDb::open(path, password.as_deref(), keyfile).ok();
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.state.on_open_file_dialog_cancel();
                        }
                    });
                });
        }
    }
}

impl eframe::App for AppUI {
    fn on_close_event(&mut self) -> bool {
        self.state.on_show_confirm_quit_dialog();
        self.state.is_allowed_to_quit()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(cfg) = serde_json::to_string(&self.state.config) {
            storage.set_string(APP_NAME, cfg);
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.state.config.dark_mode {
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
