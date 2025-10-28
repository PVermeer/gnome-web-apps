use super::NavPage;
use crate::application::pages::ContentPage;
use libadwaita::{
    ActionRow, NavigationPage,
    gtk::{
        self, Button, Label,
        prelude::{BoxExt, ButtonExt},
    },
};
use std::rc::Rc;

pub struct HomePage {
    nav_page: NavigationPage,
    nav_row: ActionRow,
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
        let title = "Home page";
        let icon = "go-home-symbolic";

        let ContentPage {
            nav_page,
            nav_row,
            content_box,
            ..
        } = Self::build_nav_page(title, icon).with_content_box();

        let top_label = Label::builder()
            .label(concat!(
                "<b>Placeholder home page</b>\n",
                "<span>With some standard widgets</span>\n",
            ))
            .wrap(true)
            .use_markup(true)
            .halign(gtk::Align::Start)
            .build();

        let button = Button::builder()
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .label("Open file")
            .build();

        button.connect_clicked(|_| println!("TODO"));

        content_box.append(&top_label);
        content_box.append(&button);

        Rc::new(Self { nav_page, nav_row })
    }
}
