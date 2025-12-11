use crate::application::{
    App,
    pages::{ContentPage, NavPage, Page},
};
use common::config::{self, OnceLockExt};
use gtk::{
    Align, Button, Orientation,
    prelude::{ButtonExt, WidgetExt},
};
use libadwaita::{
    ActionRow, NavigationPage, WrapBox,
    gtk::{self, Label, prelude::BoxExt},
};
use std::rc::Rc;

pub struct HomePage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
    content_box: gtk::Box,
}
impl NavPage for HomePage {
    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }

    fn get_nav_row(&self) -> Option<&ActionRow> {
        Some(&self.nav_row)
    }
}
impl HomePage {
    pub fn new() -> Rc<Self> {
        let title = "Home";
        let icon = "go-home-symbolic";

        let ContentPage {
            nav_page,
            nav_row,
            content_box,
            ..
        } = Self::build_nav_page(title, icon).with_content_box();

        Rc::new(Self {
            nav_page,
            nav_row,
            content_box,
        })
    }

    pub fn init(&self, app: &Rc<App>) {
        self.content_box.set_spacing(24);

        let header = Self::build_header(app);
        let text = Self::build_text();
        let buttons = Self::build_action_buttons(app);

        self.content_box.append(&header);
        self.content_box.append(&text);
        self.content_box.append(&buttons);
    }

    fn build_header(app: &Rc<App>) -> gtk::Box {
        let content_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .halign(Align::Center)
            .valign(Align::Fill)
            .build();

        let icon = app.get_icon();
        icon.set_pixel_size(96);
        icon.set_css_classes(&["icon-dropshadow"]);
        icon.set_margin_start(25);
        icon.set_margin_end(25);

        let name = Label::builder()
            .label(config::APP_NAME.get_value())
            .css_classes(["title-1"])
            .wrap(true)
            .build();

        content_box.append(&icon);
        content_box.append(&name);

        content_box
    }

    fn build_text() -> gtk::Box {
        let content_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Center)
            .spacing(12)
            .build();

        let text = Label::builder()
            .label("To get started you may create your first web app.")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();

        let text2 = Label::builder()
            .label("The browser tab show the supported and installed browsers")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();

        let text3 = Label::builder()
            .label("The info tab shows some tips and tricks + some general information")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();

        content_box.append(&text);
        content_box.append(&text2);
        content_box.append(&text3);

        content_box
    }

    fn build_action_buttons(app: &Rc<App>) -> WrapBox {
        let content_box = WrapBox::builder()
            .orientation(Orientation::Horizontal)
            .justify(libadwaita::JustifyMode::Spread)
            .justify_last_line(true)
            .child_spacing(12)
            .line_spacing(12)
            .halign(Align::Center)
            .valign(Align::Center)
            .vexpand(true)
            .height_request(200)
            .build();

        let go_to_apps_button = Button::builder()
            .label("Go to Web Apps")
            .css_classes(["suggested-action", "pill"])
            .valign(Align::Center)
            .halign(Align::Center)
            .build();

        let more_info_button = Button::builder()
            .label("More information")
            .css_classes(["pill"])
            .valign(Align::Center)
            .halign(Align::Center)
            .build();

        let app_clone = app.clone();
        go_to_apps_button.connect_clicked(move |_| {
            app_clone.navigate(&Page::WebApps);
        });

        let app_clone = app.clone();
        more_info_button.connect_clicked(move |_| {
            app_clone.navigate(&Page::Info);
        });

        content_box.append(&go_to_apps_button);
        content_box.append(&more_info_button);

        content_box
    }
}
