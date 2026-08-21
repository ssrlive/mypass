use crate::settings::{ProxyProtocol, ProxySettings, Settings};
use wxdragon::{
    BoxSizer, ButtonEvents, Choice, Dialog, FlexGridSizer, Notebook, Orientation, Panel, SizerFlag, StaticText, TextCtrl, TextCtrlStyle,
    WxWidget,
};

pub fn show(parent: &dyn WxWidget, settings: &mut Settings) -> bool {
    let dialog = Dialog::builder(parent, "Settings").with_size(520, 360).build();
    let notebook = Notebook::builder(&dialog).build();
    let general_page = Panel::builder(&notebook).build();
    general_page.set_sizer(BoxSizer::builder(Orientation::Vertical).build(), true);
    notebook.add_page(&general_page, "General", true, None);

    let proxy_page = Panel::builder(&notebook).build();
    let proxy_grid = FlexGridSizer::builder(0, 2).with_vgap(8).with_hgap(8).build();
    proxy_grid.add_growable_col(1, 1);
    let proxy = settings.proxy.clone().unwrap_or_default();
    let protocol = Choice::builder(&proxy_page)
        .with_choices(vec!["None".to_string(), "HTTP".to_string(), "SOCKS5".to_string()])
        .build();
    let protocol_index = match proxy.protocol {
        ProxyProtocol::None => 0,
        ProxyProtocol::Http => 1,
        ProxyProtocol::Socks5 => 2,
    };
    protocol.set_selection(protocol_index);
    let host = TextCtrl::builder(&proxy_page).with_value(&proxy.host).build();
    let port = TextCtrl::builder(&proxy_page).with_value(&proxy.port).build();
    let username = TextCtrl::builder(&proxy_page)
        .with_value(proxy.username.as_deref().unwrap_or(""))
        .build();
    let password = TextCtrl::builder(&proxy_page)
        .with_value(proxy.password.as_deref().unwrap_or(""))
        .with_style(TextCtrlStyle::Password)
        .build();
    proxy_grid.add(
        &StaticText::builder(&proxy_page).with_label("Protocol").build(),
        0,
        SizerFlag::All,
        4,
    );
    proxy_grid.add(&protocol, 1, SizerFlag::All | SizerFlag::Expand, 4);
    proxy_grid.add(
        &StaticText::builder(&proxy_page).with_label("IP / host").build(),
        0,
        SizerFlag::All,
        4,
    );
    proxy_grid.add(&host, 1, SizerFlag::All | SizerFlag::Expand, 4);
    proxy_grid.add(&StaticText::builder(&proxy_page).with_label("Port").build(), 0, SizerFlag::All, 4);
    proxy_grid.add(&port, 1, SizerFlag::All | SizerFlag::Expand, 4);
    proxy_grid.add(
        &StaticText::builder(&proxy_page).with_label("Username").build(),
        0,
        SizerFlag::All,
        4,
    );
    proxy_grid.add(&username, 1, SizerFlag::All | SizerFlag::Expand, 4);
    proxy_grid.add(
        &StaticText::builder(&proxy_page).with_label("Password").build(),
        0,
        SizerFlag::All,
        4,
    );
    proxy_grid.add(&password, 1, SizerFlag::All | SizerFlag::Expand, 4);
    proxy_page.set_sizer(proxy_grid, true);
    notebook.add_page(&proxy_page, "Proxy", false, None);

    let root = BoxSizer::builder(Orientation::Vertical).build();
    root.add(&notebook, 1, SizerFlag::All | SizerFlag::Expand, 8);
    let actions = BoxSizer::builder(Orientation::Horizontal).build();
    let spacer = StaticText::builder(&dialog).with_label("").build();
    let cancel = wxdragon::Button::builder(&dialog)
        .with_id(wxdragon::ID_CANCEL)
        .with_label("Cancel")
        .build();
    let ok = wxdragon::Button::builder(&dialog).with_label("OK").build();
    actions.add(&spacer, 1, SizerFlag::Expand, 0);
    actions.add(&cancel, 0, SizerFlag::All, 4);
    actions.add(&ok, 0, SizerFlag::All, 4);
    root.add_sizer(&actions, 0, SizerFlag::All | SizerFlag::Expand, 8);
    dialog.set_sizer(root, true);
    dialog.set_escape_id(wxdragon::ID_CANCEL);
    let dialog_for_cancel = dialog;
    cancel.on_click(move |_| dialog_for_cancel.end_modal(wxdragon::ID_CANCEL));
    let dialog_for_ok = dialog;
    ok.on_click(move |_| dialog_for_ok.end_modal(wxdragon::ID_OK));

    if dialog.show_modal() != wxdragon::ID_OK {
        dialog.destroy();
        return false;
    }
    let protocol = match protocol.get_selection().unwrap_or(0) {
        0 => None,
        1 => Some(ProxyProtocol::Http),
        2 => Some(ProxyProtocol::Socks5),
        _ => Some(ProxyProtocol::None),
    };
    settings.proxy = protocol.map(|protocol| ProxySettings {
        protocol,
        host: host.get_value(),
        port: port.get_value(),
        username: (!username.get_value().is_empty()).then_some(username.get_value()),
        password: (!password.get_value().is_empty()).then_some(password.get_value()),
    });
    settings.save();
    dialog.destroy();
    true
}
