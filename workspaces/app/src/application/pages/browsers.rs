use super::NavPage;
use crate::application::{App, pages::PrefPage};
use common::browsers::{Browser, Installation};
use gtk::{
    Align, Label, Orientation,
    prelude::{BoxExt, WidgetExt},
};
use libadwaita::{
    ActionRow, ExpanderRow, NavigationPage, PreferencesGroup, PreferencesPage, StatusPage,
    prelude::{ExpanderRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use std::fmt::Write as _;
use std::rc::Rc;

pub struct Browsers {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    prefs_page: PreferencesPage,
}
impl NavPage for Browsers {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        Some(&self.nav_row)
    }
}
impl Browsers {
    pub fn new() -> Rc<Self> {
        let title = "Browsers";
        let icon = "web-browser-symbolic";

        let PrefPage {
            nav_page,
            nav_row,
            prefs_page,
            ..
        } = Self::build_nav_page(title, icon).with_preference_page();

        Rc::new(Self {
            nav_page,
            nav_row,
            prefs_page,
        })
    }

    pub fn init(self: &Rc<Self>, app: &Rc<App>) {
        let browser_pref_groups = Self::build_browser_sections(app);

        for pref_group in browser_pref_groups {
            self.prefs_page.add(&pref_group);
        }
    }

    fn build_browser_sections(app: &Rc<App>) -> Vec<PreferencesGroup> {
        let flatpak_browsers = app.browser_configs.get_flatpak_browsers();
        let system_browsers = app.browser_configs.get_system_browsers();

        if flatpak_browsers.is_empty() && system_browsers.is_empty() {
            let status_page = StatusPage::builder()
                .title("No compatible browsers found")
                .description("Try installing one!")
                .icon_name("system-search-symbolic")
                .build();

            let pref_group = PreferencesGroup::builder().build();
            pref_group.add(&status_page);

            return Vec::from([pref_group]);
        }

        let flatpak_pref_group = PreferencesGroup::builder().title("Flatpak").build();
        let system_pref_group = PreferencesGroup::builder().title("System").build();

        for browser in &flatpak_browsers {
            let browser_row = Self::build_browser_row(browser);
            flatpak_pref_group.add(&browser_row);
        }
        for browser in &system_browsers {
            let browser_row = Self::build_browser_row(browser);
            system_pref_group.add(&browser_row);
        }

        Vec::from([flatpak_pref_group, system_pref_group])
    }

    fn build_browser_row(browser: &Browser) -> ExpanderRow {
        let row = ExpanderRow::builder().title(&browser.name).build();
        row.add_prefix(&browser.get_icon());

        let browser_expand = Self::build_browser_expand_content(browser);
        row.add_row(&browser_expand);

        row
    }

    fn build_browser_expand_content(browser: &Browser) -> gtk::Box {
        let content_box = gtk::Box::new(Orientation::Vertical, 12);
        content_box.set_margin_top(12);
        content_box.set_margin_bottom(12);

        let header_box = gtk::Box::new(Orientation::Horizontal, 6);
        header_box.set_halign(Align::Center);
        header_box.set_margin_top(12);
        header_box.set_margin_bottom(12);

        let app_label = Label::builder()
            .label(&browser.name)
            .css_classes(["title-2"])
            .build();

        let app_image = &browser.get_icon();
        app_image.set_css_classes(&["icon-dropshadow"]);
        app_image.set_pixel_size(32);

        header_box.append(app_image);
        header_box.append(&app_label);
        content_box.append(&header_box);

        if browser.is_flatpak() {
            let mut label = String::new();

            if let Some(flatpak_id) = &browser.flatpak_id {
                let _ = write!(label, "{flatpak_id}");
            }

            if let Installation::Flatpak(installation) = &browser.installation {
                let _ = write!(label, " ({installation})");
            }

            let flatpak_label = Label::builder().label(&label).valign(Align::Center).build();
            content_box.append(&flatpak_label);
        }

        if browser.is_system()
            && let Some(executable) = &browser.executable
        {
            let executable_label = Label::builder()
                .label(executable)
                .valign(Align::Center)
                .build();
            content_box.append(&executable_label);
        }

        if browser.can_isolate {
            let isolation_label = Label::builder()
                .label("<b>This browser can isolate your web apps.</b>")
                .use_markup(true)
                .build();
            content_box.append(&isolation_label);
        }

        if !browser.issues.is_empty() {
            let mut markup_issues = String::new();
            for issue in &browser.issues {
                let _ = writeln!(markup_issues, "• {issue}");
            }

            let issues_label = Label::builder()
                .use_markup(true)
                .label("<b>Known issues:</b>")
                .build();

            let issues_list_label = Label::builder()
                .label(&markup_issues)
                .wrap(true)
                .halign(Align::Center)
                .build();

            content_box.append(&issues_label);
            content_box.append(&issues_list_label);
        }

        content_box
    }
}
