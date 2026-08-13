#![allow(unused_variables)]
use crate::{
    error::Error,
    fonts::find_cjk_fonts,
    keepass::KpDb,
    password,
    uistate::{Config, UiState},
    uitree::{TreeEvent, UiTree},
};
use eframe::{
    egui::{self, Hyperlink, Label, Panel, RichText, ScrollArea},
    emath::Align,
};
use std::cell::RefCell;
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, Receiver, Sender};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

const PADDING: f32 = 1.0;
pub const APP_NAME: &str = "mypass";

#[derive(Clone, Copy)]
enum TrayAction {
    ToggleWindow,
    About,
    Quit,
}

#[derive(Default)]
struct TrayMenuIds {
    show_hide: Option<String>,
    about: Option<String>,
    quit: Option<String>,
}

struct TrayState {
    _icon: TrayIcon,
    ids: TrayMenuIds,
}

thread_local! {
    static TRAY_STATE: RefCell<Option<TrayState>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub struct AppUI {
    kpdb: Option<KpDb>,
    state: UiState,
    uitree: UiTree,
    #[cfg(target_os = "linux")]
    tray_actions: Option<Receiver<TrayAction>>,
}

impl AppUI {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_fonts(&cc.egui_ctx);
        let config = cc
            .storage
            .and_then(|storage| storage.get_string(APP_NAME))
            .and_then(|cfg| serde_json::from_str::<Config>(&cfg).ok())
            .unwrap_or_default();
        let mut state = UiState::new();
        state.config = config;

        let block = || {
            let db_path = std::env::var("DB_PATH")?;
            let password = std::env::var("PASSWORD").ok();
            let key_file = std::env::var("KEY_FILE").ok();

            let kpdb = KpDb::open(&db_path, password.as_deref(), key_file.as_deref())?;
            Ok::<KpDb, Error>(kpdb)
        };

        #[cfg(target_os = "linux")]
        let (tray_sender, tray_actions) = mpsc::channel::<TrayAction>();
        let app = Self {
            kpdb: block().ok(),
            state,
            #[cfg(target_os = "linux")]
            tray_actions: Some(tray_actions),
            ..Default::default()
        };
        #[cfg(target_os = "linux")]
        Self::start_linux_tray(cc.egui_ctx.clone(), tray_sender);
        app
    }

    #[cfg(target_os = "linux")]
    fn start_linux_tray(egui_ctx: egui::Context, tray_sender: Sender<TrayAction>) {
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let action = Self::tray_action_for_id(event.id.as_ref());
            if let Some(action) = action {
                let _ = tray_sender.send(action);
                egui_ctx.request_repaint();
            }
        }));

        let icon = Self::create_tray_icon();
        std::thread::spawn(move || {
            gtk::init().expect("initialize GTK for tray icon");
            TRAY_STATE.with(|state| {
                let mut state = state.borrow_mut();
                *state = Some(Self::build_tray_state(icon));
            });
            gtk::main();
        });
    }

    fn build_tray_state(icon: Icon) -> TrayState {
        let tray_menu = Menu::new();
        let show_hide = MenuItem::new("Show/Hide main window", true, None);
        let about = MenuItem::new("About", true, None);
        let quit = MenuItem::new("Quit", true, None);
        tray_menu.append(&show_hide).expect("append tray menu item");
        tray_menu.append(&about).expect("append tray menu item");
        tray_menu.append(&quit).expect("append tray menu item");

        let ids = TrayMenuIds {
            show_hide: Some(show_hide.id().as_ref().to_owned()),
            about: Some(about.id().as_ref().to_owned()),
            quit: Some(quit.id().as_ref().to_owned()),
        };
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip(APP_NAME)
            .with_icon(icon)
            .build()
            .expect("create tray icon");

        TrayState { _icon: icon, ids }
    }

    fn tray_action_for_id(id: &str) -> Option<TrayAction> {
        TRAY_STATE.with(|state| {
            let state = state.borrow();
            let ids = &state.as_ref()?.ids;
            if ids.show_hide.as_deref() == Some(id) {
                Some(TrayAction::ToggleWindow)
            } else if ids.about.as_deref() == Some(id) {
                Some(TrayAction::About)
            } else if ids.quit.as_deref() == Some(id) {
                Some(TrayAction::Quit)
            } else {
                None
            }
        })
    }

    fn create_tray_icon() -> Icon {
        let mut rgba = vec![0_u8; 16 * 16 * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[30, 144, 255, 255]);
        }
        Icon::from_rgba(rgba, 16, 16).expect("create tray icon image")
    }

    #[cfg(not(target_os = "linux"))]
    fn setup_tray_icon(&mut self) {
        TRAY_STATE.with(|state| {
            if state.borrow().is_some() {
                return;
            }
            let icon = Self::create_tray_icon();
            *state.borrow_mut() = Some(Self::build_tray_state(icon));
            log::info!("System tray icon created");
        });
    }

    fn toggle_main_window(&self, frame: &mut eframe::Frame) {
        if let Some(window) = frame.winit_window() {
            let is_visible = window.is_visible().unwrap_or(false);
            let should_show = !is_visible;
            window.set_visible(should_show);
            if should_show {
                window.focus_window();
            }
        }
    }

    fn handle_tray_events(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(target_os = "linux")]
        if let Some(actions) = &self.tray_actions {
            while let Ok(action) = actions.try_recv() {
                match action {
                    TrayAction::ToggleWindow => self.toggle_main_window(frame),
                    TrayAction::About => self.state.show_about_dialog = true,
                    TrayAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match Self::tray_action_for_id(event.id.as_ref()) {
                Some(TrayAction::ToggleWindow) => self.toggle_main_window(frame),
                Some(TrayAction::About) => self.state.show_about_dialog = true,
                Some(TrayAction::Quit) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                None => {}
            }
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
        font_def.font_data.insert(font_name.to_string(), font_data.into());
        let font_family = eframe::epaint::FontFamily::Proportional;
        font_def.families.get_mut(&font_family)?.insert(0, font_name);
        Some(())
    }

    fn render_top_panel(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        Panel::top("top_panel").show(ui, |ui| {
            ui.add_space(PADDING);
            ui.horizontal(|ui| {
                ui.menu_button("Main", |ui| {
                    let v = ["Hide architecture", "Show architecture"];
                    let show = &mut self.state.config.show_tree_panel;
                    let text = if *show { v[0] } else { v[1] };
                    if ui.button(text).clicked() {
                        *show = !*show;
                        ui.close_kind(egui::UiKind::Menu);
                    }
                    if ui.button("Quit").clicked() {
                        self.state.on_show_confirm_quit_dialog();
                    }
                });
                if let Some(ref file_path) = self.kpdb.as_ref().and_then(|kpdb| kpdb.db_path.clone()) {
                    ui.vertical_centered(|ui| {
                        ui.label(file_path);
                    });
                }
            });
            ui.separator();

            egui::MenuBar::new().ui(ui, |ui| {
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
                        // frame.close();
                    }

                    let text = RichText::new("🔄").text_style(egui::TextStyle::Body);
                    if ui.add(egui::Button::new(text)).clicked() {
                        // TODO: refresh
                    }

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

    fn render_footer(&self, ui: &mut egui::Ui) {
        Panel::bottom("footer").show(ui, |ui| {
            ui.vertical_centered(|ui: &mut egui::Ui| {
                ui.add_space(PADDING);
                ui.add(Label::new("This is a footer"));
                ui.add(Hyperlink::from_label_and_url("Made with egui", "https://gihub.com/emilk/egui"));
                ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                    ui.add(egui::Hyperlink::new("https://www.rust-lang.org/"));
                });
            });
        });
    }

    fn viewport_size(ctx: &egui::Context) -> egui::Vec2 {
        ctx.input(|i| {
            i.viewport()
                .inner_rect
                .map(|r| r.size())
                .unwrap_or_else(|| egui::vec2(960.0, 640.0))
        })
    }

    fn render_confirm_exit_dialog(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.is_confirm_quit_dialog_visible() {
            let size = Self::viewport_size(ctx);
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
                            // frame.close();
                            log::info!("{APP_NAME} closed.");
                        }
                        if ui.button("Cancel").clicked() {
                            self.state.on_confirm_quit_dialog_cancel();
                        }
                    });
                });
        }
    }

    fn render_about_dialog(&mut self, ctx: &egui::Context) {
        if self.state.show_about_dialog {
            let size = Self::viewport_size(ctx);
            let pos = egui::Pos2::new(size.x / 3.0, size.y / 3.0);

            egui::Window::new("About mypass")
                .collapsible(false)
                .resizable(false)
                .order(egui::Order::Foreground)
                .default_pos(pos)
                .show(ctx, |ui| {
                    ui.heading(APP_NAME);
                    ui.label("A KeePass database viewer.");
                    ui.separator();
                    ui.label("Built with egui and keepass-ng.");
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        if ui.button("Close").clicked() {
                            self.state.show_about_dialog = false;
                        }
                    });
                });
        }
    }

    fn render_open_file_dialog(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.state.is_open_file_dialog_visible() {
            let size = Self::viewport_size(ctx);
            let pos = egui::Pos2::new(size.x / 3.0, size.y / 3.0);

            egui::Window::new("Open keepass file")
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
                                let keyfile = keyfile.as_ref().and_then(|p| p.to_str());
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
            let screen_rect = ctx
                .input(|i| i.viewport().inner_rect)
                .unwrap_or(Rect::from_min_size(Pos2::ZERO, Vec2::new(960.0, 640.0)));
            painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
            painter.text(
                screen_rect.center(),
                Align2::CENTER_CENTER,
                text,
                TextStyle::Heading.resolve(&ctx.global_style()),
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
                let node = self.kpdb.as_ref().and_then(|kpdb| kpdb.get_root());
                self.uitree.ui(ui, node);
            });
    }

    fn render_kp_node_details_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) -> Option<()> {
        let node = self
            .state
            .current_node_id
            .and_then(|id| self.kpdb.as_ref().and_then(|kpdb| kpdb.get_node_by_id(id)))?;
        let title = &node.borrow().get_title().unwrap_or("(no title)").to_owned();

        let size = Self::viewport_size(ctx);
        let pos = egui::Pos2::new(size.x * 0.3, size.y / 5.0);

        egui::Window::new(title)
            .open(&mut self.state.show_details_panel)
            .default_pos(pos)
            .default_width(size.x / 2.0)
            .vscroll(true)
            .hscroll(true)
            .show(ctx, |ui| {
                ui.label(title);
                ui.label(format!("{:?}", node));
            });
        Some(())
    }

    fn db_events_handler(&mut self) {
        if let Some(event) = self.uitree.peek_event() {
            match event {
                TreeEvent::NodeSelected(node) => {
                    self.state.show_details_panel = true;
                    self.state.current_node_id = Some(node);
                }
                TreeEvent::NodeDeleted(id) => {
                    self.state.show_details_panel = false;
                    self.state.current_node_id = None;
                    self.kpdb.as_mut().map(|kpdb| kpdb.delete_node(id));
                }
                TreeEvent::EntryCreated(parent_id) => {
                    self.kpdb.as_mut().map(|kpdb| {
                        kpdb.create_new_entry(parent_id).map(|node| {
                            self.state.show_details_panel = true;
                            self.state.current_node_id = Some(node.borrow().get_uuid());
                        })
                    });
                }
                TreeEvent::GroupCreated(parent_id) => {
                    self.kpdb.as_mut().map(|kpdb| {
                        kpdb.create_new_group(parent_id).map(|node| {
                            self.state.show_details_panel = true;
                            self.state.current_node_id = Some(node.borrow().get_uuid());
                        })
                    });
                }
            }
        }
    }
}

