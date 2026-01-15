mod view;

use crate::application::App;
use common::{
    assets,
    config::{self, OnceLockExt},
};
use gtk::License;
use libadwaita::{
    AboutDialog, ApplicationWindow,
    gtk::prelude::GtkWindowExt,
    prelude::{AdwApplicationWindowExt, AdwDialogExt},
};
use semver::Version;
use std::fmt::Write as _;
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
            .icon_name(config::APP_ID.get_value())
            .default_height(775)
            .default_width(900)
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
        let license = match config::LICENSE.get_value().as_str() {
            "GPL-3.0" => License::Gpl30,
            "GPL-3.0-only" => License::Gpl30Only,
            _ => panic!("Could not convert license"),
        };

        let about = AboutDialog::builder()
            .application_icon(config::APP_ID.get_value())
            .application_name(config::APP_NAME.get_value())
            .version(config::VERSION.get_value())
            .developer_name(config::DEVELOPER.get_value())
            .license_type(license)
            .issue_url(config::ISSUES_URL.get_value())
            .release_notes(Self::parse_release_notes_xml())
            .build();

        about.present(Some(&self.adw_window));
    }

    pub fn close(&self) {
        self.adw_window.close();
    }

    fn parse_release_notes_xml() -> String {
        let metainfo = assets::get_meta_info();
        let mut release_xml = String::new();

        let mut release_version = String::new();
        let mut release_count = 1;

        for line in metainfo.lines() {
            let line = line.trim();
            if line.starts_with("<release") {
                if release_count >= 5 {
                    break;
                }

                let start_pattern = r#"version=""#;
                let end_pattern = r#"" date="#;
                let Some(version_start) = line.find(start_pattern) else {
                    continue;
                };
                let Some(version_end) = line.find(end_pattern) else {
                    continue;
                };
                let version_str = &line[version_start + start_pattern.len()..version_end];
                let (Ok(version), Ok(app_version)) = (
                    Version::parse(version_str),
                    Version::parse(config::VERSION.get_value()),
                ) else {
                    continue;
                };
                if version != app_version {
                    let _ = write!(release_xml, "<p><em>Previous version {version}</em></p>");
                    release_count += 1;
                }

                let _ = write!(release_version, "{version}");
                continue;
            } else if line.starts_with("</release>") {
                release_version.clear();
                continue;
            }
            if release_version.is_empty() {
                continue;
            }

            if line.starts_with("<p>")
                || line.starts_with("<ul>")
                || line.starts_with("<ol>")
                || line.starts_with("<li>")
                || line.starts_with("</p>")
                || line.starts_with("</ul>")
                || line.starts_with("</ol>")
                || line.starts_with("</li>")
            {
                let _ = writeln!(release_xml, "{line}");
            }
        }

        release_xml
    }
}
