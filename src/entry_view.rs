use crate::add_detail_row;
use keepass_ng::db::{Entry, Node};
use std::{cell::Cell, rc::Rc};
use wxdragon::{
    BoxSizer, Button, ButtonEvents, FlexGridSizer, HyperlinkCtrl, ListColumnFormat, ListCtrl, ListCtrlStyle, MessageDialog,
    MessageDialogStyle, Notebook, Orientation, Panel, Size, SizerFlag, StaticText, TextCtrl, TextCtrlStyle, WxWidget,
};

pub fn build_entry_view(parent: &Panel, entry: &Entry) {
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let title_text = entry.get_title().filter(|title| !title.trim().is_empty()).unwrap_or("(no title)");
    let title = StaticText::builder(parent).with_label(title_text).build();
    let edit_button = Button::builder(parent).with_label("Edit").with_size(Size::new(85, 34)).build();
    let parent_for_edit = *parent;
    edit_button.on_click(move |_| {
        MessageDialog::builder(&parent_for_edit, "Entry editing is not implemented yet.", "Edit Entry")
            .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
            .build()
            .show_modal();
    });
    let header_sizer = BoxSizer::builder(Orientation::Horizontal).build();
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
    let password_controls = BoxSizer::builder(Orientation::Horizontal).build();
    let password = entry.get_password().unwrap_or("").to_owned();
    let password_value = TextCtrl::builder(&general_page)
        .with_value("******")
        .with_style(TextCtrlStyle::ReadOnly)
        .with_size(Size::new(240, 34))
        .build();
    let password_toggle = Button::builder(&general_page)
        .with_label("Show")
        .with_size(Size::new(85, 34))
        .build();
    let password_visible = Rc::new(Cell::new(false));
    let password_visible_for_toggle = Rc::clone(&password_visible);
    let password_value_for_toggle = password_value;
    let password_toggle_for_toggle = password_toggle;
    password_toggle.on_click(move |_| {
        let visible = !password_visible_for_toggle.get();
        password_visible_for_toggle.set(visible);
        password_value_for_toggle.set_value(if visible { &password } else { "******" });
        password_toggle_for_toggle.set_label(if visible { "Hide" } else { "Show" });
    });
    password_controls.add(&password_toggle, 0, SizerFlag::AlignCenterVertical, 4);
    password_controls.add(&password_value, 0, SizerFlag::AlignCenterVertical, 0);
    general_grid.add(&password_label, 0, SizerFlag::All | SizerFlag::AlignCenterVertical, 4);
    general_grid.add_sizer(&password_controls, 1, SizerFlag::All | SizerFlag::Expand, 4);
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
}
