use crate::{
    entry_view::{bitmap_for_icon, bitmap_for_icon_fixed},
    find_tree_item,
    icon_cache::icon_for_emoji,
    keepass::KpDb,
    node_title, show_node_view,
};
use keepass_ng::{
    Uuid,
    db::{Entry, Group, Icon, IconId, Node, NodePtr, group_get_children, node_is_group, with_node, with_node_mut},
};
use std::{cell::RefCell, rc::Rc};
use wxdragon::{
    BoxSizer, Button, ButtonEvents, CheckBox, FlexGridSizer, Frame, HasItemData, ImageList, ListColumnFormat, ListCtrl, ListCtrlStyle,
    Notebook, Orientation, Panel, ScrolledWindow, ScrolledWindowStyle, Size, SizerFlag, StaticBitmap, StaticText, StatusBar, TextCtrl,
    TextCtrlStyle, TreeCtrl, WxWidget, image_list_type,
};

#[allow(clippy::too_many_arguments)]
pub fn build_group_view(
    parent: &Panel,
    frame: Frame,
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
    let group_for_edit = group.clone();
    let kpdb_for_edit = Rc::clone(kpdb);
    let tree_for_refresh = *tree;
    let content_for_refresh = *content;
    let current_view_for_refresh = Rc::clone(current_view);
    let status_bar_for_refresh = *status_bar;
    edit_button.on_click(move |_| {
        if show_group_editor(&frame, &group_for_edit, Rc::clone(&kpdb_for_edit)) == wxdragon::ID_OK {
            show_node_view(
                &content_for_refresh,
                frame,
                &current_view_for_refresh,
                &group_for_edit,
                &tree_for_refresh,
                &kpdb_for_edit,
                &status_bar_for_refresh,
            );
        }
    });
    let header_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    match group.borrow().get_icon() {
        Icon::BuiltIn(icon_id) => {
            if let Some(bitmap) = icon_for_emoji(icon_id.to_string().trim(), 28) {
                let group_icon = StaticBitmap::builder(parent)
                    .with_bitmap(Some(bitmap))
                    .with_size(Size::new(32, 32))
                    .build();
                group_icon.set_tooltip(&format!("Built-in icon {icon_id}"));
                header_sizer.add(&group_icon, 0, SizerFlag::AlignCenterVertical | SizerFlag::Right, 8);
            }
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
            Icon::BuiltIn(icon_id) => {
                let image_index = icon_for_emoji(&icon_id.to_string(), 20).map(|bitmap| image_list.add_bitmap(&bitmap));
                (String::new(), image_index)
            }
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
            frame,
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

pub(crate) fn show_group_editor(parent: &dyn WxWidget, node: &NodePtr, kpdb: Rc<RefCell<Option<KpDb>>>) -> wxdragon::Id {
    let Some(group) = with_node::<Group, _, _>(node, |group| group.clone()) else {
        return wxdragon::ID_CANCEL;
    };

    let dialog = wxdragon::Dialog::builder(parent, "Edit Group").with_size(760, 580).build();
    dialog.set_min_size(Size::new(760, 580));
    let dialog_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let notebook = Notebook::builder(&dialog).build();

    let group_page = Panel::builder(&notebook).build();
    let group_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let group_grid = FlexGridSizer::builder(0, 2).with_vgap(8).with_hgap(12).build();
    group_grid.add_growable_col(1, 1);
    let name = TextCtrl::builder(&group_page).with_value(group.get_title().unwrap_or("")).build();
    let notes = TextCtrl::builder(&group_page)
        .with_value(group.get_notes().unwrap_or(""))
        .with_style(TextCtrlStyle::MultiLine)
        .with_size(Size::new(-1, 150))
        .build();
    let expires = CheckBox::builder(&group_page)
        .with_label("Expires")
        .with_value(group.get_times().get_expires())
        .build();
    let search_inheritance = CheckBox::builder(&group_page)
        .with_label("Search settings are inherited from parent")
        .with_value(true)
        .build();
    search_inheritance.enable(false);
    let autotype_inheritance = CheckBox::builder(&group_page)
        .with_label("Auto-Type settings are inherited from parent")
        .with_value(true)
        .build();
    autotype_inheritance.enable(false);
    group_grid.add(&StaticText::builder(&group_page).with_label("Name").build(), 0, SizerFlag::All, 4);
    group_grid.add(&name, 1, SizerFlag::All | SizerFlag::Expand, 4);
    group_grid.add(&StaticText::builder(&group_page).with_label("Notes").build(), 0, SizerFlag::All, 4);
    group_grid.add(&notes, 1, SizerFlag::All | SizerFlag::Expand, 4);
    group_grid.add(
        &StaticText::builder(&group_page).with_label("Expiration").build(),
        0,
        SizerFlag::All,
        4,
    );
    group_grid.add(&expires, 1, SizerFlag::All | SizerFlag::Expand, 4);
    group_grid.add(&StaticText::builder(&group_page).with_label("Search").build(), 0, SizerFlag::All, 4);
    group_grid.add(&search_inheritance, 1, SizerFlag::All, 4);
    group_grid.add(
        &StaticText::builder(&group_page).with_label("Auto-Type").build(),
        0,
        SizerFlag::All,
        4,
    );
    group_grid.add(&autotype_inheritance, 1, SizerFlag::All, 4);
    group_sizer.add_sizer(&group_grid, 0, SizerFlag::All | SizerFlag::Expand, 12);
    group_sizer.add(
        &StaticText::builder(&group_page)
            .with_label("Inheritance options are shown for compatibility; this database library currently exposes them as read-only.")
            .build(),
        0,
        SizerFlag::All,
        12,
    );
    group_page.set_sizer(group_sizer, true);

    let icon_page = Panel::builder(&notebook).build();
    let icon_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let selected_icon = Rc::new(std::cell::Cell::new(group.get_icon()));
    let icon_buttons = Rc::new(RefCell::new(Vec::<(Button, Icon)>::new()));
    let selected_label = StaticText::builder(&icon_page).with_label("").build();
    let selected_background = wxdragon::Colour::rgb(198, 224, 180);
    let default_background = wxdragon::Colour::rgb(255, 255, 255);
    icon_sizer.add(
        &StaticText::builder(&icon_page).with_label("Built-in icons").build(),
        0,
        SizerFlag::All,
        4,
    );
    let built_in_sizer = BoxSizer::builder(Orientation::Vertical).build();
    const ICONS_PER_ROW: usize = 15;
    for row_start in (0..IconId::count()).step_by(ICONS_PER_ROW) {
        let row = BoxSizer::builder(Orientation::Horizontal).build();
        for icon_number in row_start..(row_start + ICONS_PER_ROW).min(IconId::count()) {
            let icon_number: IconId = icon_number.try_into().unwrap_or(IconId::KEY);
            let button = Button::builder(&icon_page).with_size(Size::new(36, 36)).build();
            if let Some(bitmap) = icon_for_emoji(&icon_number.to_string(), 28) {
                button.set_bitmap_label(&bitmap);
            }
            let icon = Icon::BuiltIn(icon_number);
            let buttons_for_click = Rc::clone(&icon_buttons);
            let selected_for_click = Rc::clone(&selected_icon);
            let label_for_click = selected_label;
            button.on_click(move |_| {
                selected_for_click.set(icon);
                for (candidate, candidate_icon) in buttons_for_click.borrow().iter() {
                    candidate.set_background_color(if *candidate_icon == icon {
                        selected_background
                    } else {
                        default_background
                    });
                }
                label_for_click.set_label(&format!("Selected built-in icon {icon_number}"));
            });
            icon_buttons.borrow_mut().push((button, icon));
            row.add(&button, 0, SizerFlag::All, 2);
        }
        built_in_sizer.add_sizer(&row, 0, SizerFlag::All, 0);
    }
    icon_sizer.add_sizer(&built_in_sizer, 0, SizerFlag::All, 4);
    icon_sizer.add(
        &StaticText::builder(&icon_page).with_label("Custom icons in this database").build(),
        0,
        SizerFlag::All,
        4,
    );
    let custom_scroll = ScrolledWindow::builder(&icon_page).with_style(ScrolledWindowStyle::VScroll).build();
    custom_scroll.set_scroll_rate(0, 20);
    custom_scroll.enable_scrolling(false, true);
    let custom_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let mut custom_rows = Vec::<BoxSizer>::new();
    const CUSTOM_ICONS_PER_ROW: usize = 15;
    let custom_icons = kpdb
        .borrow()
        .as_ref()
        .and_then(|db| db.db.as_ref())
        .map(|db| {
            db.meta
                .custom_icons()
                .map(|(uuid, icon)| (*uuid, icon.data.clone(), icon.name().unwrap_or("Custom icon").to_owned()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (index, (uuid, data, icon_name)) in custom_icons.into_iter().enumerate() {
        let button = Button::builder(&custom_scroll).with_size(Size::new(42, 36)).build();
        if let Some(bitmap) = bitmap_for_icon(&data, 28) {
            button.set_bitmap_label(&bitmap);
        }
        button.set_tooltip(&icon_name);
        let icon = Icon::Custom(uuid);
        let buttons_for_click = Rc::clone(&icon_buttons);
        let selected_for_click = Rc::clone(&selected_icon);
        let label_for_click = selected_label;
        button.on_click(move |_| {
            selected_for_click.set(icon);
            for (candidate, candidate_icon) in buttons_for_click.borrow().iter() {
                candidate.set_background_color(if *candidate_icon == icon {
                    selected_background
                } else {
                    default_background
                });
            }
            label_for_click.set_label(&format!("Selected custom icon {uuid}"));
        });
        icon_buttons.borrow_mut().push((button, icon));
        let row_index = index / CUSTOM_ICONS_PER_ROW;
        let row = if let Some(row) = custom_rows.get(row_index).copied() {
            row
        } else {
            let row = BoxSizer::builder(Orientation::Horizontal).build();
            custom_sizer.add_sizer(&row, 0, SizerFlag::All, 0);
            custom_rows.push(row);
            row
        };
        row.add(&button, 0, SizerFlag::All, 2);
    }
    custom_scroll.set_sizer(custom_sizer, true);
    icon_sizer.add(&custom_scroll, 1, SizerFlag::All | SizerFlag::Expand, 4);
    icon_sizer.add(&selected_label, 0, SizerFlag::All, 4);
    icon_page.set_sizer(icon_sizer, true);
    let current_icon = selected_icon.get();
    for (button, icon) in icon_buttons.borrow().iter() {
        button.set_background_color(if *icon == current_icon {
            selected_background
        } else {
            default_background
        });
    }
    selected_label.set_label(&format!("Selected icon {current_icon:?}"));

    let properties_page = Panel::builder(&notebook).build();
    let properties_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let properties_grid = FlexGridSizer::builder(0, 2).with_vgap(8).with_hgap(12).build();
    let created = group.get_times().get_creation().map(|time| time.to_string()).unwrap_or_default();
    let modified = group.get_times().get_last_modification().map(|t| t.to_string()).unwrap_or_default();
    for (label, value) in [("Created", created), ("Modified", modified), ("UUID", group.get_uuid().to_string())] {
        properties_grid.add(
            &StaticText::builder(&properties_page).with_label(label).build(),
            0,
            SizerFlag::All,
            4,
        );
        properties_grid.add(
            &TextCtrl::builder(&properties_page)
                .with_value(&value)
                .with_style(TextCtrlStyle::ReadOnly)
                .with_size(Size::new(400, 28))
                .build(),
            1,
            SizerFlag::All | SizerFlag::Expand,
            4,
        );
    }
    properties_sizer.add_sizer(&properties_grid, 0, SizerFlag::All | SizerFlag::Expand, 12);
    let custom_data = ListCtrl::builder(&properties_page)
        .with_style(ListCtrlStyle::Report | ListCtrlStyle::SingleSel | ListCtrlStyle::VRules | ListCtrlStyle::HRules)
        .build();
    custom_data.insert_column(0, "Key", ListColumnFormat::Left, 240);
    custom_data.insert_column(1, "Value", ListColumnFormat::Left, -1);
    let mut custom_items: Vec<_> = group.custom_data().iter().collect();
    custom_items.sort_by(|left, right| left.0.cmp(right.0));
    for (index, (key, item)) in custom_items.iter().enumerate() {
        if custom_data.insert_item(index as i64, key, None) >= 0 {
            custom_data.set_item_text_by_column(index as i64, 1, &format!("{item:?}"));
        }
    }
    properties_sizer.add(
        &StaticText::builder(&properties_page).with_label("Plugin Data").build(),
        0,
        SizerFlag::All,
        4,
    );
    properties_sizer.add(&custom_data, 1, SizerFlag::All | SizerFlag::Expand, 4);
    properties_page.set_sizer(properties_sizer, true);

    notebook.add_page(&group_page, "Group", true, None);
    notebook.add_page(&icon_page, "Icon", false, None);
    notebook.add_page(&properties_page, "Properties", false, None);
    dialog_sizer.add(&notebook, 1, SizerFlag::All | SizerFlag::Expand, 8);
    let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let spacer = StaticText::builder(&dialog).with_label("").build();
    let cancel = Button::builder(&dialog).with_id(wxdragon::ID_CANCEL).with_label("Cancel").build();
    let ok = Button::builder(&dialog).with_label("OK").build();
    button_sizer.add(&spacer, 1, SizerFlag::Expand, 0);
    button_sizer.add(&cancel, 0, SizerFlag::All, 4);
    button_sizer.add(&ok, 0, SizerFlag::All, 4);
    dialog_sizer.add_sizer(&button_sizer, 0, SizerFlag::All | SizerFlag::Expand, 8);
    dialog.set_sizer(dialog_sizer, true);
    dialog.set_escape_id(wxdragon::ID_CANCEL);

    let dialog_for_cancel = dialog;
    cancel.on_click(move |_| dialog_for_cancel.end_modal(wxdragon::ID_CANCEL));
    let dialog_for_ok = dialog;
    let node_for_ok = node.clone();
    let selected_icon_for_ok = Rc::clone(&selected_icon);
    ok.on_click(move |_| {
        let name_value = name.get_value();
        let notes_value = notes.get_value();
        with_node_mut::<Group, _, _>(&node_for_ok, |group| {
            group.set_title(if name_value.trim().is_empty() { None } else { Some(&name_value) });
            group.set_notes(Some(&notes_value));
            group.set_icon(selected_icon_for_ok.get());
            group.get_times_mut().set_expires(expires.get_value());
            group.get_times_mut().set_last_modification(Some(keepass_ng::db::Times::now()));
        });
        if let Some(db) = kpdb.borrow_mut().as_mut() {
            db.mark_data_changed();
        }
        dialog_for_ok.end_modal(wxdragon::ID_OK);
    });
    dialog.center();
    let result = dialog.show_modal();
    dialog.destroy();
    result
}
