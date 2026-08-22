#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use keepass_ng::{
    Uuid,
    db::{Entry, Icon, NodePtr, group_get_children, node_is_group},
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wxdragon::prelude::*;

pub mod entry_view;
pub mod error;
pub mod favicon;
pub mod group_view;
pub mod icon_cache;
pub mod icon_picker;
pub mod keepass;
pub mod settings;
pub mod settings_dlg;

use keepass::KpDb;
use settings::{MAX_RECENT_FILES, Settings};

const TREE_PANE_NAME: &str = "architecture-tree";
const MENU_OPEN: i32 = 2001;
const MENU_SAVE: i32 = 2002;
const MENU_EXIT: i32 = 2003;
const MENU_CLOSE: i32 = 2004;
const MENU_SETTINGS: i32 = 2100;
const MENU_TOGGLE_TREE: i32 = 2101;
const MENU_TOGGLE_SHOW: i32 = 2102;
const MENU_ABOUT: i32 = 2201;
const MENU_TREE_NEW_GROUP: i32 = 2301;
const MENU_TREE_NEW_ENTRY: i32 = 2302;
const MENU_TREE_EDIT: i32 = 2303;
const MENU_TREE_DELETE: i32 = 2304;
const MENU_RECENT_FILE_FIRST: i32 = 2410;
const MENU_RECENT_FILE_LAST: i32 = MENU_RECENT_FILE_FIRST + MAX_RECENT_FILES as i32 - 1;

#[allow(dead_code)]
struct TrayState {
    taskbar: TaskBarIcon,
    popup_menu: Menu,
}

thread_local! {
    static TRAY_STATE: RefCell<Option<TrayState>> = const { RefCell::new(None) };
}

fn node_title(node: &NodePtr) -> String {
    node.borrow()
        .get_title()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("(no title)")
        .to_owned()
}

fn update_recent_menu(menu_bar: &MenuBar, recent_files: Option<&[String]>) {
    let Some(file_menu) = menu_bar.get_menu(0) else {
        return;
    };
    let Some(recent_item) = file_menu.find_item_by_position(6) else {
        return;
    };
    let Some(recent_menu) = recent_item.get_sub_menu() else {
        return;
    };
    for id in MENU_RECENT_FILE_FIRST..=MENU_RECENT_FILE_LAST {
        if recent_menu.find_item(id).is_some() {
            recent_menu.delete(id);
        }
    }
    let Some(recent_files) = recent_files else {
        if let Some(item) = recent_menu.append(MENU_RECENT_FILE_FIRST, "No recent files", "", ItemKind::Normal) {
            item.enable(false);
        }
        return;
    };
    if recent_files.is_empty() {
        if let Some(item) = recent_menu.append(MENU_RECENT_FILE_FIRST, "No recent files", "", ItemKind::Normal) {
            item.enable(false);
        }
        return;
    }
    for (index, path) in recent_files.iter().enumerate() {
        recent_menu.append(
            MENU_RECENT_FILE_FIRST + index as i32,
            &format!("{}  {}", index + 1, path),
            path,
            ItemKind::Normal,
        );
    }
}

fn node_icon_index(tree: &TreeCtrl, node: &NodePtr, kpdb: Option<&KpDb>) -> Option<i32> {
    let image_list = tree.get_image_list()?;
    let bitmap = match node.borrow().get_icon() {
        Icon::BuiltIn(icon_id) => icon_cache::icon_for_emoji(&icon_id.to_string(), 20),
        Icon::Custom(uuid) => kpdb
            .and_then(|db| db.db.as_ref())
            .and_then(|db| db.meta.custom_icon(uuid))
            .and_then(|icon| entry_view::bitmap_for_icon_fixed(&icon.data, 20)),
    }?;
    Some(image_list.add_bitmap(&bitmap))
}

fn set_node_icon(tree: &TreeCtrl, item: &TreeItemId, node: &NodePtr, kpdb: Option<&KpDb>) {
    let image_index = node_icon_index(tree, node, kpdb).unwrap_or(-1);
    for icon_type in [
        TreeItemIcon::Normal,
        TreeItemIcon::Selected,
        TreeItemIcon::Expanded,
        TreeItemIcon::SelectedExpanded,
    ] {
        tree.set_item_image(item, image_index, icon_type);
    }
}

fn append_nodes(tree: &TreeCtrl, parent: &TreeItemId, node: &NodePtr, kpdb: Option<&KpDb>) {
    let Some(children) = group_get_children(node) else {
        return;
    };

    for child in children {
        let uuid = child.borrow().get_uuid();
        let image_index = node_icon_index(tree, &child, kpdb);
        let Some(item) = tree.append_item_with_data(parent, &node_title(&child), uuid, image_index, image_index) else {
            continue;
        };
        if node_is_group(&child) {
            append_nodes(tree, &item, &child, kpdb);
        }
    }
}

fn append_node(tree: &TreeCtrl, parent: &TreeItemId, node: &NodePtr, kpdb: Option<&KpDb>) -> Option<TreeItemId> {
    let uuid = node.borrow().get_uuid();
    let image_index = node_icon_index(tree, node, kpdb);
    let item = tree.append_item_with_data(parent, &node_title(node), uuid, image_index, image_index)?;
    if node_is_group(node) {
        append_nodes(tree, &item, node, kpdb);
    }
    Some(item)
}

fn populate_tree(tree: &TreeCtrl, kpdb: Option<&KpDb>) -> Option<TreeItemId> {
    tree.set_image_list(ImageList::new(20, 20, true, 0));
    let Some(root) = kpdb.and_then(KpDb::get_root) else {
        tree.add_root("No keepass database loaded", None, None);
        return None;
    };

    let image_index = node_icon_index(tree, &root, kpdb);
    let root_item = tree.add_root_with_data(&node_title(&root), root.borrow().get_uuid(), image_index, image_index)?;
    append_nodes(tree, &root_item, &root, kpdb);
    tree.expand(&root_item);
    Some(root_item)
}

