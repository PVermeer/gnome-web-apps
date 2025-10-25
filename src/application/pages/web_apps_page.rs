use super::NavPage;
use libadwaita::NavigationPage;

pub struct WebAppsPage {
    pub nav_page: NavigationPage,
    title: String,
}
impl NavPage for WebAppsPage {
    fn get_title(&self) -> &str {
        &self.title
    }

    fn get_navpage(&self) -> &NavigationPage {
        &self.nav_page
    }
}
impl WebAppsPage {
    pub fn new() -> Self {
        let title = String::from("Web Apps");
        let (nav_page, _header, _content_box) = Self::build_nav_page(&title);

        return Self { nav_page, title };
    }
}
