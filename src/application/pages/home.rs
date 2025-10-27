use super::NavPage;
use libadwaita::{
    NavigationPage,
    gtk::{
        self, Button, Label,
        prelude::{BoxExt, ButtonExt},
    },
};

pub struct HomePage {
    nav_page: NavigationPage,
    title: String,
    icon: String,
}
impl NavPage for HomePage {
    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_icon(&self) -> &str {
        &self.icon
    }

    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }
}
impl HomePage {
    pub fn new() -> Self {
        let title = String::from("Home page");
        let icon = "go-home-symbolic".to_string();

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

        let (nav_page, _header, content_box) = Self::build_nav_page(&title);

        content_box.append(&top_label);
        content_box.append(&button);

        Self {
            nav_page,
            title,
            icon,
        }
    }
}