fn add_detail_row(grid: &FlexGridSizer, parent: &Panel, label: &str, value: &str) {
    let label = StaticText::builder(parent).with_label(label).build();
    let value = StaticText::builder(parent).with_label(value).build();
    grid.add(&label, 0, SizerFlag::All, 4);
    grid.add(&value, 1, SizerFlag::All | SizerFlag::Expand, 4);
}

fn find_tree_item(tree: &TreeCtrl, item: &TreeItemId, uuid: Uuid) -> Option<TreeItemId> {
    if let Some(data) = tree.get_custom_data(item)
        && let Some(item_uuid) = data.downcast_ref::<Uuid>()
        && *item_uuid == uuid
    {
        return Some(item.clone());
    }

    let (mut child, mut cookie) = tree.get_first_child(item)?;
    loop {
        if let Some(found) = find_tree_item(tree, &child, uuid) {
            return Some(found);
        }
        child = tree.get_next_child(item, &mut cookie)?;
    }
}

#[allow(clippy::too_many_arguments)]
fn build_node_view(
    parent: &Panel,
    frame: Frame,
    node: &NodePtr,
    tree: &TreeCtrl,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    status_bar: &StatusBar,
) {
    if node.borrow().downcast_ref::<Entry>().is_some() {
        let content_for_refresh = *content;
        let current_view_for_refresh = Rc::clone(current_view);
        let node_for_refresh = node.clone();
        let tree_for_refresh = *tree;
        let kpdb_for_refresh = Rc::clone(kpdb);
        let status_bar_for_refresh = *status_bar;
        let refresh = Rc::new(move || {
            show_node_view(
                &content_for_refresh,
                frame,
                &current_view_for_refresh,
                &node_for_refresh,
                &tree_for_refresh,
                &kpdb_for_refresh,
                &status_bar_for_refresh,
            );
        });
        entry_view::build_entry_view(parent, frame, node, refresh, Rc::clone(kpdb));
    } else {
        group_view::build_group_view(parent, frame, node, tree, content, current_view, kpdb, status_bar);
    }
}

fn show_node_view(
    content: &Panel,
    frame: Frame,
    current_view: &Rc<RefCell<Option<Panel>>>,
    node: &NodePtr,
    tree: &TreeCtrl,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    status_bar: &StatusBar,
) {
    if let Some(root_item) = tree.get_root_item()
        && let Some(tree_item) = find_tree_item(tree, &root_item, node.borrow().get_uuid())
    {
        let was_expanded = tree.is_expanded(&tree_item);
        tree.set_item_text(&tree_item, &node_title(node));
        set_node_icon(tree, &tree_item, node, kpdb.borrow().as_ref());
        if was_expanded {
            tree.expand(&tree_item);
        } else {
            tree.collapse(&tree_item);
        }
        tree.refresh(false, None);
    }
    if let Some(old_view) = current_view.borrow_mut().take() {
        old_view.destroy();
    }
    let new_view = Panel::builder(content).build();
    build_node_view(&new_view, frame, node, tree, content, current_view, kpdb, status_bar);
    let new_content_sizer = BoxSizer::builder(Orientation::Vertical).build();
    new_content_sizer.add(&new_view, 1, SizerFlag::All | SizerFlag::Expand, 0);
    content.set_sizer(new_content_sizer, true);
    content.layout();
    *current_view.borrow_mut() = Some(new_view);
}

fn show_node_editor_from_tree(
    frame: Frame,
    tree: &TreeCtrl,
    item: &TreeItemId,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    status_bar: &StatusBar,
) {
    let Some(data) = tree.get_custom_data(item) else {
        return;
    };
    let Some(uuid) = data.downcast_ref::<Uuid>() else {
        return;
    };
    let Some(node) = kpdb.borrow().as_ref().and_then(|db| db.get_node_by_id(*uuid)) else {
        return;
    };
    let result = if node_is_group(&node) {
        group_view::show_group_editor(&frame, &node, Rc::clone(kpdb))
    } else {
        entry_view::show_entry_editor(&frame, &node, Rc::clone(kpdb))
    };
    if result == wxdragon::ID_OK {
        show_node_view(content, frame, current_view, &node, tree, kpdb, status_bar);
    }
}

fn refresh_tree(
    frame: Frame,
    tree: &TreeCtrl,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    status_bar: &StatusBar,
    selected_uuid: Option<Uuid>,
) {
    tree.delete_all_items();
    let root_item = populate_tree(tree, kpdb.borrow().as_ref());
    let Some(root) = kpdb.borrow().as_ref().and_then(KpDb::get_root) else {
        return;
    };
    let selected_node = selected_uuid
        .and_then(|uuid| kpdb.borrow().as_ref().and_then(|db| db.get_node_by_id(uuid)))
        .unwrap_or_else(|| root.clone());
    if let Some(root_item) = root_item
        && let Some(selected_item) = find_tree_item(tree, &root_item, selected_node.borrow().get_uuid())
    {
        tree.select_item(&selected_item);
    }
    show_node_view(content, frame, current_view, &selected_node, tree, kpdb, status_bar);
}

