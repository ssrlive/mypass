use crate::add_detail_row;
use crate::favicon::{FaviconDownloader, image_from_bytes};
use crate::icon_cache::icon_for_emoji;
use crate::icon_picker::show_icon_picker;
use crate::keepass::KpDb;
use chrono::{Datelike, Duration, Local, Months, NaiveDate, NaiveDateTime, Timelike};
use keepass_ng::db::{AutoType, Entry, Icon, Node, NodePtr, with_node, with_node_mut};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
};
use wxdragon::{
    Bitmap, BoxSizer, Button, ButtonEvents, CheckBox, Choice, DatePickerCtrl, DatePickerCtrlStyle, Dialog, FlexGridSizer, Frame,
    HyperlinkCtrl, ListColumnFormat, ListCtrl, ListCtrlStyle, MessageDialog, MessageDialogStyle, Notebook, Orientation, Panel, Size,
    SizerFlag, StaticBitmap, StaticText, TextCtrl, TextCtrlStyle, TimePickerCtrl, Timer, WindowEvents, WxWidget,
};

pub fn build_entry_view(parent: &Panel, frame: Frame, node: &NodePtr, refresh: Rc<dyn Fn()>, kpdb: Rc<RefCell<Option<KpDb>>>) {
    let Some(entry) = with_node::<Entry, _, _>(node, |entry| entry.clone()) else {
        return;
    };
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let title_text = entry.get_title().filter(|title| !title.trim().is_empty()).unwrap_or("(no title)");
    let title = StaticText::builder(parent).with_label(title_text).build();
    let edit_button = Button::builder(parent).with_label("Edit").with_size(Size::new(85, 34)).build();
    let node_for_edit = node.clone();
    let refresh_after_edit = Rc::clone(&refresh);
    let kpdb_for_edit = Rc::clone(&kpdb);
    edit_button.on_click(move |_| {
        if show_entry_editor(&frame, &node_for_edit, Rc::clone(&kpdb_for_edit)) == wxdragon::ID_OK {
            refresh_after_edit();
        }
    });
    let header_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    match entry.get_icon() {
        Icon::BuiltIn(icon_id) => {
            if let Some(bitmap) = icon_for_emoji(&icon_id.to_string(), 28) {
                let title_icon = StaticBitmap::builder(parent)
                    .with_bitmap(Some(bitmap))
                    .with_size(Size::new(32, 32))
                    .build();
                title_icon.set_tooltip(&format!("Built-in icon {icon_id}"));
                header_sizer.add(&title_icon, 0, SizerFlag::AlignCenterVertical | SizerFlag::Right, 8);
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
                let title_icon = StaticBitmap::builder(parent)
                    .with_bitmap(Some(bitmap))
                    .with_size(Size::new(32, 32))
                    .build();
                title_icon.set_tooltip(custom_icon.name().unwrap_or("Custom icon"));
                header_sizer.add(&title_icon, 0, SizerFlag::AlignCenterVertical | SizerFlag::Right, 8);
            }
        }
    }
    header_sizer.add(&title, 1, SizerFlag::Expand, 0);
    let header_spacer = StaticText::builder(parent).with_label("").build();
    header_sizer.add(&header_spacer, 1, SizerFlag::Expand, 0);
    header_sizer.add(&edit_button, 0, SizerFlag::AlignCenterVertical, 0);
    sizer.add_sizer(&header_sizer, 0, SizerFlag::All | SizerFlag::Expand, 12);

    let notebook = Notebook::builder(parent).build();

    let general_page = Panel::builder(&notebook).build();
    let general_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let general_grid = FlexGridSizer::builder(0, 2).with_vgap(8).with_hgap(16).build();
    add_detail_row(&general_grid, &general_page, "Username", entry.get_username().unwrap_or(""));
    let password_label = StaticText::builder(&general_page).with_label("Password").build();
    let password = entry.get_password().unwrap_or("").to_owned();
    let password_panel = Panel::builder(&general_page).build();
    let password_value = TextCtrl::builder(&password_panel)
        .with_value(&password)
        .with_style(TextCtrlStyle::Password | TextCtrlStyle::ReadOnly)
        .with_size(Size::new(240, 34))
        .build();
    let password_visible_value = TextCtrl::builder(&password_panel)
        .with_value(&password)
        .with_style(TextCtrlStyle::ReadOnly)
        .with_size(Size::new(240, 34))
        .build();
    password_visible_value.show(false);
    let password_toggle = Button::builder(&password_panel).with_size(Size::new(38, 34)).build();
    if let Some(bitmap) = icon_for_emoji("👁", 20) {
        password_toggle.set_bitmap_label(&bitmap);
    }
    password_toggle.set_tooltip("Show or hide password");
    let password_visible = Rc::new(Cell::new(false));
    let password_visible_for_toggle = Rc::clone(&password_visible);
    let password_value_for_toggle = password_value;
    let password_visible_value_for_toggle = password_visible_value;
    let password_panel_for_toggle = password_panel;
    password_toggle.on_click(move |_| {
        let visible = !password_visible_for_toggle.get();
        password_visible_for_toggle.set(visible);
        if visible {
            password_visible_value_for_toggle.set_value(&password_value_for_toggle.get_value());
            password_value_for_toggle.show(false);
            password_visible_value_for_toggle.show(true);
        } else {
            password_value_for_toggle.set_value(&password_visible_value_for_toggle.get_value());
            password_visible_value_for_toggle.show(false);
            password_value_for_toggle.show(true);
        }
        password_panel_for_toggle.layout();
    });
    let password_controls = BoxSizer::builder(Orientation::Horizontal).build();
    password_controls.add(&password_value, 1, SizerFlag::All | SizerFlag::Expand, 0);
    password_controls.add(&password_visible_value, 1, SizerFlag::All | SizerFlag::Expand, 0);
    password_controls.add(&password_toggle, 0, SizerFlag::All, 4);
    password_panel.set_sizer(password_controls, true);
    general_grid.add(&password_label, 0, SizerFlag::All | SizerFlag::AlignCenterVertical, 4);
    password_panel.set_min_size(Size::new(282, 34));
    general_grid.add(&password_panel, 1, SizerFlag::All, 4);
    let url_label = StaticText::builder(&general_page).with_label("URL").build();
    general_grid.add(&url_label, 0, SizerFlag::All | SizerFlag::AlignCenterVertical, 4);
    if let Some(url) = entry.get_url().filter(|url| !url.is_empty()) {
        let url_link = HyperlinkCtrl::builder(&general_page).with_label(url).with_url(url).build();
        general_grid.add(&url_link, 1, SizerFlag::AlignLeft | SizerFlag::AlignCenterVertical, 4);
    } else {
        let empty_url = StaticText::builder(&general_page).with_label("").build();
        general_grid.add(&empty_url, 1, SizerFlag::AlignCenterVertical, 4);
    }
    let expiry = entry
        .get_times()
        .get_expiry_time()
        .filter(|_| entry.get_times().get_expires())
        .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "Never".to_string());
    let last_modified = entry
        .get_times()
        .get_last_modification()
        .map(|time| time.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    add_detail_row(&general_grid, &general_page, "Expires", &expiry);
    add_detail_row(&general_grid, &general_page, "Last Modified", &last_modified);
    add_detail_row(&general_grid, &general_page, "Tags", &entry.get_tags().join(", "));
    add_detail_row(&general_grid, &general_page, "Notes", entry.get_notes().unwrap_or(""));
    general_sizer.add_sizer(&general_grid, 0, SizerFlag::All | SizerFlag::Expand, 12);
    general_page.set_sizer(general_sizer, true);

    let advanced_page = Panel::builder(&notebook).build();
    let advanced_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let attributes_panel = Panel::builder(&advanced_page).build();
    let attributes_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let attributes_title = StaticText::builder(&attributes_panel).with_label("Attributes").build();
    attributes_sizer.add(&attributes_title, 0, SizerFlag::All | SizerFlag::Expand, 4);
    let mut attributes = entry.additional_attributes();
    attributes.sort_by(|left, right| left.0.cmp(&right.0));
    let attributes_text = attributes
        .iter()
        .map(|(name, value)| format!("{}\n{}\n\n", name, value))
        .collect::<String>();
    let attributes_text = if attributes_text.is_empty() {
        "No additional attributes".to_string()
    } else {
        attributes_text
    };
    let attributes_value = TextCtrl::builder(&attributes_panel)
        .with_value(&attributes_text)
        .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::WordWrap)
        .build();
    attributes_sizer.add(&attributes_value, 1, SizerFlag::All | SizerFlag::Expand, 4);
    attributes_panel.set_sizer(attributes_sizer, true);

    let attachments_panel = Panel::builder(&advanced_page).build();
    let attachments_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let attachments_title = StaticText::builder(&attachments_panel).with_label("Attachments").build();
    attachments_sizer.add(&attachments_title, 0, SizerFlag::All | SizerFlag::Expand, 4);
    let attachments = ListCtrl::builder(&attachments_panel)
        .with_style(ListCtrlStyle::Report | ListCtrlStyle::SingleSel | ListCtrlStyle::VRules | ListCtrlStyle::HRules)
        .build();
    attachments.insert_column(0, "Name", ListColumnFormat::Left, 180);
    attachments.insert_column(1, "Size", ListColumnFormat::Right, -1);
    let mut attachment_names: Vec<_> = entry.attachments.keys().collect();
    attachment_names.sort();
    for (index, name) in attachment_names.iter().enumerate() {
        let row = index as i64;
        let Some(attachment) = entry.attachments.get(*name) else {
            continue;
        };
        if attachments.insert_item(row, name, None) < 0 {
            continue;
        }
        attachments.set_item_text_by_column(row, 1, &format!("{} bytes", attachment.data.get().len()));
    }
    attachments_sizer.add(&attachments, 1, SizerFlag::All | SizerFlag::Expand, 4);
    attachments_panel.set_sizer(attachments_sizer, true);

    advanced_sizer.add(&attributes_panel, 1, SizerFlag::All | SizerFlag::Expand, 4);
    advanced_sizer.add(&attachments_panel, 1, SizerFlag::All | SizerFlag::Expand, 4);
    advanced_page.set_sizer(advanced_sizer, true);

    let autotype_page = Panel::builder(&notebook).build();
    let autotype_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let default_sequence = entry
        .get_autotype()
        .and_then(|autotype| autotype.default_sequence.as_deref())
        .unwrap_or("");
    let default_sequence_label = StaticText::builder(&autotype_page)
        .with_label(&format!("Default Sequence  {}", default_sequence))
        .build();
    autotype_sizer.add(&default_sequence_label, 0, SizerFlag::All | SizerFlag::Expand, 8);
    let autotype_list = ListCtrl::builder(&autotype_page)
        .with_style(ListCtrlStyle::Report | ListCtrlStyle::SingleSel | ListCtrlStyle::VRules | ListCtrlStyle::HRules)
        .build();
    autotype_list.insert_column(0, "Window", ListColumnFormat::Left, 260);
    autotype_list.insert_column(1, "Sequence", ListColumnFormat::Left, -1);
    if let Some(autotype) = entry.get_autotype() {
        for (index, association) in autotype.associations.iter().enumerate() {
            let row = index as i64;
            if autotype_list.insert_item(row, association.window.as_deref().unwrap_or(""), None) < 0 {
                continue;
            }
            autotype_list.set_item_text_by_column(row, 1, association.sequence.as_deref().unwrap_or(""));
        }
    }
    autotype_sizer.add(&autotype_list, 1, SizerFlag::All | SizerFlag::Expand, 4);
    autotype_page.set_sizer(autotype_sizer, true);

    notebook.add_page(&general_page, "General", true, None);
    notebook.add_page(&advanced_page, "Advanced", false, None);
    notebook.add_page(&autotype_page, "Autotype", false, None);
    sizer.add(&notebook, 1, SizerFlag::All | SizerFlag::Expand, 0);
    parent.set_sizer(sizer, true);
    parent.layout();
    notebook.set_focus();
    notebook.navigate(true);
}

