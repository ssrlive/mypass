use eframe::egui;
use keepass_ng::{
    Uuid,
    db::{Entry, NodePtr, group_get_children, node_is_group},
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
    fn node_title(node: &NodePtr) -> String {
        let node = node.borrow();
        let title = node.get_title().map(str::trim).filter(|title| !title.is_empty());
        let entry_parts = node.downcast_ref::<Entry>().map(|entry| {
            let name = entry
                .get_username()
                .or_else(|| entry.get("Email"))
                .and_then(|value| value.split('@').next())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            let website = entry
                .get_url()
                .or_else(|| entry.get("Website"))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(Self::website_display_name);

            (name, website)
        });

        let Some((name, website)) = entry_parts else {
            return title.unwrap_or("(no title)").to_owned();
        };

        let account = match (name, website) {
            (Some(name), Some(website)) => format!("{name} @ {website}"),
            (Some(name), None) => name,
            (None, Some(website)) => website,
            (None, None) => String::new(),
        };

        match (title, account.is_empty()) {
            (Some(title), false) => format!("{title} · {account}"),
            (Some(title), true) => title.to_owned(),
            (None, false) => account,
            (None, true) => "(no title)".to_string(),
        }
    }

    fn website_display_name(website: &str) -> String {
        url::Url::parse(website)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_else(|| website.to_owned())
    }

    fn ui_impl(&mut self, ui: &mut egui::Ui, depth: usize, node: Option<NodePtr>) {
        let node_uuid = node.as_ref().map(|n| n.borrow().get_uuid());
        let title = if depth == 0 && node.is_none() {
            "No keepass database loaded".to_string()
        } else {
            Self::node_title(node.as_ref().unwrap())
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