fn save_if_data_changed(frame: Frame, kpdb: &Rc<RefCell<Option<KpDb>>>) -> Result<(), String> {
    let mut kpdb = kpdb.borrow_mut();
    let Some(db) = kpdb.as_mut() else {
        return Ok(());
    };
    if !db.is_data_changed() {
        return Ok(());
    }
    let should_upgrade = |version: keepass_ng::DatabaseVersion| {
        let msg = format!("This database is {version:?}, but saving requires KDBX4.1. Upgrade it to KDBX4.1?\nChoose No to abandon it.",);
        let dlg = MessageDialog::builder(&frame, &msg, "Upgrade database format?")
            .with_style(MessageDialogStyle::YesNo | MessageDialogStyle::IconWarning)
            .build();
        let res = dlg.show_modal() == wxdragon::ID_YES;
        dlg.destroy();
        res
    };
    let saved = db.save(Some(&should_upgrade)).map_err(|error| error.to_string())?;
    if saved && let (Some(path), Some(database)) = (db.db_path.as_deref(), db.db.as_ref()) {
        frame.set_title(&format!("mypass - {path} ({})", database.config.version));
    }
    Ok(())
}

fn close_current_file(
    frame: Frame,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    tree: &TreeCtrl,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    status_bar: &StatusBar,
) -> Result<(), String> {
    if kpdb.borrow().is_some() {
        save_if_data_changed(frame, kpdb)?;
    }
    kpdb.borrow_mut().take();
    tree.delete_all_items();
    if let Some(view) = current_view.borrow_mut().take() {
        view.destroy();
    }
    content.set_sizer(BoxSizer::builder(Orientation::Vertical).build(), true);
    content.layout();
    status_bar.set_status_text("Ready", 0);
    status_bar.set_status_text("No database loaded", 1);
    frame.set_title("mypass");
    Ok(())
}

fn open_database_from_picker(
    frame: Frame,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    tree: &TreeCtrl,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    status_bar: &StatusBar,
) -> Result<bool, String> {
    let database_dialog = FileDialog::builder(&frame)
        .with_message("Choose a KeePass database")
        .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
        .with_wildcard("KeePass database (*.kdbx)|*.kdbx|All files (*.*)|*.*")
        .build();
    if database_dialog.show_modal() != wxdragon::ID_OK {
        return Ok(false);
    }
    let Some(database_path) = database_dialog.get_path() else {
        return Ok(false);
    };
    open_database_path(frame, kpdb, tree, content, current_view, status_bar, database_path)
}

