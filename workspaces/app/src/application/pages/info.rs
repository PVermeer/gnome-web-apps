use crate::application::{
    App,
    pages::{NavPage, PrefPage},
};
use gtk::{Image, Orientation};
use libadwaita::{
    ActionRow, ExpanderRow, NavigationPage, PreferencesGroup, PreferencesPage,
    gtk::{self, Label, prelude::BoxExt},
    prelude::{ExpanderRowExt, PreferencesGroupExt, PreferencesPageExt},
};
use std::rc::Rc;

static GENERAL_PANGO: &str = include_str!("./info/general.pango");
static PERMISSIONS_PANGO: &str = include_str!("./info/permissions.pango");

pub struct InfoPage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    prefs_page: PreferencesPage,
}
impl NavPage for InfoPage {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        Some(&self.nav_row)
    }
}
impl InfoPage {
    const CONTENT_MARGING: i32 = 12;

    pub fn new() -> Rc<Self> {
        let title = "Info";
        let icon = "help-about-symbolic";

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

    pub fn init(&self, _app: &Rc<App>) {
        let info_pref_group = PreferencesGroup::new();
        let expandable_pref_group = PreferencesGroup::new();

        let general_info = Self::build_tips_row();
        let permisssions = Self::build_permissions_row();

        info_pref_group.add(&general_info);
        expandable_pref_group.add(&permisssions);

        self.prefs_page.add(&info_pref_group);
        self.prefs_page.add(&expandable_pref_group);
    }

    fn build_tips_row() -> ExpanderRow {
        let row = ExpanderRow::builder()
            .title("Tips &amp; tricks")
            .expanded(true)
            .build();
        row.add_prefix(&Image::from_icon_name("checkbox-checked-symbolic"));

        let content_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_start(Self::CONTENT_MARGING)
            .margin_end(Self::CONTENT_MARGING)
            .margin_top(Self::CONTENT_MARGING)
            .margin_bottom(Self::CONTENT_MARGING)
            .build();

        let text_label = Label::builder()
            .use_markup(true)
            .wrap(true)
            .label(GENERAL_PANGO)
            .build();

        content_box.append(&text_label);
        row.add_row(&content_box);

        row
    }

    fn build_permissions_row() -> ExpanderRow {
        let row = ExpanderRow::builder().title("Permissions").build();
        row.add_prefix(&Image::from_icon_name("security-medium-rtl-symbolic"));

        let content_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_start(Self::CONTENT_MARGING)
            .margin_end(Self::CONTENT_MARGING)
            .margin_top(Self::CONTENT_MARGING)
            .margin_bottom(Self::CONTENT_MARGING)
            .build();

        let text_label = Label::builder()
            .use_markup(true)
            .wrap(true)
            .label(PERMISSIONS_PANGO)
            .build();

        content_box.append(&text_label);
        row.add_row(&content_box);

        row
    }
}
