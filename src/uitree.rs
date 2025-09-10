use eframe::egui;
use keepass_ng::{
    Uuid,
    db::{NodePtr, group_get_children, node_is_group},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum TreeEvent {
    NodeSelected(Uuid),
    NodeDeleted(Uuid),
    EntryCreated(Uuid),
    GroupCreated(Uuid),
}

#[derive(Debug, Default)]
pub(crate) struct UiTree {
    pub event: Option<TreeEvent>,
}

impl UiTree {
    pub fn ui(&mut self, ui: &mut egui::Ui, node: Option<NodePtr>) {
        self.ui_impl(ui, 0, node)
    }

    pub fn peek_event(&mut self) -> Option<TreeEvent> {
        self.event.take()
    }
}

impl UiTree {
    fn ui_impl(&mut self, ui: &mut egui::Ui, depth: usize, node: Option<NodePtr>) {
        let node_uuid = node.as_ref().map(|n| n.borrow().get_uuid());
        let title = if depth == 0 && node.is_none() {
            "No keepass database loaded".to_string()
        } else {
            node.as_ref().unwrap().borrow().get_title().unwrap_or("(no title)").to_string()
        };
        if node.as_ref().map(node_is_group).unwrap_or(false) {
            let response = egui::CollapsingHeader::new(&title)
                .default_open(depth < 1)
                .show(ui, |ui| self.children_ui(ui, depth, node.clone()));
            response.header_response.context_menu(|ui| {
                if ui.button("Show details").clicked() {
                    log::info!("Show group {title} details");
                    self.event = node_uuid.map(TreeEvent::NodeSelected);
                    ui.close_kind(egui::UiKind::Menu);
                }
                if depth > 0 {
                    let del = egui::RichText::new("Delete").color(ui.visuals().warn_fg_color);
                    if ui.button(del).clicked() {
                        log::info!("Delete group {title}");
                        self.event = node_uuid.map(TreeEvent::NodeDeleted);
                        ui.close_kind(egui::UiKind::Menu);
                    }
                }
                if ui.button("Create new entry").clicked() {
                    log::info!("Create new entry in {title}");
                    self.event = node_uuid.map(TreeEvent::EntryCreated);
                    ui.close_kind(egui::UiKind::Menu);
                }
                if ui.button("Create new group").clicked() {
                    log::info!("Create new group in {title}");
                    self.event = node_uuid.map(TreeEvent::GroupCreated);
                    ui.close_kind(egui::UiKind::Menu);
                }
            });
            response.body_returned.unwrap_or(())
        } else {
            let _response = ui.button(&title).context_menu(|ui| {
                if ui.button("Show details").clicked() {
                    log::info!("Show entry details {title}");
                    self.event = node_uuid.map(TreeEvent::NodeSelected);
                    ui.close_kind(egui::UiKind::Menu);
                }
                let del = egui::RichText::new("Delete").color(ui.visuals().warn_fg_color);
                if ui.button(del).clicked() {
                    log::info!("Delete entry {title}");
                    self.event = node_uuid.map(TreeEvent::NodeDeleted);
                    ui.close_kind(egui::UiKind::Menu);
                }
            });
            // if _response.clicked() {
            //     self.event = node_uuid.map(TreeEvent::NodeSelected);
            // }
        }
    }

    fn children_ui(&mut self, ui: &mut egui::Ui, depth: usize, node: Option<NodePtr>) {
        if let Some(ref group) = node {
            group_get_children(group).unwrap().iter().for_each(|node| {
                self.ui_impl(ui, depth + 1, Some(node.clone()));
            });
        }
    }
}
