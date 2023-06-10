use eframe::egui::{self, Label, Sense};
use keepass::db::NodeRef;

#[derive(Debug, Default)]
pub(crate) struct Tree;

impl Tree {
    pub fn ui(&mut self, ui: &mut egui::Ui, node: &Option<NodeRef<'_>>) {
        self.ui_impl(ui, 0, node)
    }

    pub fn is_group(&self, node: &NodeRef<'_>) -> bool {
        matches!(node, NodeRef::Group(_))
    }
}

impl Tree {
    fn ui_impl(&mut self, ui: &mut egui::Ui, depth: usize, node: &Option<NodeRef<'_>>) {
        let title = node
            .as_ref()
            .and_then(|node| match node {
                NodeRef::Group(g) => Some(g.name.as_str()),
                NodeRef::Entry(e) => e.get_title(),
            })
            .unwrap_or("(no title)");

        if node.as_ref().map(|node| self.is_group(node)).unwrap_or(false) {
            let response = egui::CollapsingHeader::new(title)
                .default_open(depth < 1)
                .show(ui, |ui| self.children_ui(ui, depth, node));
            response.header_response.context_menu(|ui| {
                if ui.button("Show details").clicked() {
                    log::info!("Show group {title} details");
                    ui.close_menu();
                }
                if depth > 0 {
                    let del = egui::RichText::new("Delete").color(ui.visuals().warn_fg_color);
                    if ui.button(del).clicked() {
                        log::info!("Delete group {title}");
                        ui.close_menu();
                    }
                }
                if ui.button("Create new entry").clicked() {
                    log::info!("Create new entry in {title}");
                    ui.close_menu();
                }
                if ui.button("Create new group").clicked() {
                    log::info!("Create new group in {title}");
                    ui.close_menu();
                }
            });
            response.body_returned.unwrap_or(())
        } else {
            let response = ui.add(Label::new(title).sense(Sense::click()));
            response.context_menu(|ui| {
                if ui.button("Show details").clicked() {
                    log::info!("Show entry details {title}");
                    ui.close_menu();
                }
                let del = egui::RichText::new("Delete").color(ui.visuals().warn_fg_color);
                if ui.button(del).clicked() {
                    log::info!("Delete entry {title}");
                    ui.close_menu();
                }
            });
        }
    }

    fn children_ui(&mut self, ui: &mut egui::Ui, depth: usize, node: &Option<NodeRef<'_>>) {
        if let Some(node) = node {
            if let NodeRef::Group(group) = node {
                group.children.iter().for_each(|node| {
                    self.ui_impl(ui, depth + 1, &Some(node.into()));
                });
            }
        }
    }
}
