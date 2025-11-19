use crate::{application::App, services::config};
use anyhow::Error;
use gtk::Orientation;
use libadwaita::{
    AlertDialog, ResponseAppearance,
    prelude::{AdwDialogExt, AlertDialogExt},
};
use std::rc::Rc;

pub struct ErrorDialog {
    dialog: AlertDialog,
}
impl ErrorDialog {
    pub const DIALOG_EXIT: &str = "exit";

    pub fn new() -> Self {
        let dialog = Self::build_dialog();

        Self { dialog }
    }

    pub fn init(&self, app: &Rc<App>) {
        self.connect_dialog(app);
    }

    pub fn show(&self, app: &Rc<App>, error: &Error) {
        self.dialog.set_body(&error.to_string());
        self.dialog.present(Some(&app.window.adw_window));
    }

    fn build_dialog() -> AlertDialog {
        let content_box = gtk::Box::new(Orientation::Horizontal, 0);
        let dialog = AlertDialog::builder()
            .heading(format!("{} Error:", config::APP_NAME))
            .extra_child(&content_box)
            .build();
        dialog.add_response(Self::DIALOG_EXIT, "Close");
        dialog.set_response_appearance(Self::DIALOG_EXIT, ResponseAppearance::Destructive);
        dialog.set_default_response(Some(Self::DIALOG_EXIT));

        dialog
    }

    fn connect_dialog(&self, app: &Rc<App>) {
        let app_clone = app.clone();
        self.dialog
            .connect_response(Some(Self::DIALOG_EXIT), move |_, _| {
                app_clone.close();
            });
    }
}
