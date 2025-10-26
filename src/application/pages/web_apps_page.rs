use super::NavPage;
use libadwaita::NavigationPage;

pub struct WebAppsPage {
    pub nav_page: NavigationPage,
    title: String,
    icon: String,
}
impl NavPage for WebAppsPage {
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
impl WebAppsPage {
    pub fn new() -> Self {
        let title = String::from("Web Apps");
        let icon = "preferences-desktop-apps-symbolic".to_string();
        let (nav_page, _header, _content_box) = Self::build_nav_page(&title);

        Self {
            nav_page,
            title,
            icon,
        }
    }
}
