use crate::{entry_view::bitmap_for_icon, keepass::KpDb};
use keepass_ng::{
    Uuid,
    db::{Icon, IconId},
};
use std::{
    cell::{Cell, RefCell},
    fs,
    rc::Rc,
};
use wxdragon::{
    BoxSizer, Button, ButtonEvents, Dialog, FileDialog, FileDialogStyle, MessageDialog, MessageDialogStyle, Orientation, Panel,
    ScrolledWindow, ScrolledWindowStyle, Size, SizerFlag, StaticText, WxWidget,
};

pub(crate) fn show_icon_picker(parent: &dyn WxWidget, kpdb: Rc<RefCell<Option<KpDb>>>, current: Icon) -> Option<Icon> {
    let dialog = Dialog::builder(parent, "Icon Picker").with_size(760, 580).build();
    let root = BoxSizer::builder(Orientation::Vertical).build();
    let panel = Panel::builder(&dialog).build();
    let panel_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let selected = Rc::new(Cell::new(current));
    let buttons = Rc::new(RefCell::new(Vec::<(Button, Icon)>::new()));
    let selected_label = StaticText::builder(&panel).with_label("").build();
    set_dialog_icon(&dialog, &current, &kpdb);
    let selected_background = wxdragon::Colour::rgb(198, 224, 180);
    let default_background = wxdragon::Colour::rgb(255, 255, 255);

    panel_sizer.add(
        &StaticText::builder(&panel).with_label("Built-in icons").build(),
        0,
        SizerFlag::All,
        4,
    );
    let builtin = BoxSizer::builder(Orientation::Vertical).build();
    const PER_ROW: usize = 15;
    for start in (0..IconId::count()).step_by(PER_ROW) {
        let row = BoxSizer::builder(Orientation::Horizontal).build();
        for number in start..(start + PER_ROW).min(IconId::count()) {
            let id: IconId = number.try_into().unwrap_or(IconId::KEY);
            let button = Button::builder(&panel).with_size(Size::new(36, 36)).build();
            if let Some(bitmap) = crate::icon_cache::icon_for_emoji(&id.to_string(), 28) {
                button.set_bitmap_label(&bitmap);
            }
            button.set_tooltip(&format!("Built-in icon {id}"));
            let icon = Icon::BuiltIn(id);
            bind_icon_button(
                &button,
                icon,
                &buttons,
                &selected,
                dialog,
                &kpdb,
                selected_label,
                selected_background,
                default_background,
            );
            buttons.borrow_mut().push((button, icon));
            row.add(&button, 0, SizerFlag::All, 2);
        }
        builtin.add_sizer(&row, 0, SizerFlag::All, 0);
    }
    panel_sizer.add_sizer(&builtin, 0, SizerFlag::All, 4);
    panel_sizer.add(
        &StaticText::builder(&panel).with_label("Custom icons in this database").build(),
        0,
        SizerFlag::All,
        4,
    );
    let scroll = ScrolledWindow::builder(&panel).with_style(ScrolledWindowStyle::VScroll).build();
    scroll.set_scroll_rate(0, 20);
    scroll.enable_scrolling(false, true);
    let custom_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let add_custom = Rc::new({
        let buttons = Rc::clone(&buttons);
        let selected = Rc::clone(&selected);
        let kpdb = Rc::clone(&kpdb);
        move |uuid: Uuid, data: &[u8], name: Option<&str>| {
            let button = Button::builder(&scroll).with_size(Size::new(42, 36)).build();
            if let Some(bitmap) = bitmap_for_icon(data, 28) {
                button.set_bitmap_label(&bitmap);
            }
            button.set_tooltip(name.unwrap_or("Custom icon"));
            let icon = Icon::Custom(uuid);
            bind_icon_button(
                &button,
                icon,
                &buttons,
                &selected,
                dialog,
                &kpdb,
                selected_label,
                selected_background,
                default_background,
            );
            buttons.borrow_mut().push((button, icon));
            custom_sizer.add(&button, 0, SizerFlag::All, 2);
        }
    });
    if let Some(db) = kpdb.borrow().as_ref().and_then(|db| db.db.as_ref()) {
        for (uuid, icon) in db.meta.custom_icons() {
            add_custom(*uuid, &icon.data, icon.name());
        }
    }
    scroll.set_sizer(custom_sizer, true);
    panel_sizer.add(&scroll, 1, SizerFlag::All | SizerFlag::Expand, 4);
    panel_sizer.add(&selected_label, 0, SizerFlag::All, 4);
    panel.set_sizer(panel_sizer, true);
    root.add(&panel, 1, SizerFlag::All | SizerFlag::Expand, 8);

    let actions = BoxSizer::builder(Orientation::Horizontal).build();
    let add = Button::builder(&dialog).with_label("Add").build();
    let delete = Button::builder(&dialog).with_label("Delete").build();
    let clear_unused = Button::builder(&dialog).with_label("Clear unused").build();
    let more = Button::builder(&dialog).with_label("More...").build();
    let spacer = StaticText::builder(&dialog).with_label("").build();
    let cancel = Button::builder(&dialog).with_id(wxdragon::ID_CANCEL).with_label("Cancel").build();
    let ok = Button::builder(&dialog).with_label("OK").build();
    actions.add(&add, 0, SizerFlag::All, 4);
    actions.add(&delete, 0, SizerFlag::All, 4);
    actions.add(&clear_unused, 0, SizerFlag::All, 4);
    actions.add(&more, 0, SizerFlag::All, 4);
    actions.add(&spacer, 1, SizerFlag::Expand, 0);
    actions.add(&cancel, 0, SizerFlag::All, 4);
    actions.add(&ok, 0, SizerFlag::All, 4);
    root.add_sizer(&actions, 0, SizerFlag::All | SizerFlag::Expand, 8);
    dialog.set_sizer(root, true);
    dialog.set_escape_id(wxdragon::ID_CANCEL);

    let kpdb_for_add = Rc::clone(&kpdb);
    let scroll_for_add = scroll;
    let selected_for_add = Rc::clone(&selected);
    let dialog_for_add = dialog;
    let kpdb_for_add_icon = Rc::clone(&kpdb);
    let selected_label_for_add = selected_label;
    let buttons_for_add = Rc::clone(&buttons);
    let add_custom_for_add = Rc::clone(&add_custom);
    add.on_click(move |_| {
        let picker = FileDialog::builder(&dialog).with_message("Choose an image").with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist).with_wildcard("Image files (*.png;*.jpg;*.jpeg;*.gif;*.bmp;*.ico;*.svg;*.webp)|*.png;*.jpg;*.jpeg;*.gif;*.bmp;*.ico;*.svg;*.webp|All files (*.*)|*.*").build();
        if picker.show_modal() != wxdragon::ID_OK { return; }
        let Some(path) = picker.get_path() else { return; };
        let Ok(data) = fs::read(&path) else { return; };
        let Ok(image) = crate::favicon::image_from_bytes(&data) else { return; };
        let mut png = std::io::Cursor::new(Vec::new());
        if image.write_to(&mut png, image::ImageFormat::Png).is_err() { return; }
        let Some(uuid) = kpdb_for_add.borrow_mut().as_mut().and_then(|db| db.add_custom_icon(png.into_inner(), path).ok()) else { return; };
        let Some((data, name)) = kpdb_for_add
            .borrow()
            .as_ref()
            .and_then(|db| db.db.as_ref())
            .and_then(|db| db.meta.custom_icon(uuid))
            .map(|icon| (icon.data.clone(), icon.name().map(str::to_owned)))
        else {
            return;
        };
        if !buttons_for_add.borrow().iter().any(|(_, icon)| *icon == Icon::Custom(uuid)) {
            add_custom_for_add(uuid, &data, name.as_deref());
        }
        let icon = Icon::Custom(uuid);
        selected_for_add.set(icon);
        for (button, candidate_icon) in buttons_for_add.borrow().iter() {
            button.set_background_color(if *candidate_icon == icon {
                wxdragon::Colour::rgb(198, 224, 180)
            } else {
                wxdragon::Colour::rgb(255, 255, 255)
            });
        }
        set_dialog_icon(&dialog_for_add, &icon, &kpdb_for_add_icon);
        selected_label_for_add.set_label(&format!("Selected custom icon {uuid}"));
        scroll_for_add.layout();
    });
    let kpdb_for_delete = Rc::clone(&kpdb);
    let selected_for_delete = Rc::clone(&selected);
    let buttons_for_delete = Rc::clone(&buttons);
    let scroll_for_delete = scroll;
    let dialog_for_delete = dialog;
    let kpdb_for_delete_icon = Rc::clone(&kpdb);
    delete.on_click(move |_| {
        let Icon::Custom(uuid) = selected_for_delete.get() else {
            return;
        };
        let is_used = kpdb_for_delete
            .borrow()
            .as_ref()
            .and_then(|db| db.custom_icon_is_used(uuid).ok())
            .unwrap_or(false);
        if is_used {
            let confirmation = MessageDialog::builder(
                &dialog_for_delete,
                "This icon is still used by one or more entries or groups. Delete it anyway?",
                "Delete custom icon",
            )
            .with_style(MessageDialogStyle::YesNo | MessageDialogStyle::IconWarning)
            .build();
            if confirmation.show_modal() != wxdragon::ID_YES {
                return;
            }
        }
        let Ok(true) = kpdb_for_delete
            .borrow_mut()
            .as_mut()
            .ok_or(())
            .and_then(|db| db.remove_custom_icon(uuid).map_err(|_| ()))
        else {
            return;
        };
        if let Some((button, _)) = buttons_for_delete.borrow().iter().find(|(_, icon)| *icon == Icon::Custom(uuid)) {
            button.show(false);
        }
        buttons_for_delete.borrow_mut().retain(|(_, icon)| *icon != Icon::Custom(uuid));
        scroll_for_delete.layout();
        let icon = Icon::BuiltIn(IconId::KEY);
        selected_for_delete.set(icon);
        set_dialog_icon(&dialog_for_delete, &icon, &kpdb_for_delete_icon);
        selected_label.set_label(&format!("Selected icon {icon:?}"));
    });
    let kpdb_for_clear = Rc::clone(&kpdb);
    let buttons_for_clear = Rc::clone(&buttons);
    let selected_for_clear = Rc::clone(&selected);
    let dialog_for_clear = dialog;
    let kpdb_for_clear_icon = Rc::clone(&kpdb);
    let scroll_for_clear = scroll;
    clear_unused.on_click(move |_| {
        let Ok(removed) = kpdb_for_clear
            .borrow_mut()
            .as_mut()
            .ok_or(())
            .and_then(|db| db.purge_unused_custom_icons().map_err(|_| ()))
        else {
            return;
        };
        if removed == 0 {
            return;
        }
        let remaining = kpdb_for_clear
            .borrow()
            .as_ref()
            .and_then(|db| db.db.as_ref())
            .map(|db| db.meta.custom_icons().map(|(uuid, _)| *uuid).collect::<Vec<_>>())
            .unwrap_or_default();
        buttons_for_clear.borrow().iter().for_each(|(button, icon)| {
            if let Icon::Custom(uuid) = icon {
                button.show(remaining.contains(uuid));
            }
        });
        buttons_for_clear.borrow_mut().retain(|(_, icon)| match icon {
            Icon::BuiltIn(_) => true,
            Icon::Custom(uuid) => remaining.contains(uuid),
        });
        if let Icon::Custom(uuid) = selected_for_clear.get()
            && !remaining.contains(&uuid)
        {
            let icon = Icon::BuiltIn(IconId::KEY);
            selected_for_clear.set(icon);
            set_dialog_icon(&dialog_for_clear, &icon, &kpdb_for_clear_icon);
            selected_label.set_label(&format!("Selected icon {icon:?}"));
        }
        scroll_for_clear.layout();
    });
    let kpdb_for_more = Rc::clone(&kpdb);
    let selected_for_more = Rc::clone(&selected);
    more.on_click(move |_| {
        let message = match selected_for_more.get() {
            Icon::BuiltIn(id) => format!("Built-in icon {id}"),
            Icon::Custom(uuid) => kpdb_for_more
                .borrow()
                .as_ref()
                .and_then(|db| db.db.as_ref())
                .and_then(|db| db.meta.custom_icon(uuid))
                .and_then(|icon| icon.name())
                .unwrap_or("Custom icon")
                .to_owned(),
        };
        MessageDialog::builder(&dialog, &message, "Icon details")
            .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
            .build()
            .show_modal();
    });
    let selected_for_ok = Rc::clone(&selected);
    let dialog_for_ok = dialog;
    ok.on_click(move |_| dialog_for_ok.end_modal(wxdragon::ID_OK));
    cancel.on_click(move |_| dialog.end_modal(wxdragon::ID_CANCEL));
    for (button, icon) in buttons.borrow().iter() {
        button.set_background_color(if *icon == current {
            selected_background
        } else {
            default_background
        });
    }
    selected_label.set_label(&format!("Selected icon {current:?}"));
    dialog.center();
    if let Some((button, _)) = buttons.borrow().iter().find(|(_, icon)| *icon == current) {
        button.set_focus();
    }
    let result = dialog.show_modal();
    let value = selected_for_ok.get();
    dialog.destroy();
    (result == wxdragon::ID_OK).then_some(value)
}