fn open_database_path(
    frame: Frame,
    kpdb: &Rc<RefCell<Option<KpDb>>>,
    tree: &TreeCtrl,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    status_bar: &StatusBar,
    database_path: String,
) -> Result<bool, String> {
    let dialog = Dialog::builder(&frame, "Open KeePass database")
        .with_style(DialogStyle::DefaultDialogStyle | DialogStyle::ResizeBorder | DialogStyle::MaximizeBox)
        .with_size(800, 220)
        .build();
    dialog.set_min_size(Size::new(620, 244));
    let dialog_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let fields = FlexGridSizer::builder(0, 3).with_vgap(8).with_hgap(8).build();
    fields.add_growable_col(1, 1);

    let database_path_control = TextCtrl::builder(&dialog)
        .with_value(&database_path)
        .with_style(TextCtrlStyle::ReadOnly)
        .build();
    let password_panel = Panel::builder(&dialog).build();
    let password_control = TextCtrl::builder(&password_panel)
        .with_style(TextCtrlStyle::Password)
        .with_size(Size::new(-1, 24))
        .build();
    let password_visible_control = TextCtrl::builder(&password_panel).with_size(Size::new(-1, 24)).build();
    password_visible_control.show(false);
    let password_toggle = Button::builder(&password_panel).with_size(Size::new(32, 24)).build();
    if let Some(bitmap) = icon_cache::icon_for_emoji("👁", 16) {
        password_toggle.set_bitmap_label(&bitmap);
    }
    password_toggle.set_tooltip("Show or hide password");
    let password_is_visible = Rc::new(Cell::new(false));
    let password_for_toggle = password_control;
    let password_visible_for_toggle = password_visible_control;
    let password_panel_for_toggle = password_panel;
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
    password_controls.add(&password_control, 1, SizerFlag::All | SizerFlag::Expand, 0);
    password_controls.add(&password_visible_control, 1, SizerFlag::All | SizerFlag::Expand, 0);
    password_controls.add(&password_toggle, 0, SizerFlag::All, 4);
    password_panel.set_sizer(password_controls, true);
    let key_file_control = TextCtrl::builder(&dialog)
        .with_value(" ")
        .with_style(TextCtrlStyle::ReadOnly)
        .with_size(Size::new(300, 28))
        .build();
    key_file_control.set_min_size(Size::new(300, 28));
    let key_file_button = Button::builder(&dialog).with_label("Pick key file...").build();
    fields.add(
        &StaticText::builder(&dialog).with_label("Database file").build(),
        0,
        SizerFlag::All,
        4,
    );
    fields.add(&database_path_control, 1, SizerFlag::All | SizerFlag::Expand, 4);
    fields.add(&StaticText::builder(&dialog).with_label("").build(), 0, SizerFlag::All, 4);
    fields.add(&StaticText::builder(&dialog).with_label("Password").build(), 0, SizerFlag::All, 4);
    fields.add(&password_panel, 1, SizerFlag::All | SizerFlag::Expand, 4);
    fields.add(&StaticText::builder(&dialog).with_label("").build(), 0, SizerFlag::All, 4);
    fields.add(&StaticText::builder(&dialog).with_label("Key file").build(), 0, SizerFlag::All, 4);
    fields.add(&key_file_control, 1, SizerFlag::All | SizerFlag::Expand, 4);
    fields.add(&key_file_button, 0, SizerFlag::All, 4);
    dialog_sizer.add_sizer(&fields, 1, SizerFlag::All | SizerFlag::Expand, 12);

    let key_file_for_picker = key_file_control;
    key_file_button.on_click(move |_| {
        let key_file_dialog = FileDialog::builder(&key_file_for_picker)
            .with_message("Choose a KeePass key file")
            .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
            .with_wildcard("Key files (*.*)|*.*")
            .build();
        if key_file_dialog.show_modal() == wxdragon::ID_OK
            && let Some(path) = key_file_dialog.get_path()
        {
            key_file_for_picker.set_value(&path);
        }
    });

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
    ok.on_click(move |_| dialog_for_ok.end_modal(wxdragon::ID_OK));

    if dialog.show_modal() != wxdragon::ID_OK {
        return Ok(false);
    }

    let key_file = key_file_control.get_value();
    let key_file = (!key_file.trim().is_empty()).then_some(key_file);
    let password = if password_is_visible.get() {
        password_visible_control.get_value()
    } else {
        password_control.get_value()
    };
    let password = (!password.is_empty()).then_some(password);
    let new_db = KpDb::open(&database_path, password.as_deref(), key_file.as_deref()).map_err(|error| error.to_string())?;
    save_if_data_changed(frame, kpdb)?;
    kpdb.borrow_mut().replace(new_db);
    tree.delete_all_items();
    if let Some(view) = current_view.borrow_mut().take() {
        view.destroy();
    }
    content.set_sizer(BoxSizer::builder(Orientation::Vertical).build(), true);
    content.layout();
    let root_item = populate_tree(tree, kpdb.borrow().as_ref());
    if let Some(root) = kpdb.borrow().as_ref().and_then(KpDb::get_root) {
        show_node_view(content, frame, current_view, &root, tree, kpdb, status_bar);
        if let Some(root_item) = root_item {
            tree.select_item(&root_item);
        }
    }
    let version = kpdb
        .borrow()
        .as_ref()
        .and_then(|db| db.db.as_ref())
        .map(|db| db.config.version.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    frame.set_title(&format!("mypass - {database_path} ({version})"));
    status_bar.set_status_text(&database_path, 1);
    status_bar.set_status_text("Database opened", 0);
    Ok(true)
}

fn application_icon() -> Option<Bitmap> {
    const MAIN_ICON_PNG: &[u8] = include_bytes!("../res/main-icon.png");
    let image = image::load_from_memory(MAIN_ICON_PNG).ok()?.to_rgba8();
    Bitmap::from_rgba(image.as_raw(), image.width(), image.height())
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    SystemOptions::set_option_by_int("msw.no-manifest-check", 1);
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    if let Err(e) = wxdragon::main(on_wxdragon_init) {
        log::error!("Failed to run wxDragon application: {e}");
    }
}

fn on_wxdragon_init(app: App) {
    let settings = Rc::new(RefCell::new(Settings::load()));
    let application_icon = application_icon();
    let frame = Frame::builder().with_title("mypass").with_size(Size::new(960, 640)).build();
    if let Some(icon) = application_icon.as_ref() {
        frame.set_icon(icon);
    }
    let saved_position = settings.borrow().window_position;
    let saved_size = settings.borrow().window_size;
    match (saved_position, saved_size) {
        (Some([x, y]), Some([width, height])) => frame.set_size_with_pos(x, y, width, height),
        (Some([x, y]), None) => {
            let size = frame.get_size();
            frame.set_size_with_pos(x, y, size.width, size.height);
        }
        (None, Some([width, height])) => frame.set_size(Size::new(width, height)),
        (None, None) => {}
    }
    let status_bar = StatusBar::builder(&frame)
        .with_fields_count(2)
        .with_status_widths(vec![-1, 280])
        .add_initial_text(0, "Ready")
        .add_initial_text(1, "No database loaded")
        .build();

    let kpdb = std::env::var("DB_PATH").ok().and_then(|path| {
        let password = std::env::var("PASSWORD").ok();
        let key_file = std::env::var("KEY_FILE").ok();
        KpDb::open(&path, password.as_deref(), key_file.as_deref()).ok()
    });
    let kpdb = Rc::new(RefCell::new(kpdb));
    if let Some(db) = kpdb.borrow().as_ref()
        && let Some(path) = db.db_path.as_deref()
        && let Some(database) = db.db.as_ref()
    {
        status_bar.set_status_text(path, 1);
        frame.set_title(&format!("mypass - {path} ({})", database.config.version));
    }
    if let Some(path) = kpdb.borrow().as_ref().and_then(|db| db.db_path.clone()) {
        settings.borrow_mut().add_recent_file(path);
    }

    let recent_menu = Menu::builder().build();
    let file_menu = Menu::builder()
        .append_item(MENU_OPEN, "Open...", "Open a KeePass database")
        .append_item(MENU_SAVE, "Save", "Save the current database")
        .append_item(MENU_CLOSE, "Close", "Close the current database")
        .append_separator()
        .append_item(MENU_SETTINGS, "Settings", "Open application settings")
        .build();
    file_menu.append_separator();
    file_menu.append_submenu(recent_menu, "Recent files", "Open a recently used KeePass database");
    file_menu.append_separator();
    file_menu.append(MENU_EXIT, "Exit", "Exit mypass", ItemKind::Normal);
    let view_menu = Menu::builder()
        .append_check_item(MENU_TOGGLE_TREE, "Architecture tree", "Show or hide the architecture tree")
        .build();
    let help_menu = Menu::builder()
        .append_item(MENU_ABOUT, "About mypass", "About this application")
        .build();
    let menu_bar = MenuBar::builder()
        .append(file_menu, "File")
        .append(view_menu, "View")
        .append(help_menu, "Help")
        .build();
    frame.set_menu_bar(menu_bar);
    if let Some(menu_bar) = frame.get_menu_bar() {
        update_recent_menu(&menu_bar, settings.borrow().recent_files.as_deref());
    }

    let content = Panel::builder(&frame).build();
    let initial_view = Panel::builder(&content).build();
    let initial_details = StaticText::builder(&initial_view)
        .with_label("Select an entry or group from the architecture tree.")
        .build();
    let initial_sizer = BoxSizer::builder(Orientation::Vertical).build();
    initial_sizer.add(&initial_details, 1, SizerFlag::All | SizerFlag::Expand, 12);
    initial_view.set_sizer(initial_sizer, true);
    let content_sizer = BoxSizer::builder(Orientation::Vertical).build();
    content_sizer.add(&initial_view, 1, SizerFlag::All | SizerFlag::Expand, 0);
    content.set_sizer(content_sizer, true);
    let current_view = Rc::new(RefCell::new(Some(initial_view)));

    let tree_pane = Panel::builder(&frame).build();
    let tree = TreeCtrl::builder(&tree_pane)
        .with_style(TreeCtrlStyle::HasButtons | TreeCtrlStyle::LinesAtRoot | TreeCtrlStyle::Single)
        .build();
    let tree_sizer = BoxSizer::builder(Orientation::Vertical).build();
    tree_sizer.add(&tree, 1, SizerFlag::All | SizerFlag::Expand, 4);
    tree_pane.set_sizer(tree_sizer, true);
    let root_item = populate_tree(&tree, kpdb.borrow().as_ref());

    let aui = AuiManager::builder(&frame).build();
    let tree_width = settings.borrow().tree_width.unwrap_or(300);
    aui.add_pane_with_info(
        &tree_pane,
        AuiPaneInfo::new()
            .with_name(TREE_PANE_NAME)
            .with_caption("Architecture tree")
            .left()
            .best_size(tree_width, 600)
            .close_button(true)
            .floatable(true)
            .dockable(true),
    );
    aui.add_pane_with_info(
        &content,
        AuiPaneInfo::new().with_name("details").with_caption("Details").center_pane(),
    );
    aui.update();
    let show_tree_panel = settings.borrow().show_tree_panel.unwrap_or(true);
    if aui.set_pane_shown(TREE_PANE_NAME, show_tree_panel) {
        aui.update();
    }

    let kpdb_for_selection = Rc::clone(&kpdb);
    let current_view_for_selection = Rc::clone(&current_view);
    let tree_for_selection = tree;
    let content_for_selection = content;
    let status_bar_for_selection = status_bar;
    tree.on_selection_changed(move |event| {
        let Some(item) = event.get_item().or_else(|| tree.get_selection()) else {
            return;
        };
        let Some(data) = tree.get_custom_data(&item) else {
            return;
        };
        let Some(uuid) = data.downcast_ref::<Uuid>() else {
            return;
        };
        let Some(node) = kpdb_for_selection.borrow().as_ref().and_then(|db| db.get_node_by_id(*uuid)) else {
            return;
        };
        show_node_view(
            &content_for_selection,
            frame,
            &current_view_for_selection,
            &node,
            &tree_for_selection,
            &kpdb_for_selection,
            &status_bar_for_selection,
        );
        tree_for_selection.set_focus();
        status_bar.set_status_text("Node selected", 0);
    });

    let context_node = Rc::new(Cell::new(None::<Uuid>));
    let enter_edit_requested = Rc::new(Cell::new(false));
    let enter_edit_requested_for_key = Rc::clone(&enter_edit_requested);
    let context_node_for_key = Rc::clone(&context_node);
    let tree_for_key = tree;
    let kpdb_for_key = Rc::clone(&kpdb);
    let content_for_key = content;
    let current_view_for_key = Rc::clone(&current_view);
    let status_bar_for_key = status_bar;
    tree.on_key_down(move |event| {
        if let wxdragon::WindowEventData::Keyboard(key_event) = event
            && (key_event.get_key_code() == Some(13) || key_event.get_key_code() == Some(127))
            && let Some(item) = tree_for_key.get_selection()
        {
            if key_event.get_key_code() == Some(13) {
                enter_edit_requested_for_key.set(true);
                show_node_editor_from_tree(
                    frame,
                    &tree_for_key,
                    &item,
                    &kpdb_for_key,
                    &content_for_key,
                    &current_view_for_key,
                    &status_bar_for_key,
                );
            } else if let Some(data) = tree_for_key.get_custom_data(&item)
                && let Some(uuid) = data.downcast_ref::<Uuid>()
            {
                context_node_for_key.set(Some(*uuid));
                frame.process_menu_command(MENU_TREE_DELETE);
            }
        }
    });

    let enter_edit_requested_for_activation = Rc::clone(&enter_edit_requested);
    let tree_for_activation = tree;
    let kpdb_for_activation = Rc::clone(&kpdb);
    let content_for_activation = content;
    let current_view_for_activation = Rc::clone(&current_view);
    let status_bar_for_activation = status_bar;
    tree.on_item_activated(move |event| {
        if enter_edit_requested_for_activation.replace(false) {
            return;
        }
        let Some(item) = event.get_item() else {
            return;
        };
        let Some(data) = tree_for_activation.get_custom_data(&item) else {
            return;
        };
        let Some(uuid) = data.downcast_ref::<Uuid>() else {
            return;
        };
        let Some(node) = kpdb_for_activation.borrow().as_ref().and_then(|db| db.get_node_by_id(*uuid)) else {
            return;
        };
        if !node_is_group(&node) {
            show_node_editor_from_tree(
                frame,
                &tree_for_activation,
                &item,
                &kpdb_for_activation,
                &content_for_activation,
                &current_view_for_activation,
                &status_bar_for_activation,
            );
        }
    });

    let context_node_for_tree = Rc::clone(&context_node);
    let tree_for_context = tree;
    let kpdb_for_context = Rc::clone(&kpdb);
    tree.on_item_right_click(move |event| {
        let Some(item) = event.get_item() else {
            return;
        };
        let Some(data) = tree_for_context.get_custom_data(&item) else {
            return;
        };
        let Some(uuid) = data.downcast_ref::<Uuid>() else {
            return;
        };
        context_node_for_tree.set(Some(*uuid));
        tree_for_context.select_item(&item);

        let is_group = kpdb_for_context
            .borrow()
            .as_ref()
            .and_then(|db| db.get_node_by_id(*uuid))
            .map(|node| node_is_group(&node))
            .unwrap_or(false);
        let mut menu = if is_group {
            Menu::builder()
                .append_item(MENU_TREE_NEW_GROUP, "New Group", "Create a new group")
                .append_item(MENU_TREE_NEW_ENTRY, "New Entry", "Create a new entry")
                .append_separator()
                .append_item(MENU_TREE_EDIT, "Edit", "Edit this node")
                .append_item(MENU_TREE_DELETE, "Delete", "Delete this node")
                .build()
        } else {
            Menu::builder()
                .append_item(MENU_TREE_EDIT, "Edit", "Edit this node")
                .append_item(MENU_TREE_DELETE, "Delete", "Delete this node")
                .build()
        };
        tree_for_context.popup_menu(&mut menu, None);
    });
    if let Some(root_item) = root_item {
        tree.select_item(&root_item);
    }
    if let Some(root) = kpdb.borrow().as_ref().and_then(KpDb::get_root) {
        show_node_view(&content, frame, &current_view, &root, &tree, &kpdb, &status_bar);
        tree.set_focus();
    }

    let menu_bar_for_toggle = frame.get_menu_bar().expect("menu bar was just installed");
    menu_bar_for_toggle.check_item(MENU_TOGGLE_TREE, aui.is_pane_shown(TREE_PANE_NAME));

    let kpdb_for_menu = Rc::clone(&kpdb);
    let tree_for_menu = tree;
    let content_for_menu = content;
    let current_view_for_menu = Rc::clone(&current_view);
    let context_node_for_menu = Rc::clone(&context_node);
    let settings_for_menu = Rc::clone(&settings);
    frame.on_menu(move |event| match event.get_id() {
        MENU_OPEN => match open_database_from_picker(
            frame,
            &kpdb_for_menu,
            &tree_for_menu,
            &content_for_menu,
            &current_view_for_menu,
            &status_bar,
        ) {
            Ok(false) => status_bar.set_status_text("Open cancelled", 0),
            Ok(true) => {
                if let Some(path) = kpdb_for_menu.borrow().as_ref().and_then(|db| db.db_path.clone()) {
                    let mut settings = settings_for_menu.borrow_mut();
                    settings.add_recent_file(path);
                    settings.save();
                    if let Some(menu_bar) = frame.get_menu_bar() {
                        update_recent_menu(&menu_bar, settings.recent_files.as_deref());
                    }
                }
            }
            Err(error) => {
                MessageDialog::builder(&frame, &error, "Open failed")
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconError)
                    .build()
                    .show_modal();
                status_bar.set_status_text("Could not open database", 0);
            }
        },
        id @ MENU_RECENT_FILE_FIRST..=MENU_RECENT_FILE_LAST => {
            let index = (id - MENU_RECENT_FILE_FIRST) as usize;
            let Some(path) = settings_for_menu
                .borrow()
                .recent_files
                .as_ref()
                .and_then(|recent_files| recent_files.get(index))
                .cloned()
            else {
                return;
            };
            match open_database_path(
                frame,
                &kpdb_for_menu,
                &tree_for_menu,
                &content_for_menu,
                &current_view_for_menu,
                &status_bar,
                path.clone(),
            ) {
                Ok(true) => {
                    let mut settings = settings_for_menu.borrow_mut();
                    settings.add_recent_file(path);
                    settings.save();
                    if let Some(menu_bar) = frame.get_menu_bar() {
                        update_recent_menu(&menu_bar, settings.recent_files.as_deref());
                    }
                }
                Ok(false) => status_bar.set_status_text("Open cancelled", 0),
                Err(error) => status_bar.set_status_text(&format!("Open failed: {error}"), 0),
            }
        }
        MENU_SAVE => match save_if_data_changed(frame, &kpdb_for_menu) {
            Ok(()) => status_bar.set_status_text("Database saved", 0),
            Err(error) => status_bar.set_status_text(&format!("Save failed: {error}"), 0),
        },
        MENU_SETTINGS => {
            settings_dlg::show(&frame, &mut settings_for_menu.borrow_mut());
        }
        MENU_CLOSE => match close_current_file(
            frame,
            &kpdb_for_menu,
            &tree_for_menu,
            &content_for_menu,
            &current_view_for_menu,
            &status_bar,
        ) {
            Ok(()) => status_bar.set_status_text("Current database closed", 0),
            Err(error) => status_bar.set_status_text(&format!("Close failed: {error}"), 0),
        },
        MENU_EXIT => {
            if let Err(error) = save_if_data_changed(frame, &kpdb_for_menu) {
                MessageDialog::builder(&frame, &format!("Could not save database: {error}"), "Save failed")
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconError)
                    .build()
                    .show_modal();
                return;
            }
            frame.close(true);
        }
        MENU_TREE_NEW_GROUP | MENU_TREE_NEW_ENTRY => {
            let Some(parent_uuid) = context_node_for_menu.get() else {
                return;
            };
            let Some(parent) = kpdb_for_menu.borrow().as_ref().and_then(|db| db.get_node_by_id(parent_uuid)) else {
                status_bar.set_status_text("No database loaded", 0);
                return;
            };
            if !node_is_group(&parent) {
                status_bar.set_status_text("New nodes can only be created in a group", 0);
                return;
            }
            let result = if let Some(db) = kpdb_for_menu.borrow_mut().as_mut() {
                if event.get_id() == MENU_TREE_NEW_GROUP {
                    db.create_new_group(parent_uuid)
                } else {
                    db.create_new_entry(parent_uuid)
                }
            } else {
                status_bar.set_status_text("No database loaded", 0);
                return;
            };
            match result {
                Ok(node) => {
                    let uuid = node.borrow().get_uuid();
                    let editor_result = if node_is_group(&node) {
                        group_view::show_group_editor(&frame, &node, Rc::clone(&kpdb_for_menu))
                    } else {
                        entry_view::show_entry_editor(&frame, &node, Rc::clone(&kpdb_for_menu))
                    };
                    if editor_result == wxdragon::ID_OK {
                        status_bar.set_status_text("Node created", 0);
                    } else {
                        status_bar.set_status_text("Node created without changes", 0);
                    }
                    refresh_tree(
                        frame,
                        &tree_for_menu,
                        &kpdb_for_menu,
                        &content_for_menu,
                        &current_view_for_menu,
                        &status_bar,
                        Some(uuid),
                    );
                }
                Err(error) => status_bar.set_status_text(&format!("Create failed: {error}"), 0),
            }
        }
        MENU_TREE_EDIT => {
            let Some(uuid) = context_node_for_menu.get() else {
                return;
            };
            let Some(root_item) = tree_for_menu.get_root_item() else {
                return;
            };
            let Some(item) = find_tree_item(&tree_for_menu, &root_item, uuid) else {
                return;
            };
            show_node_editor_from_tree(
                frame,
                &tree_for_menu,
                &item,
                &kpdb_for_menu,
                &content_for_menu,
                &current_view_for_menu,
                &status_bar,
            );
        }
        MENU_TREE_DELETE => {
            let Some(uuid) = context_node_for_menu.get() else {
                return;
            };
            let Some(node) = kpdb_for_menu.borrow().as_ref().and_then(|db| db.get_node_by_id(uuid)) else {
                return;
            };
            let Some(parent_uuid) = node.borrow().get_parent() else {
                status_bar.set_status_text("The database root cannot be deleted", 0);
                return;
            };
            let Some(root_item) = tree_for_menu.get_root_item() else {
                return;
            };
            let Some(tree_item) = find_tree_item(&tree_for_menu, &root_item, uuid) else {
                return;
            };
            let Some(parent_item) = find_tree_item(&tree_for_menu, &root_item, parent_uuid) else {
                return;
            };
            let title = node_title(&node);
            let dialog = MessageDialog::builder(&frame, &format!("Delete {title}?"), "Confirm deletion")
                .with_style(MessageDialogStyle::YesNo | MessageDialogStyle::IconWarning)
                .build();
            let confirmed = dialog.show_modal() == wxdragon::ID_YES;
            dialog.destroy();
            if !confirmed {
                return;
            }
            let delete_result = {
                let mut kpdb = kpdb_for_menu.borrow_mut();
                kpdb.as_mut().map(|db| db.delete_node(uuid))
            };
            match delete_result {
                Some(Ok(())) => {
                    tree_for_menu.delete(&tree_item);
                    tree_for_menu.select_item(&parent_item);
                    let (deleted_node, recycle_bin) = kpdb_for_menu
                        .borrow()
                        .as_ref()
                        .map(|db| {
                            let deleted_node = db.get_node_by_id(uuid);
                            let recycle_bin = db.db.as_ref().and_then(|database| database.get_recycle_bin());
                            (deleted_node, recycle_bin)
                        })
                        .unwrap_or((None, None));
                    if let (Some(deleted_node), Some(recycle_bin)) = (deleted_node, recycle_bin)
                        && let Some(root_item) = tree_for_menu.get_root_item()
                    {
                        let recycle_bin_uuid = recycle_bin.borrow().get_uuid();
                        let recycle_bin_item = find_tree_item(&tree_for_menu, &root_item, recycle_bin_uuid)
                            .or_else(|| append_node(&tree_for_menu, &root_item, &recycle_bin, kpdb_for_menu.borrow().as_ref()));
                        if let Some(recycle_bin_item) = recycle_bin_item {
                            let deleted_item = find_tree_item(&tree_for_menu, &recycle_bin_item, uuid)
                                .or_else(|| append_node(&tree_for_menu, &recycle_bin_item, &deleted_node, kpdb_for_menu.borrow().as_ref()));
                            tree_for_menu.expand(&recycle_bin_item);
                            if let Some(deleted_item) = deleted_item {
                                tree_for_menu.ensure_visible(&deleted_item);
                            }
                        }
                    }
                    if let Some(parent) = kpdb_for_menu.borrow().as_ref().and_then(|db| db.get_node_by_id(parent_uuid)) {
                        show_node_view(
                            &content_for_menu,
                            frame,
                            &current_view_for_menu,
                            &parent,
                            &tree_for_menu,
                            &kpdb_for_menu,
                            &status_bar,
                        );
                    }
                    status_bar.set_status_text("Node deleted", 0);
                }
                Some(Err(error)) => status_bar.set_status_text(&format!("Delete failed: {error}"), 0),
                None => status_bar.set_status_text("No database loaded", 0),
            }
        }
        MENU_TOGGLE_TREE => {
            let shown = !aui.is_pane_shown(TREE_PANE_NAME);
            if aui.set_pane_shown(TREE_PANE_NAME, shown) {
                aui.update();
                menu_bar_for_toggle.check_item(MENU_TOGGLE_TREE, shown);
                status_bar.set_status_text("Architecture tree visibility changed", 0);
            }
        }
        MENU_ABOUT => {
            MessageDialog::builder(&frame, "A KeePass database viewer.", "About mypass")
                .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
                .build()
                .show_modal();
        }
        _ => {}
    });

    let menu_bar_for_open = frame.get_menu_bar().expect("menu bar was just installed");
    frame.on_menu_opened(move |_| {
        menu_bar_for_open.check_item(MENU_TOGGLE_TREE, aui.is_pane_shown(TREE_PANE_NAME));
    });

    let mut popup_menu = Menu::builder()
        .append_item(MENU_TOGGLE_SHOW, "Open Application", "Open the main application window")
        .append_separator()
        .append_item(MENU_SETTINGS, "Settings", "Open application settings")
        .append_item(MENU_ABOUT, "About", "About this application")
        .append_separator()
        .append_item(MENU_EXIT, "Exit", "Exit the application")
        .build();

    let settings_for_tray = Rc::clone(&settings);
    let taskbar = TaskBarIcon::builder().with_icon_type(TaskBarIconType::Default).build();
    taskbar.set_popup_menu(&mut popup_menu);

    // Bind menu event handler to the TaskBarIcon itself (not the frame)
    taskbar.on_menu(move |event| {
        let menu_id = event.get_id();
        match menu_id {
            MENU_TOGGLE_SHOW => {
                log::info!("Open Application clicked");
                frame.show(true);
            }
            MENU_SETTINGS => {
                settings_dlg::show(&frame, &mut settings_for_tray.borrow_mut());
            }
            MENU_ABOUT => {
                MessageDialog::builder(&frame, "A KeePass database viewer.", "About mypass")
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
                    .build()
                    .show_modal();
            }
            MENU_EXIT => {
                log::info!("Exit clicked");
                frame.close(true);
            }
            _ => {
                log::warn!("Unknown menu item clicked: {menu_id}");
            }
        }
    });

    if let Some(icon) = application_icon.as_ref() {
        if !taskbar.set_icon(icon, "mypass") {
            log::warn!("Failed to install the mypass tray icon");
        }
    } else {
        if let Some(fallback) = Bitmap::new(16, 16)
            && !taskbar.set_icon(&fallback, "mypass")
        {
            log::warn!("Failed to install the fallback mypass tray icon");
        }
    }
    TRAY_STATE.with(|state| {
        *state.borrow_mut() = Some(TrayState { taskbar, popup_menu });
    });

    frame.show(true);
    if settings.borrow().window_position.is_none() {
        frame.centre();
    }

    let icon_warmup_timer = Rc::new(Timer::new(&frame));
    let icon_warmup_index = Rc::new(Cell::new(0usize));
    let icon_warmup_size = Rc::new(Cell::new(28u32));
    let timer_for_warmup = Rc::clone(&icon_warmup_timer);
    let timer_for_destroy = Rc::clone(&icon_warmup_timer);
    icon_warmup_timer.on_tick(move |_| {
        let index = icon_warmup_index.get();
        let size = icon_warmup_size.get();
        if icon_cache::warm_up_step(size, index) {
            icon_warmup_index.set(index + 1);
            return;
        }

        if size == 28 {
            icon_warmup_size.set(20);
            icon_warmup_index.set(0);
            return;
        }
        timer_for_warmup.stop();
        log::info!("Icon warmup completed");
    });
    icon_warmup_timer.start(100, false);
    frame.on_destroy(move |_| {
        timer_for_destroy.stop();
    });

    let os_shuting_down = Rc::new(Cell::new(false));
    let os_shuting_down_1 = Rc::clone(&os_shuting_down);
    app.on_query_end_session(move |_event| {
        os_shuting_down_1.set(true);
    });

    let kpdb_for_close = Rc::clone(&kpdb);
    let settings_for_close = Rc::clone(&settings);
    let tree_pane_for_close = tree_pane;
    let aui_for_close = aui;
    frame.on_close(move |evt| {
        if let wxdragon::WindowEventData::General(event) = &evt
            && event.can_veto()
        {
            if os_shuting_down.get() {
                _ = save_if_data_changed(frame, &kpdb_for_close);
                return;
            }
            event.veto();
            frame.show(false);
            return;
        }
        if let Err(error) = save_if_data_changed(frame, &kpdb_for_close) {
            MessageDialog::builder(&frame, &format!("Could not save database: {error}"), "Save failed")
                .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconError)
                .build()
                .show_modal();
            if let wxdragon::WindowEventData::General(event) = &evt {
                event.veto();
            }
            return;
        }
        {
            let mut settings = settings_for_close.borrow_mut();
            let position = frame.get_position();
            let size = frame.get_size();
            settings.window_position = Some([position.x, position.y]);
            settings.window_size = Some([size.width, size.height]);
            settings.tree_width = Some(tree_pane_for_close.get_size().width);
            settings.show_tree_panel = Some(aui_for_close.is_pane_shown(TREE_PANE_NAME));
            settings.save();
        }
        aui_for_close.uninit();
        if let wxdragon::WindowEventData::General(event) = &evt {
            event.skip(true);
        }
    });

    let frame_ptr = frame.handle_ptr();
    frame.on_destroy(move |evt| {
        let wxdragon::WindowEventData::General(event) = &evt else {
            log::error!("Unexpected Frame destroy event received: {evt:?}");
            return;
        };
        let Some(event_object) = event.get_event_object() else {
            log::error!("Failed to get event object from Frame destroy event: {event:?}");
            return;
        };
        let curr_p = event_object.as_ptr();
        if curr_p != frame_ptr {
            log::error!("Destroy event received for a different object {curr_p:p} than the Frame pointer {frame_ptr:?}",);
            return;
        }
        TRAY_STATE.with(|state| {
            if let Some(mut tray) = state.borrow_mut().take() {
                tray.taskbar.destroy();
                tray.popup_menu.destroy_menu();
            }
        });
        log::info!("Application destroyed, event is {evt:?}");
    });
}
