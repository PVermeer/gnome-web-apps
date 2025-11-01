use super::WebAppsPage;
use crate::application::pages::{NavPage, PrefPage};
use freedesktop_desktop_entry::DesktopEntry;
use libadwaita::{
    ActionRow, NavigationPage, PreferencesGroup, WrapBox,
    gtk::{
        self, Button, Image, Label, Orientation,
        prelude::{BoxExt, ButtonExt, WidgetExt},
    },
    prelude::{ActionRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use log::{debug, error};
use std::{borrow::Cow, process::Command, rc::Rc};

pub struct WebAppView {
    nav_page: NavigationPage,
}
impl NavPage for WebAppView {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        None
    }
}
impl WebAppView {
    const LABEL: &str = "web-app-page";
    const LOG_TARGET: &str = Self::LABEL;

    pub fn new(desktop_file: &Rc<DesktopEntry>, locales: &[String]) -> Self {
        let title = &desktop_file
            .name(locales)
            .unwrap_or(Cow::Borrowed("No name"));
        let icon = "preferences-desktop-apps-symbolic";

        let PrefPage {
            nav_page,
            prefs_page,
            ..
        } = Self::build_nav_page(title, icon).with_preference_page();

        let header = Self::build_app_header(desktop_file, locales);

        let web_app_pref_group = Self::build_app_pref_group();
        prefs_page.add(&header);
        prefs_page.add(&web_app_pref_group);

        Self { nav_page }
    }

    fn build_app_header(desktop_file: &Rc<DesktopEntry>, locales: &[String]) -> PreferencesGroup {
        let pref_group = PreferencesGroup::builder().build();
        let content_box = gtk::Box::new(Orientation::Vertical, 6);
        let app_name = desktop_file
            .name(locales)
            .unwrap_or(Cow::Borrowed("No name..."));
        let app_label = Label::builder()
            .label(app_name)
            .css_classes(["title-1"])
            .build();

        let run_button = Button::builder()
            .label("Open")
            .css_classes(["suggested-action", "pill"])
            .build();
        let mut exec_args = desktop_file.parse_exec().unwrap_or_default();
        let command = if exec_args.is_empty() {
            run_button.set_sensitive(false);
            None
        } else {
            Some(exec_args.remove(0))
        };
        let args = exec_args;
        if let Some(cmd) = command {
            run_button.connect_clicked(move |_| {
                debug!(target: Self::LOG_TARGET, "Running app: '{} {}'", cmd, args.join(" "));

                #[allow(clippy::zombie_processes)]
                let result = Command::new(cmd.clone()).args(&args).spawn();

                if let Err(error) = result {
                    error!(target: Self::LOG_TARGET, "Failed to run app '{} {}': {error}", cmd, args.join(" "));
                }
            });
        }

        let button_wrap_box = WrapBox::builder()
            .align(0.5)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        button_wrap_box.append(&run_button);

        let app_image = WebAppsPage::get_image_icon(desktop_file);
        app_image.set_css_classes(&["icon-dropshadow"]);
        app_image.set_pixel_size(96);

        content_box.append(&app_image);
        content_box.append(&app_label);
        content_box.append(&button_wrap_box);

        pref_group.add(&content_box);

        pref_group
    }

    fn build_app_pref_group() -> PreferencesGroup {
        let pref_group = PreferencesGroup::builder()
            .title("My awesome web app")
            .build();

        let row = ActionRow::builder().title("Some row").build();
        let prefix = Image::from_icon_name("web-browser-symbolic");
        row.add_prefix(&prefix);

        row.connect_activated(|_| {
            debug!("TODO");
        });

        pref_group.add(&row);

        pref_group
    }
}