pub fn show_entry_editor(parent: &dyn WxWidget, node: &NodePtr, kpdb: Rc<RefCell<Option<KpDb>>>) -> wxdragon::Id {
    let Some(entry) = with_node::<Entry, _, _>(node, |entry| entry.clone()) else {
        return wxdragon::ID_CANCEL;
    };

    let dialog = Dialog::builder(parent, "Edit entry").with_size(760, 580).build();
    let dialog_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let notebook = Notebook::builder(&dialog).build();

    let entry_page = Panel::builder(&notebook).build();
    let entry_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let entry_grid = FlexGridSizer::builder(0, 2).with_vgap(8).with_hgap(12).build();
    entry_grid.add_growable_col(1, 1);
    let title = TextCtrl::builder(&entry_page).with_value(entry.get_title().unwrap_or("")).build();
    let title_icon_button = Button::builder(&entry_page).with_size(Size::new(38, 34)).build();
    set_icon_button_bitmap(&title_icon_button, &entry.get_icon(), &kpdb);
    title_icon_button.set_tooltip("Choose icon");
    let username = TextCtrl::builder(&entry_page)
        .with_value(entry.get_username().unwrap_or(""))
        .build();
    let password_panel = Panel::builder(&entry_page).build();
    let password_value = entry.get_password().unwrap_or("").to_owned();
    let password = TextCtrl::builder(&password_panel)
        .with_value(entry.get_password().unwrap_or(""))
        .with_style(TextCtrlStyle::Password)
        .build();
    let password_visible = TextCtrl::builder(&password_panel).with_value(&password_value).build();
    password_visible.show(false);
    let password_toggle = Button::builder(&password_panel).with_size(Size::new(38, 34)).build();
    if let Some(bitmap) = icon_for_emoji("👁", 20) {
        password_toggle.set_bitmap_label(&bitmap);
    }
    password_toggle.set_tooltip("Show or hide password");
    let password_for_toggle = password;
    let password_visible_for_toggle = password_visible;
    let password_panel_for_toggle = password_panel;
    let password_is_visible = Rc::new(Cell::new(false));
    let password_is_visible_for_toggle = Rc::clone(&password_is_visible);
    password_toggle.on_click(move |_| {
        let is_visible = !password_is_visible_for_toggle.get();
        if is_visible {
            password_visible_for_toggle.set_value(&password_for_toggle.get_value());
            password_for_toggle.show(false);
            password_visible_for_toggle.show(true);
        } else {
            password_for_toggle.set_value(&password_visible_for_toggle.get_value());
            password_visible_for_toggle.show(false);
            password_for_toggle.show(true);
        }
        password_panel_for_toggle.layout();
        password_is_visible_for_toggle.set(is_visible);
    });
    let password_controls = BoxSizer::builder(Orientation::Horizontal).build();
    password_controls.add(&password, 1, SizerFlag::All | SizerFlag::Expand, 0);
    password_controls.add(&password_visible, 1, SizerFlag::All | SizerFlag::Expand, 0);
    password_controls.add(&password_toggle, 0, SizerFlag::All, 4);
    password_panel.set_sizer(password_controls, true);
    let url = TextCtrl::builder(&entry_page).with_value(entry.get_url().unwrap_or("")).build();
    let download_favicon = Button::builder(&entry_page).with_size(Size::new(38, 34)).build();
    if let Some(bitmap) = icon_for_emoji("⬇️", 20) {
        download_favicon.set_bitmap_label(&bitmap);
    }
    download_favicon.set_tooltip("Download favicon from URL");
    let tags = TextCtrl::builder(&entry_page).with_value(&entry.get_tags().join(", ")).build();
    let expires = CheckBox::builder(&entry_page)
        .with_label("Expires")
        .with_value(entry.get_times().get_expires())
        .build();
    let expiry_datetime = if entry.get_times().get_expires() {
        entry
            .get_times()
            .get_expiry_time()
            .map(datetime_to_wx)
            .unwrap_or_else(wxdragon::DateTime::now)
    } else {
        wxdragon::DateTime::now()
    };
    let expiry_date = DatePickerCtrl::builder(&entry_page)
        .with_style(DatePickerCtrlStyle::Dropdown | DatePickerCtrlStyle::ShowCentury)
        .with_value(Some(expiry_datetime.clone()))
        .build();
    let expiry_time = TimePickerCtrl::builder(&entry_page).with_value(Some(expiry_datetime)).build();
    let presets = Choice::builder(&entry_page)
        .with_choices(vec![
            "12 hours".to_string(),
            "24 hours".to_string(),
            "1 week".to_string(),
            "2 weeks".to_string(),
            "3 weeks".to_string(),
            "1 month".to_string(),
            "2 months".to_string(),
            "3 months".to_string(),
            "6 months".to_string(),
            "1 year".to_string(),
            "2 years".to_string(),
            "3 years".to_string(),
        ])
        .build();
    let expiry_controls_enabled = expires.get_value();
    expiry_date.enable(expiry_controls_enabled);
    expiry_time.enable(expiry_controls_enabled);
    presets.enable(expiry_controls_enabled);
    let expiry_date_for_toggle = expiry_date;
    let expiry_time_for_toggle = expiry_time;
    let presets_for_toggle = presets;
    expires.on_toggled(move |event| {
        let enabled = event.is_checked();
        expiry_date_for_toggle.enable(enabled);
        expiry_time_for_toggle.enable(enabled);
        presets_for_toggle.enable(enabled);
    });
    let expiry_date_for_preset = expiry_date;
    let expiry_time_for_preset = expiry_time;
    presets.on_selection_changed(move |event| {
        let Some(selection) = event.get_selection() else {
            return;
        };
        let now = Local::now().naive_local();
        let expiry = match selection {
            0 => now.checked_add_signed(Duration::hours(12)),
            1 => now.checked_add_signed(Duration::hours(24)),
            2 => now.checked_add_signed(Duration::weeks(1)),
            3 => now.checked_add_signed(Duration::weeks(2)),
            4 => now.checked_add_signed(Duration::weeks(3)),
            5 => now.checked_add_months(Months::new(1)),
            6 => now.checked_add_months(Months::new(2)),
            7 => now.checked_add_months(Months::new(3)),
            8 => now.checked_add_months(Months::new(6)),
            9 => now.checked_add_months(Months::new(12)),
            10 => now.checked_add_months(Months::new(24)),
            11 => now.checked_add_months(Months::new(36)),
            _ => None,
        };
        if let Some(expiry) = expiry {
            let wx_expiry = datetime_to_wx(expiry);
            expiry_date_for_preset.set_value(&wx_expiry);
            expiry_time_for_preset.set_value(&wx_expiry);
        }
    });
    let notes = TextCtrl::builder(&entry_page)
        .with_value(entry.get_notes().unwrap_or(""))
        .with_style(TextCtrlStyle::MultiLine)
        .with_size(Size::new(-1, 150))
        .build();
    let title_controls = BoxSizer::builder(Orientation::Horizontal).build();
    title_controls.add(&title, 1, SizerFlag::All | SizerFlag::Expand, 0);
    title_controls.add(&title_icon_button, 0, SizerFlag::All, 4);
    entry_grid.add(&StaticText::builder(&entry_page).with_label("Title").build(), 0, SizerFlag::All, 4);
    entry_grid.add_sizer(&title_controls, 1, SizerFlag::All | SizerFlag::Expand, 4);
    entry_grid.add(
        &StaticText::builder(&entry_page).with_label("Username").build(),
        0,
        SizerFlag::All,
        4,
    );
    entry_grid.add(&username, 1, SizerFlag::All | SizerFlag::Expand, 4);
    let password_label = StaticText::builder(&entry_page).with_label("Password").build();
    entry_grid.add(&password_label, 0, SizerFlag::All, 4);
    entry_grid.add(&password_panel, 1, SizerFlag::All | SizerFlag::Expand, 4);
    let url_label = StaticText::builder(&entry_page).with_label("URL").build();
    entry_grid.add(&url_label, 0, SizerFlag::All, 4);
    let url_controls = BoxSizer::builder(Orientation::Horizontal).build();
    url_controls.add(&url, 1, SizerFlag::All | SizerFlag::Expand, 0);
    url_controls.add(&download_favicon, 0, SizerFlag::All, 4);
    entry_grid.add_sizer(&url_controls, 1, SizerFlag::All | SizerFlag::Expand, 4);
    let tags_label = StaticText::builder(&entry_page).with_label("Tags").build();
    entry_grid.add(&tags_label, 0, SizerFlag::All, 4);
    entry_grid.add(&tags, 1, SizerFlag::All | SizerFlag::Expand, 4);
    let expires_label = StaticText::builder(&entry_page).with_label("Expires").build();
    entry_grid.add(&expires_label, 0, SizerFlag::All, 4);
    let expiry_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    expiry_sizer.add(&expires, 0, SizerFlag::All, 0);
    expiry_sizer.add(&expiry_date, 1, SizerFlag::All | SizerFlag::Expand, 4);
    expiry_sizer.add(&expiry_time, 0, SizerFlag::All, 4);
    expiry_sizer.add(&presets, 0, SizerFlag::All, 4);
    entry_grid.add_sizer(&expiry_sizer, 1, SizerFlag::All | SizerFlag::Expand, 4);
    let notes_label = StaticText::builder(&entry_page).with_label("Notes").build();
    entry_grid.add(&notes_label, 0, SizerFlag::All, 4);
    entry_grid.add(&notes, 1, SizerFlag::All | SizerFlag::Expand, 4);
    entry_sizer.add_sizer(&entry_grid, 1, SizerFlag::All | SizerFlag::Expand, 12);
    entry_page.set_sizer(entry_sizer, true);

    let advanced_page = Panel::builder(&notebook).build();
    let advanced_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let attributes = TextCtrl::builder(&advanced_page)
        .with_value(
            &entry
                .additional_attributes()
                .iter()
                .map(|(name, value)| format!("{}\n{}\n\n", name, value))
                .collect::<String>(),
        )
        .with_style(TextCtrlStyle::MultiLine)
        .build();
    advanced_sizer.add(
        &StaticText::builder(&advanced_page).with_label("Additional attributes").build(),
        0,
        SizerFlag::All,
        4,
    );
    advanced_sizer.add(&attributes, 1, SizerFlag::All | SizerFlag::Expand, 4);
    let attachments = ListCtrl::builder(&advanced_page)
        .with_style(ListCtrlStyle::Report | ListCtrlStyle::VRules | ListCtrlStyle::HRules)
        .build();
    attachments.insert_column(0, "Name", ListColumnFormat::Left, 240);
    attachments.insert_column(1, "Size", ListColumnFormat::Right, -1);
    for (index, name) in entry.attachments.keys().enumerate() {
        let row = index as i64;
        if let Some(attachment) = entry.attachments.get(name)
            && attachments.insert_item(row, name, None) >= 0
        {
            attachments.set_item_text_by_column(row, 1, &format!("{} bytes", attachment.data.get().len()));
        }
    }
    advanced_sizer.add(
        &StaticText::builder(&advanced_page).with_label("Attachments").build(),
        0,
        SizerFlag::All,
        4,
    );
    advanced_sizer.add(&attachments, 1, SizerFlag::All | SizerFlag::Expand, 4);
    advanced_page.set_sizer(advanced_sizer, true);

    let selected_icon = Rc::new(Cell::new(entry.get_icon()));
    let title_icon_button_for_picker = title_icon_button;
    let selected_icon_for_picker = Rc::clone(&selected_icon);
    let kpdb_for_picker = Rc::clone(&kpdb);
    title_icon_button.on_click(move |_| {
        if let Some(icon) = show_icon_picker(&dialog, Rc::clone(&kpdb_for_picker), selected_icon_for_picker.get()) {
            selected_icon_for_picker.set(icon);
            set_icon_button_bitmap(&title_icon_button_for_picker, &icon, &kpdb_for_picker);
        }
    });
    let url_for_download = url;
    #[allow(clippy::type_complexity)]
    let download_result: Arc<Mutex<Option<Result<(Vec<u8>, String), String>>>> = Arc::new(Mutex::new(None));
    let download_timer = Rc::new(Timer::new(&entry_page));
    let download_timer_for_tick = Rc::clone(&download_timer);
    let download_timer_to_stop = Rc::clone(&download_timer);
    let download_result_for_tick = Arc::clone(&download_result);
    let download_button_for_tick = download_favicon;
    let parent_for_download = entry_page;
    let kpdb_for_download = Rc::clone(&kpdb);
    let selected_icon_for_download = Rc::clone(&selected_icon);
    download_timer_for_tick.on_tick(move |_| {
        let Some(result) = download_result_for_tick.lock().unwrap().take() else {
            return;
        };
        download_timer_to_stop.stop();
        download_button_for_tick.enable(true);
        match result {
            Ok((png_bytes, source_url)) => {
                let Some(uuid) = kpdb_for_download
                    .borrow_mut()
                    .as_mut()
                    .and_then(|db| db.add_custom_icon(png_bytes, source_url).ok())
                else {
                    MessageDialog::builder(&parent_for_download, "No database loaded", "Download favicon")
                        .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
                        .build()
                        .show_modal();
                    return;
                };
                selected_icon_for_download.set(Icon::Custom(uuid));
                set_icon_button_bitmap(&title_icon_button, &Icon::Custom(uuid), &kpdb_for_download);
            }
            Err(error) => {
                MessageDialog::builder(&parent_for_download, &error, "Download favicon")
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
                    .build()
                    .show_modal();
            }
        }
    });
    let download_timer_for_destroy = Rc::clone(&download_timer);
    entry_page.on_destroy(move |_| download_timer_for_destroy.stop());
    let download_timer_for_click = Rc::clone(&download_timer);
    let download_result_for_worker = Arc::clone(&download_result);
    download_favicon.on_click(move |_| {
        download_favicon.enable(false);
        let website_url = url_for_download.get_value();
        let download_result_for_worker = Arc::clone(&download_result_for_worker);
        std::thread::spawn(move || {
            let result = FaviconDownloader::new()
                .and_then(|downloader| downloader.download(&website_url))
                .and_then(|favicon| {
                    favicon.ok_or_else(|| "No favicon found for this URL".into()).and_then(|favicon| {
                        let source_url = favicon.source_url.to_string();
                        favicon.to_png_bytes().map(|png_bytes| (png_bytes, source_url))
                    })
                })
                .map_err(|error| error.to_string());
            *download_result_for_worker.lock().unwrap() = Some(result);
            wxdragon::call_after(Box::new(|| {}));
        });
        download_timer_for_click.start(100, false);
    });

    let autotype_page = Panel::builder(&notebook).build();
    let autotype_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let autotype = entry.get_autotype();
    let autotype_enabled = CheckBox::builder(&autotype_page)
        .with_label("Enable Auto-Type for this entry")
        .with_value(autotype.map(|value| value.enabled).unwrap_or(false))
        .build();
    let default_sequence = TextCtrl::builder(&autotype_page)
        .with_value(autotype.and_then(|value| value.default_sequence.as_deref()).unwrap_or(""))
        .build();
    autotype_sizer.add(&autotype_enabled, 0, SizerFlag::All, 4);
    autotype_sizer.add(
        &StaticText::builder(&autotype_page).with_label("Default sequence").build(),
        0,
        SizerFlag::All,
        4,
    );
    autotype_sizer.add(&default_sequence, 0, SizerFlag::All | SizerFlag::Expand, 4);
    autotype_sizer.add(
        &StaticText::builder(&autotype_page).with_label("Window associations").build(),
        0,
        SizerFlag::All,
        8,
    );
    let associations = ListCtrl::builder(&autotype_page)
        .with_style(ListCtrlStyle::Report | ListCtrlStyle::VRules | ListCtrlStyle::HRules)
        .build();
    associations.insert_column(0, "Window", ListColumnFormat::Left, 260);
    associations.insert_column(1, "Sequence", ListColumnFormat::Left, -1);
    if let Some(autotype) = autotype {
        for (index, association) in autotype.associations.iter().enumerate() {
            let row = index as i64;
            if associations.insert_item(row, association.window.as_deref().unwrap_or(""), None) >= 0 {
                associations.set_item_text_by_column(row, 1, association.sequence.as_deref().unwrap_or(""));
            }
        }
    }
    autotype_sizer.add(&associations, 1, SizerFlag::All | SizerFlag::Expand, 4);
    autotype_page.set_sizer(autotype_sizer, true);

    let properties_page = Panel::builder(&notebook).build();
    let properties_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let properties_grid = FlexGridSizer::builder(0, 2).with_vgap(8).with_hgap(12).build();
    add_detail_row(
        &properties_grid,
        &properties_page,
        "Created",
        &format_option_time(entry.get_times().get_creation()),
    );
    add_detail_row(
        &properties_grid,
        &properties_page,
        "Modified",
        &format_option_time(entry.get_times().get_last_modification()),
    );
    add_detail_row(
        &properties_grid,
        &properties_page,
        "Last accessed",
        &format_option_time(entry.get_times().get_last_access()),
    );
    add_detail_row(&properties_grid, &properties_page, "UUID", &entry.get_uuid().to_string());
    add_detail_row(
        &properties_grid,
        &properties_page,
        "Usage count",
        &entry.get_times().get_usage_count().to_string(),
    );
    properties_sizer.add_sizer(&properties_grid, 0, SizerFlag::All | SizerFlag::Expand, 12);
    properties_page.set_sizer(properties_sizer, true);

    notebook.add_page(&entry_page, "Entry", true, None);
    notebook.add_page(&advanced_page, "Advanced", false, None);
    notebook.add_page(&autotype_page, "Auto-Type", false, None);
    notebook.add_page(&properties_page, "Properties", false, None);
    dialog_sizer.add(&notebook, 1, SizerFlag::All | SizerFlag::Expand, 8);

    let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let button_spacer = StaticText::builder(&dialog).with_label("").build();
    let cancel = Button::builder(&dialog).with_id(wxdragon::ID_CANCEL).with_label("Cancel").build();
    let ok = Button::builder(&dialog).with_label("OK").build();
    button_sizer.add(&button_spacer, 1, SizerFlag::Expand, 0);
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
        let selected_icon_value = selected_icon_for_ok.get();
        with_node_mut::<Entry, _, _>(&node_for_ok, |entry| {
            let title_value = title.get_value();
            let username_value = username.get_value();
            let password_value = if password_is_visible.get() {
                password_visible.get_value()
            } else {
                password.get_value()
            };
            let url_value = url.get_value();
            let notes_value = notes.get_value();
            entry.set_title(if title_value.trim().is_empty() { None } else { Some(&title_value) });
            entry.set_username(Some(&username_value));
            entry.set_password(Some(&password_value));
            entry.set_url(Some(&url_value));
            entry.set_notes(Some(&notes_value));
            entry.get_tags_mut().clear();
            entry.get_tags_mut().extend(
                tags.get_value()
                    .split(',')
                    .map(str::trim)
                    .filter(|tag| !tag.is_empty())
                    .map(str::to_owned),
            );
            let expires_value = expires.get_value();
            entry.get_times_mut().set_expires(expires_value);
            if expires_value {
                let selected_date = expiry_date.get_value();
                let selected_time = expiry_time.get_value();
                if let Some(expiry_date) =
                    NaiveDate::from_ymd_opt(selected_date.year(), selected_date.month() as u32, selected_date.day() as u32).and_then(
                        |date| {
                            date.and_hms_opt(
                                selected_time.hour() as u32,
                                selected_time.minute() as u32,
                                selected_time.second() as u32,
                            )
                        },
                    )
                {
                    entry.get_times_mut().set_expiry_time(Some(expiry_date));
                }
            } else {
                entry.get_times_mut().set_expiry_time(None);
            }
            let default_sequence_value = default_sequence.get_value();
            let auto_type = AutoType {
                enabled: autotype_enabled.get_value(),
                default_sequence: if default_sequence_value.trim().is_empty() {
                    None
                } else {
                    Some(default_sequence_value)
                },
                ..entry.get_autotype().cloned().unwrap_or_default()
            };
            entry.set_autotype(Some(auto_type));
            entry.set_icon(selected_icon_value);
            entry.update_history();
        });
        if let Some(db) = kpdb.borrow_mut().as_mut() {
            db.mark_data_changed();
        }
        dialog_for_ok.end_modal(wxdragon::ID_OK);
    });
    dialog.center();
    let res = dialog.show_modal();
    dialog.destroy();
    res
}

