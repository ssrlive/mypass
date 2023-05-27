use eframe::egui::{self, ScrollArea};

const WHITE: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const PADDING: f32 = 1.0;

#[derive(Default)]
pub struct App {
    kp_items: Vec<KpItem>,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let iter = (0..20).map(|i| KpItem {
            title: format!("title {}", i),
            username: format!("username {}", i),
            password: format!("password {}", i),
        });
        Self {
            kp_items: iter.collect(),
        }
    }

    fn render_kp_items(&mut self, ui: &mut egui::Ui) {
        for item in &mut self.kp_items {
            ui.separator();
            ui.add_space(PADDING);
            ui.colored_label(WHITE, item.title.clone());
            ui.add_space(PADDING);
            ui.horizontal(|ui| {
                ui.label(item.username.clone());
                ui.label(item.password.clone());
            });
            ui.add_space(PADDING);
        }
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

#[derive(Debug, Default)]
struct KpItem {
    title: String,
    username: String,
    password: String,
}
