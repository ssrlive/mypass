use eframe::egui;
use std::path::PathBuf;

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct Config {
    pub dark_mode: bool,
}

#[derive(Default, Debug)]
pub(crate) struct UiState {
    pub show_open_file_dialog: bool,
    pub file_path: Option<PathBuf>,
    pub password: String,
    pub keyfile: Option<PathBuf>,

    pub show_confirm_quit_dialog: bool,
    pub allowed_to_quit: bool,

    pub dropped_files: Vec<egui::DroppedFile>,

    pub config: Config,
}

impl UiState {
    pub fn on_show_open_file_dialog(&mut self) {
        self.show_open_file_dialog = true;
        self.show_confirm_quit_dialog = false;
        self.file_path = None;
        self.password.clear();
        self.keyfile = None;
    }

    pub fn is_files_being_dropped(&self) -> bool {
        !self.dropped_files.is_empty()
    }

    pub fn deal_with_dropped_files(&mut self) {
        let mut file_path: Option<PathBuf> = None;
        for file in &self.dropped_files {
            let info = if let Some(path) = &file.path {
                path.display().to_string()
            } else if !file.name.is_empty() {
                file.name.clone()
            } else {
                "???".to_owned()
            };
            if info.ends_with(".kdbx") {
                file_path = file.path.clone();
                break;
            }
        }
        if file_path.is_some() {
            self.on_show_open_file_dialog();
            self.file_path = file_path;
        }

        self.dropped_files.clear();
    }

    pub fn is_open_file_dialog_visible(&self) -> bool {
        self.show_open_file_dialog
    }

    pub fn on_open_file_dialog_confirm(&mut self) -> (Option<PathBuf>, String, Option<PathBuf>) {
        self.show_open_file_dialog = false;
        let password = std::mem::take(&mut self.password);
        (self.file_path.take(), password, self.keyfile.take())
    }

    pub fn on_open_file_dialog_cancel(&mut self) {
        self.show_open_file_dialog = false;
        self.password.clear();
        self.file_path = None;
        self.keyfile = None;
    }

    pub fn on_show_confirm_quit_dialog(&mut self) {
        self.show_open_file_dialog = false;
        self.show_confirm_quit_dialog = true;
    }

    pub fn on_confirm_quit_dialog_quit(&mut self) {
        self.allowed_to_quit = true;
        self.show_confirm_quit_dialog = false;
    }

    pub fn on_confirm_quit_dialog_cancel(&mut self) {
        self.show_confirm_quit_dialog = false;
    }

    pub fn is_confirm_quit_dialog_visible(&self) -> bool {
        self.show_confirm_quit_dialog
    }

    pub fn is_allowed_to_quit(&self) -> bool {
        self.allowed_to_quit
    }
}