fn format_option_time<T: ToString>(time: Option<T>) -> String {
    time.map(|time| time.to_string()).unwrap_or_default()
}

pub(crate) fn bitmap_for_icon(data: &[u8], size: u32) -> Option<Bitmap> {
    let image = image_from_bytes(data).ok()?.thumbnail(size, size);
    let rgba = image.to_rgba8();
    Bitmap::from_rgba(rgba.as_raw(), rgba.width(), rgba.height())
}

pub(crate) fn bitmap_for_icon_fixed(data: &[u8], size: u32) -> Option<Bitmap> {
    let image = image_from_bytes(data)
        .ok()?
        .resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    let rgba = image.to_rgba8();
    Bitmap::from_rgba(rgba.as_raw(), rgba.width(), rgba.height())
}

pub(crate) fn set_icon_button_bitmap(button: &Button, icon: &Icon, kpdb: &Rc<RefCell<Option<KpDb>>>) {
    let bitmap = match icon {
        Icon::BuiltIn(icon_id) => icon_for_emoji(&icon_id.to_string(), 20),
        Icon::Custom(uuid) => kpdb
            .borrow()
            .as_ref()
            .and_then(|db| db.db.as_ref())
            .and_then(|db| db.meta.custom_icon(*uuid))
            .and_then(|icon| bitmap_for_icon(&icon.data, 20)),
    };
    if let Some(bitmap) = bitmap {
        button.set_bitmap_label(&bitmap);
    }
}

fn datetime_to_wx(time: NaiveDateTime) -> wxdragon::DateTime {
    wxdragon::DateTime::new(
        time.year(),
        time.month() as u16,
        time.day() as i16,
        time.hour() as i16,
        time.minute() as i16,
        time.second() as i16,
    )
}
