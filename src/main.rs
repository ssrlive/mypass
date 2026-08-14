#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use keepass_ng::{
    Uuid,
    db::{Entry, NodePtr, group_get_children, node_is_group},
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wxdragon::prelude::*;

pub mod entry_view;
pub mod error;
pub mod group_view;
pub mod keepass;

use keepass::KpDb;

const TREE_PANE_NAME: &str = "architecture-tree";
const MENU_OPEN: i32 = 2001;
const MENU_SAVE: i32 = 2002;
const MENU_EXIT: i32 = 2003;
const MENU_SETTINGS: i32 = 2100;
const MENU_TOGGLE_TREE: i32 = 2101;
const MENU_TOGGLE_SHOW: i32 = 2102;
const MENU_ABOUT: i32 = 2201;

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

fn append_nodes(tree: &TreeCtrl, parent: &TreeItemId, node: &NodePtr) {
    let Some(children) = group_get_children(node) else {
        return;
    };

    for child in children {
        let uuid = child.borrow().get_uuid();
        let Some(item) = tree.append_item_with_data(parent, &node_title(&child), uuid, None, None) else {
            continue;
        };
        if node_is_group(&child) {
            append_nodes(tree, &item, &child);
        }
    }
}

fn populate_tree(tree: &TreeCtrl, kpdb: Option<&KpDb>) -> Option<TreeItemId> {
    let Some(root) = kpdb.and_then(KpDb::get_root) else {
        tree.add_root("No keepass database loaded", None, None);
        return None;
    };

    let root_item = tree.add_root_with_data(&node_title(&root), root.borrow().get_uuid(), None, None)?;
    append_nodes(tree, &root_item, &root);
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

fn build_node_view(
    parent: &Panel,
    node: &NodePtr,
    tree: &TreeCtrl,
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    kpdb: &Rc<Option<KpDb>>,
    status_bar: &StatusBar,
) {
    if node.borrow().downcast_ref::<Entry>().is_some() {
        entry_view::build_entry_view(parent, node);
    } else {
        group_view::build_group_view(parent, node, tree, content, current_view, kpdb, status_bar);
    }
}

fn show_node_view(
    content: &Panel,
    current_view: &Rc<RefCell<Option<Panel>>>,
    node: &NodePtr,
    tree: &TreeCtrl,
    kpdb: &Rc<Option<KpDb>>,
    status_bar: &StatusBar,
) {
    if let Some(old_view) = current_view.borrow_mut().take() {
        old_view.destroy();
    }
    let new_view = Panel::builder(content).build();
    build_node_view(&new_view, node, tree, content, current_view, kpdb, status_bar);
    let new_content_sizer = BoxSizer::builder(Orientation::Vertical).build();
    new_content_sizer.add(&new_view, 1, SizerFlag::All | SizerFlag::Expand, 0);
    content.set_sizer(new_content_sizer, true);
    content.layout();
    *current_view.borrow_mut() = Some(new_view);
}

fn main() {
    dotenvy::dotenv().ok();
    SystemOptions::set_option_by_int("msw.no-manifest-check", 1);
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    let _ = wxdragon::main(|_| {
        let frame = Frame::builder().with_title("mypass").with_size(Size::new(960, 640)).build();
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
        let kpdb = Rc::new(kpdb);
        if let Some(path) = kpdb.as_ref().as_ref().and_then(|db| db.db_path.clone()) {
            status_bar.set_status_text(&path, 1);
        }

        let file_menu = Menu::builder()
            .append_item(MENU_OPEN, "Open...", "Open a KeePass database")
            .append_item(MENU_SAVE, "Save", "Save the current database")
            .append_separator()
            .append_item(MENU_EXIT, "Exit", "Exit mypass")
            .build();
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
        let root_item = populate_tree(&tree, kpdb.as_ref().as_ref());
        let exit_requested = Rc::new(Cell::new(false));

        let aui = AuiManager::builder(&frame).build();
        aui.add_pane_with_info(
            &tree_pane,
            AuiPaneInfo::new()
                .with_name(TREE_PANE_NAME)
                .with_caption("Architecture tree")
                .left()
                .best_size(300, 600)
                .close_button(true)
                .floatable(true)
                .dockable(true),
        );
        aui.add_pane_with_info(
            &content,
            AuiPaneInfo::new().with_name("details").with_caption("Details").center_pane(),
        );
        aui.update();

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
            let Some(node) = kpdb_for_selection.as_ref().as_ref().and_then(|db| db.get_node_by_id(*uuid)) else {
                return;
            };
            show_node_view(
                &content_for_selection,
                &current_view_for_selection,
                &node,
                &tree_for_selection,
                &kpdb_for_selection,
                &status_bar_for_selection,
            );
            tree_for_selection.set_focus();
            status_bar.set_status_text("Node selected", 0);
        });
        if let Some(root_item) = root_item {
            tree.select_item(&root_item);
        }
        if let Some(root) = kpdb.as_ref().as_ref().and_then(KpDb::get_root) {
            show_node_view(&content, &current_view, &root, &tree, &kpdb, &status_bar);
            tree.set_focus();
        }

        let menu_bar_for_toggle = frame.get_menu_bar().expect("menu bar was just installed");
        menu_bar_for_toggle.check_item(MENU_TOGGLE_TREE, aui.is_pane_shown(TREE_PANE_NAME));

        let menu_exit_requested = Rc::clone(&exit_requested);
        frame.on_menu(move |event| match event.get_id() {
            MENU_OPEN => status_bar.set_status_text("Open is not implemented yet", 0),
            MENU_SAVE => status_bar.set_status_text("Save is not implemented yet", 0),
            MENU_EXIT => {
                menu_exit_requested.set(true);
                frame.close(false);
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

        let taskbar = TaskBarIcon::builder().with_icon_type(TaskBarIconType::Default).build();
        taskbar.set_popup_menu(&mut popup_menu);

        // Bind menu event handler to the TaskBarIcon itself (not the frame)
        let tray_exit_requested = Rc::clone(&exit_requested);
        taskbar.on_menu(move |event| {
            let menu_id = event.get_id();
            match menu_id {
                MENU_TOGGLE_SHOW => {
                    log::info!("Open Application clicked");
                    frame.show(true);
                }
                MENU_SETTINGS => {
                    log::info!("Settings clicked");
                }
                MENU_ABOUT => {
                    MessageDialog::builder(&frame, "A KeePass database viewer.", "About mypass")
                        .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
                        .build()
                        .show_modal();
                }
                MENU_EXIT => {
                    log::info!("Exit clicked");
                    tray_exit_requested.set(true);
                    frame.close(true);
                }
                _ => {
                    log::warn!("Unknown menu item clicked: {menu_id}");
                }
            }
        });

        let icon = ArtProvider::get_bitmap(ArtId::Help, ArtClient::Menu, Some(Size::new(16, 16)));

        if let Some(icon) = icon {
            if !taskbar.set_icon(&icon, "mypass") {
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
        frame.centre();

        let close_exit_requested = Rc::clone(&exit_requested);
        let aui_for_close = aui;
        frame.on_close(move |evt| {
            if let wxdragon::WindowEventData::General(event) = &evt
                && event.can_veto()
                && !close_exit_requested.get()
            {
                use MessageDialogStyle as MDS;
                let res = MessageDialog::builder(&frame, "Are you sure you want to close the application?", "Confirm Close")
                    .with_style(MDS::OK | MDS::Cancel | MDS::IconInformation)
                    .build()
                    .show_modal();

                if res != wxdragon::ID_OK {
                    event.veto();
                    return;
                }
            }
            close_exit_requested.set(true);
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
    });
}
