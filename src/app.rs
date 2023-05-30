use crate::{error::Error, keepass::KpDb};
use eframe::{
    egui::{self, ScrollArea},
    emath::Align,
};
use keepass::db::NodeRef;

const PADDING: f32 = 1.0;

#[derive(Default)]
pub struct App {
    kpdb: Option<KpDb>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let block = || {
            let db_path = dotenvy::var("DB_PATH")?;
            let password = dotenvy::var("PASSWORD")?;
            // let key_file = dotenvy::var("KEY_FILE")?;

            let kpdb = KpDb::open(&db_path, Some(&password), None)?;
            Ok::<KpDb, Error>(kpdb)
        };
        Self { kpdb: block().ok() }
    }

    fn render_kp_items(&mut self, ui: &mut egui::Ui) {
        if let Some(kpdb) = &self.kpdb {
            if let Some(root) = kpdb.get_root() {
                for node in root {
                    self.draw_node(ui, node);
                }
            }
        }
    }

    fn draw_node(&self, ui: &mut egui::Ui, node: NodeRef) {
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("KeePass items");
            ui.label("This is a list of KeePass items");

            ScrollArea::vertical().show(ui, |ui| {
                self.render_kp_items(ui);
            });
        });
    }
}
