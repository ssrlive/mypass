use crate::{entry_view::bitmap_for_icon, entry_view::bitmap_for_icon_fixed, find_tree_item, keepass::KpDb, node_title, show_node_view};
use keepass_ng::{
    Uuid,
    db::{Entry, Icon, Node, NodePtr, group_get_children, node_is_group},
};
use std::{cell::RefCell, rc::Rc};
use wxdragon::{
    BoxSizer, Button, ButtonEvents, HasItemData, ImageList, ListColumnFormat, ListCtrl, ListCtrlStyle, MessageDialog, MessageDialogStyle,
    Orientation, Panel, Size, SizerFlag, StaticBitmap, StaticText, StatusBar, TreeCtrl, WxWidget, image_list_type,
};

pub fn build_group_view(
    parent: &Panel,
    group: &NodePtr,
    tree: &TreeCtrl,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    status_bar: &StatusBar,
) {
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let title = StaticText::builder(parent).with_label(&node_title(group)).build();
    let edit_button = Button::builder(parent).with_label("Edit").with_size(Size::new(85, 34)).build();
    let parent_for_edit = *parent;
    edit_button.on_click(move |_| {
        MessageDialog::builder(&parent_for_edit, "Group editing is not implemented yet.", "Edit Group")
            .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
            .build()
            .show_modal();
    });
    let header_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    match group.borrow().get_icon() {
        Icon::BuiltIn(icon_id) => {
            let group_icon = StaticText::builder(parent).with_label(&icon_id.to_string()).build();
            if let Some(font) = wxdragon::Font::builder().with_point_size(20).build() {
                group_icon.set_font(&font);
            }
            group_icon.set_tooltip(&format!("Built-in icon {icon_id}"));
            header_sizer.add(&group_icon, 0, SizerFlag::AlignCenterVertical | SizerFlag::Right, 8);
        }
        Icon::Custom(uuid) => {
            let custom_icon = kpdb
                .borrow()
                .as_ref()
                .and_then(|db| db.db.as_ref())
                .and_then(|db| db.meta.custom_icon(uuid))
                .cloned();
            if let Some(custom_icon) = custom_icon
                && let Some(bitmap) = bitmap_for_icon(&custom_icon.data, 28)
            {
                let group_icon = StaticBitmap::builder(parent)
                    .with_bitmap(Some(bitmap))
                    .with_size(Size::new(32, 32))
                    .build();
                group_icon.set_tooltip(custom_icon.name().unwrap_or("Custom icon"));
                header_sizer.add(&group_icon, 0, SizerFlag::AlignCenterVertical | SizerFlag::Right, 8);
            }
        }
    }
    header_sizer.add(&title, 1, SizerFlag::Expand, 0);
    let header_spacer = StaticText::builder(parent).with_label("").build();
    header_sizer.add(&header_spacer, 1, SizerFlag::Expand, 0);
    header_sizer.add(&edit_button, 0, SizerFlag::AlignCenterVertical, 0);
    sizer.add_sizer(&header_sizer, 0, SizerFlag::All | SizerFlag::Expand, 12);

    let list = ListCtrl::builder(parent)
        .with_style(ListCtrlStyle::Report | ListCtrlStyle::SingleSel | ListCtrlStyle::HRules | ListCtrlStyle::VRules)
        .build();
    let image_list = ImageList::new(20, 20, false, 0);
    let children = group_get_children(group).unwrap_or_default();
    let mut row_icons = Vec::with_capacity(children.len());
    for child in &children {
        let child_icon = child.borrow().get_icon();
        let (icon_label, image_index) = match child_icon {
            Icon::BuiltIn(icon_id) => (icon_id.to_string(), None),
            Icon::Custom(uuid) => {
                let image_index = kpdb
                    .borrow()
                    .as_ref()
                    .and_then(|db| db.db.as_ref())
                    .and_then(|db| db.meta.custom_icon(uuid))
                    .and_then(|icon| bitmap_for_icon_fixed(&icon.data, 20))
                    .map(|bitmap| image_list.add_bitmap(&bitmap));
                (String::new(), image_index)
            }
        };
        row_icons.push((icon_label, image_index));
    }
    list.set_image_list(image_list, image_list_type::SMALL);
    list.insert_column(0, "Icon", ListColumnFormat::Left, 36);
    list.insert_column(1, "Type", ListColumnFormat::Left, 60);
    list.insert_column(2, "Title", ListColumnFormat::Left, 150);
    list.insert_column(3, "Username", ListColumnFormat::Left, 140);
    list.insert_column(4, "URL", ListColumnFormat::Left, 200);
    list.insert_column(5, "Last Modified", ListColumnFormat::Left, 140);
    list.insert_column(6, "Notes", ListColumnFormat::Left, -1);

    for (index, child) in children.iter().enumerate() {
        let row = index as i64;
        let (icon_label, image_index) = &row_icons[index];
        let kind = if node_is_group(child) { "Group" } else { "Entry" };
        if list.insert_item(row, icon_label, *image_index) < 0 {
            continue;
        }
        list.set_item_text_by_column(row, 1, kind);
        list.set_custom_data(row as u64, child.borrow().get_uuid());
        list.set_item_text_by_column(row, 2, &node_title(child));
        let last_modified = child
            .borrow()
            .get_times()
            .get_last_modification()
            .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();
        list.set_item_text_by_column(row, 5, &last_modified);
        if let Some(entry) = child.borrow().downcast_ref::<Entry>() {
            list.set_item_text_by_column(row, 3, entry.get_username().unwrap_or(""));
            list.set_item_text_by_column(row, 4, entry.get_url().unwrap_or(""));
            list.set_item_text_by_column(row, 6, entry.get_notes().unwrap_or(""));
        }
    }

    let tree_for_activation = *tree;
    let content_for_activation = *content;
    let current_view_for_activation = Rc::clone(current_view);
    let kpdb_for_activation = Rc::clone(kpdb);
    let status_bar_for_activation = *status_bar;
    list.on_item_activated(move |event| {
        let row = event.get_item_index();
        if row < 0 {
            return;
        }
        let Some(data) = list.get_custom_data(row as u64) else {
            return;
        };
        let Some(uuid) = data.downcast_ref::<Uuid>() else {
            return;
        };
        let uuid = *uuid;
        if let Some(root_item) = tree_for_activation.get_root_item()
            && let Some(tree_item) = find_tree_item(&tree_for_activation, &root_item, uuid)
        {
            tree_for_activation.select_item(&tree_item);
        }
        let Some(node) = kpdb_for_activation.borrow().as_ref().and_then(|db| db.get_node_by_id(uuid)) else {
            return;
        };
        show_node_view(
            &content_for_activation,
            &current_view_for_activation,
            &node,
            &tree_for_activation,
            &kpdb_for_activation,
            &status_bar_for_activation,
        );
        status_bar_for_activation.set_status_text("Node selected", 0);
    });

    sizer.add(&list, 1, SizerFlag::All | SizerFlag::Expand, 4);
    parent.set_sizer(sizer, true);
}
