mod view;

use crate::{application::App, config, services::config::OnceLockExt};
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
        let window = ApplicationWindow::builder()
            .application(adw_application)
            .title(config::APP_NAME.get_value())
            .default_height(700)
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
            .application_name(config::APP_NAME.get_value())
            .version(config::VERSION.get_value())
            .developer_name(config::DEVELOPER.get_value())
            .license_type(*config::LICENSE.get_value())
            .issue_url(config::ISSUES_URL.get_value())
            .build();

        about.present(Some(&self.adw_window));
    }

    pub fn close(&self) {
        self.adw_window.close();
    }
}