impl eframe::App for AppUI {
    /*
    fn on_close_event(&mut self) -> bool {
        self.state.on_show_confirm_quit_dialog();
        self.state.is_allowed_to_quit()
    }
    */
    fn on_exit(&mut self) {
        log::info!("on_exit");
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Ok(cfg) = serde_json::to_string(&self.state.config) {
            storage.set_string(APP_NAME, cfg);
        }
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(not(target_os = "linux"))]
        self.setup_tray_icon();
        self.handle_tray_events(ctx, frame);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        #[cfg(not(target_os = "linux"))]
        self.setup_tray_icon();
        self.handle_tray_events(&ctx, frame);
        if self.state.config.dark_mode {
            ctx.set_theme(egui::Theme::Dark);
        } else {
            ctx.set_theme(egui::Theme::Light);
        }
        self.render_top_panel(ui, frame);
        self.render_footer(ui);
        egui::CentralPanel::default().show(ui, |ui| {
            ScrollArea::vertical().show(ui, |_: &mut egui::Ui| {});
        });
        self.render_confirm_exit_dialog(&ctx, frame);
        self.render_about_dialog(&ctx);
        self.render_open_file_dialog(&ctx, frame);
        self.render_tree_panel(&ctx, frame);

        self.db_events_handler();

        self.render_kp_node_details_panel(&ctx, frame);

        Self::render_preview_files_being_dropped(&ctx);
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
