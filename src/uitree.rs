use crate::keepass::{get_uuid, is_group};
use eframe::egui;
use keepass::db::NodeRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum TreeEvent {
    NodeSelected(uuid::Uuid),
    NodeDeleted(uuid::Uuid),
    EntryCreated(uuid::Uuid),
    GroupCreated(uuid::Uuid),
}

#[derive(Debug, Default)]
pub(crate) struct UiTree {
    pub event: Option<TreeEvent>,
}

impl UiTree {
    pub fn ui(&mut self, ui: &mut egui::Ui, node: &Option<NodeRef<'_>>) {
        self.ui_impl(ui, 0, node)
    }

    pub fn peek_event(&mut self) -> Option<TreeEvent> {
        self.event.take()
    }
}

impl UiTree {
    fn ui_impl(&mut self, ui: &mut egui::Ui, depth: usize, node: &Option<NodeRef<'_>>) {
        let node_uuid = node.as_ref().map(get_uuid).copied();
        let title = if depth == 0 && node.is_none() {
            "No keepass database loaded"
        } else {
            node.as_ref()
                .and_then(|node| match node {
                    NodeRef::Group(g) => Some(g.name.as_str()),
                    NodeRef::Entry(e) => e.get_title(),
                })
                .unwrap_or("(no title)")
        };
        if node.as_ref().map(is_group).unwrap_or(false) {
            let response = egui::CollapsingHeader::new(title)
                .default_open(depth < 1)
                .show(ui, |ui| self.children_ui(ui, depth, node));
            response.header_response.context_menu(|ui| {
                if ui.button("Show details").clicked() {
                    log::info!("Show group {title} details");
                    self.event = node_uuid.map(TreeEvent::NodeSelected);
                    ui.close_menu();
                }
                if depth > 0 {
                    let del = egui::RichText::new("Delete").color(ui.visuals().warn_fg_color);
                    if ui.button(del).clicked() {
                        log::info!("Delete group {title}");
                        self.event = node_uuid.map(TreeEvent::NodeDeleted);
                        ui.close_menu();
                    }
                }
                if ui.button("Create new entry").clicked() {
                    log::info!("Create new entry in {title}");
                    self.event = node_uuid.map(TreeEvent::EntryCreated);
                    ui.close_menu();
                }
                if ui.button("Create new group").clicked() {
                    log::info!("Create new group in {title}");
                    self.event = node_uuid.map(TreeEvent::GroupCreated);
                    ui.close_menu();
                }
            });
            response.body_returned.unwrap_or(())
        } else {
            let response = ui.button(title).context_menu(|ui| {
                if ui.button("Show details").clicked() {
                    log::info!("Show entry details {title}");
                    self.event = node_uuid.map(TreeEvent::NodeSelected);
                    ui.close_menu();
                }
                let del = egui::RichText::new("Delete").color(ui.visuals().warn_fg_color);
                if ui.button(del).clicked() {
                    log::info!("Delete entry {title}");
                    self.event = node_uuid.map(TreeEvent::NodeDeleted);
                    ui.close_menu();
                }
            });
            if response.clicked() {
                self.event = node_uuid.map(TreeEvent::NodeSelected);
            }
        }
    }

    fn children_ui(&mut self, ui: &mut egui::Ui, depth: usize, node: &Option<NodeRef<'_>>) {
        if let Some(NodeRef::Group(group)) = node {
            group.children.iter().for_each(|node| {
                self.ui_impl(ui, depth + 1, &Some(node.into()));
            });
        }
    }
}
