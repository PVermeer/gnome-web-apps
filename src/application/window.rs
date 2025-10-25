pub mod view;

use std::rc::Rc;

use crate::{application::App, config};
use libadwaita::{
    AboutWindow, ApplicationWindow,
    gtk::{
        self,
        prelude::{GtkWindowExt, WidgetExt},
    },
    prelude::AdwApplicationWindowExt,
};
use view::View;

pub struct AppWindow {
    pub window: ApplicationWindow,
    pub view: View,
}
impl AppWindow {
    pub fn new(adw_application: &libadwaita::Application) -> Self {
        let view = View::new();
        let title = config::APP_NAME.to_string();
        let window = ApplicationWindow::builder()
            .application(adw_application)
            .title(&title)
            .default_height(600)
            .default_width(800)
            .content(&view.split_view)
            .build();

        Self { window, view }
    }

    pub fn init(&self, app: Rc<App>) {
        self.view.init(app.clone());

        self.window.add_breakpoint(self.view.breakpoint.clone());
        self.window.present();
    }

    pub fn show_about() {
        let about = AboutWindow::new();
        about.set_application_name(config::APP_NAME);
        about.set_version(config::VERSION);
        about.set_developer_name(config::DEVELOPER);
        about.add_credit_section(Some("Code by"), config::CREDITS);
        about.add_acknowledgement_section(None, config::ACKNOWLEDGEMENT);
        about.add_legal_section("Legal", None, gtk::License::Gpl30, None);
        about.show();
    }
}
