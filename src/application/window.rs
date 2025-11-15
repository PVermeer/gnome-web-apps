mod view;

use crate::{application::App, config};
use libadwaita::{
    AboutDialog, ApplicationWindow,
    gtk::prelude::GtkWindowExt,
    prelude::{AdwApplicationWindowExt, AdwDialogExt},
};
use std::rc::Rc;
use view::View;

pub struct AppWindow {
    pub adw_window: ApplicationWindow,
    pub view: View,
}
impl AppWindow {
    pub fn new(adw_application: &libadwaita::Application) -> Self {
        let view = View::new();
        let title = config::APP_NAME.to_string();
        let window = ApplicationWindow::builder()
            .application(adw_application)
            .title(&title)
            .default_height(650)
            .default_width(850)
            .content(&view.nav_split)
            .build();

        Self {
            adw_window: window,
            view,
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        self.view.init(app);

        self.adw_window.add_breakpoint(self.view.breakpoint.clone());
        self.adw_window.present();
    }

    pub fn show_about(&self) {
        let about = AboutDialog::builder()
            .name(config::APP_NAME)
            .version(config::VERSION)
            .developer_name(config::DEVELOPER)
            .license_type(config::LICENSE)
            .build();
        about.add_credit_section(Some("Code by"), config::CREDITS);
        about.add_acknowledgement_section(None, config::ACKNOWLEDGEMENT);

        about.present(Some(&self.adw_window));
    }
}