#[allow(clippy::too_many_arguments)]
fn bind_icon_button(
    button: &Button,
    icon: Icon,
    buttons: &Rc<RefCell<Vec<(Button, Icon)>>>,
    selected: &Rc<Cell<Icon>>,
    dialog: Dialog,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    label: StaticText,
    selected_background: wxdragon::Colour,
    default_background: wxdragon::Colour,
) {
    let buttons = Rc::clone(buttons);
    let selected = Rc::clone(selected);
    let kpdb = Rc::clone(kpdb);
    button.on_click(move |_| {
        selected.set(icon);
        set_dialog_icon(&dialog, &icon, &kpdb);
        for (candidate, candidate_icon) in buttons.borrow().iter() {
            candidate.set_background_color(if *candidate_icon == icon {
                selected_background
            } else {
                default_background
            });
        }
        label.set_label(&format!("Selected icon {icon:?}"));
    });
}

fn set_dialog_icon(dialog: &Dialog, icon: &Icon, kpdb: &Rc<RefCell<Option<KpDb>>>) {
    let bitmap = match icon {
        Icon::BuiltIn(icon_id) => crate::icon_cache::icon_for_emoji(&icon_id.to_string(), 32),
        Icon::Custom(uuid) => kpdb
            .borrow()
            .as_ref()
            .and_then(|db| db.db.as_ref())
            .and_then(|db| db.meta.custom_icon(*uuid))
            .and_then(|custom_icon| bitmap_for_icon(&custom_icon.data, 32)),
    };
    if let Some(bitmap) = bitmap {
        dialog.set_icon(&bitmap);
    }
}
