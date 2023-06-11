use crate::{
    error::Error,
    fonts::find_cjk_fonts,
    keepass::KpDb,
    password,
    tree::Tree,
    uistate::{Config, UiState},
};
use eframe::{
    egui::{self, Hyperlink, Label, RichText, ScrollArea, TopBottomPanel},
    emath::Align,
};
use keepass::db::NodeRef;

const PADDING: f32 = 1.0;
pub const APP_NAME: &str = "mypass";

#[derive(Default, Debug)]
pub struct AppUI {
    kpdb: Option<KpDb>,
    state: UiState,
}

impl AppUI {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_fonts(&cc.egui_ctx);
        let config = cc
            .storage
            .and_then(|storage| storage.get_string(APP_NAME))
            .and_then(|cfg| serde_json::from_str::<Config>(&cfg).ok())
            .unwrap_or_default();
        let mut state = UiState::default();
        state.config = config;

        let block = || {
            let db_path = std::env::var("DB_PATH")?;
            let password = std::env::var("PASSWORD").ok();
            let key_file = std::env::var("KEY_FILE").ok();

            let kpdb = KpDb::open(&db_path, password.as_deref(), key_file.as_deref())?;
            Ok::<KpDb, Error>(kpdb)
        };

        Self {
            kpdb: block().ok(),
            state,
            ..Default::default()
        }
    }

    pub fn configure_fonts(ctx: &eframe::egui::Context) -> Option<()> {
        let mut font_def = eframe::egui::FontDefinitions::default();

        let font_files = find_cjk_fonts()?;
        for font_file in font_files.iter() {
            Self::add_font(&mut font_def, font_file);
        }

        ctx.set_fonts(font_def);
        Some(())
    }

    fn add_font(font_def: &mut eframe::egui::FontDefinitions, font_file: &std::path::PathBuf) -> Option<()> {
        let font_name = font_file.file_stem()?.to_str()?.to_string();
        let font_file_bytes = std::fs::read(font_file).ok()?;
        let font_data = eframe::egui::FontData::from_owned(font_file_bytes);
        font_def.font_data.insert(font_name.to_string(), font_data);
        let font_family = eframe::epaint::FontFamily::Proportional;
        font_def.families.get_mut(&font_family)?.insert(0, font_name);
        Some(())
    }

    fn render_kp_node_details(&mut self, ui: &mut egui::Ui) {
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
            ui.horizontal(|ui| {
                egui::menu::menu_button(ui, "Main", |ui| {
                    let v = ["Hide architecture", "Show architecture"];
                    let show = &mut self.state.config.show_tree_panel;
                    let text = if *show { v[0] } else { v[1] };
                    if ui.button(text).clicked() {
                        *show = !*show;
                        ui.close_menu();
                    }
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
                if let Some(ref file_path) = self.kpdb.as_ref().and_then(|kpdb| kpdb.db_path.clone()) {
                    ui.vertical_centered(|ui| {
                        ui.label(file_path);
                    });
                }
            });
            ui.separator();

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
                        password::password_ui(ui, &mut self.state.password);
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
                    ui.separator();
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

    /// Preview hovering files
    fn render_preview_files_being_dropped(ctx: &egui::Context) {
        use egui::*;
        use std::fmt::Write as _;

        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let text = ctx.input(|i| {
                let mut text = "Dropping files:\n".to_owned();
                for file in &i.raw.hovered_files {
                    if let Some(path) = &file.path {
                        write!(text, "\n{}", path.display()).ok();
                    } else if !file.mime.is_empty() {
                        write!(text, "\n{}", file.mime).ok();
                    } else {
                        text += "\n???";
                    }
                }
                text
            });

            let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

            let screen_rect = ctx.screen_rect();
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.style()),
                Color32::WHITE,
            );
        }
    }

    fn render_tree_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("Architecture tree")
            .open(&mut self.state.config.show_tree_panel)
            .vscroll(true)
            .hscroll(true)
            .show(ctx, |ui| {
                let node = self
                    .kpdb
                    .as_ref()
                    .and_then(|kpdb| kpdb.get_root())
                    .map(|root| NodeRef::Group(root));
                Tree::default().ui(ui, &node);
            });
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
            ScrollArea::vertical().show(ui, |ui| {
                self.render_kp_node_details(ui);
            });
        });
        self.render_confirm_exit_dialog(ctx, frame);
        self.render_open_file_dialog(ctx, frame);
        self.render_tree_panel(ctx, frame);

        Self::render_preview_files_being_dropped(ctx);
        // Collect dropped files
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.state.dropped_files = i.raw.dropped_files.clone();
            }
        });

        if self.state.is_files_being_dropped() {
            self.state.deal_with_dropped_files();
        }
    }
}
