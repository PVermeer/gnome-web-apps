use std::rc::Rc;

use crate::application::{App, window::AppWindow};
use libadwaita::{
    gio::{ActionEntry, Menu, MenuItem, SimpleActionGroup, prelude::ActionMapExtManual},
    gtk::{MenuButton, prelude::WidgetExt},
};

pub struct AppMenu {
    pub button: MenuButton,
    actions: SimpleActionGroup,
}
impl AppMenu {
    pub const NAME: &str = "app-menu";
    pub const ACTION_LABEL: &str = "app-menu";

    pub fn new() -> Self {
        // GTK does not let a popovermenu to be created programmatically
        // https://blog.libove.org/posts/rust-gtk--creating-a-menu-bar-programmatically-with-gtk-rs
        let button = MenuButton::builder()
            .name(AppMenu::NAME)
            .icon_name("open-menu-symbolic")
            .build();
        let menu = Menu::new();
        button.set_menu_model(Some(&menu));

        // Must use actions, there is currently no way to register a fn on click or something
        let actions = SimpleActionGroup::new();
        Self::add_about(&menu, &actions);

        return Self { button, actions };
    }

    pub fn init(&self, app: Rc<App>) {
        app.app_window
            .window
            .insert_action_group(Self::ACTION_LABEL, Some(&self.actions));
    }

    fn add_about(menu: &Menu, actions: &SimpleActionGroup) {
        let item = Self::build_menu_item("About", ("about", AppWindow::show_about), actions);
        menu.prepend_item(&item);
    }

    fn build_menu_item(
        label: &str,
        (action_name, action): (&str, impl Fn() + 'static),
        actions: &SimpleActionGroup,
    ) -> MenuItem {
        let item = MenuItem::new(
            Some(label),
            Some(&(Self::ACTION_LABEL.to_owned() + "." + action_name)),
        );
        let action = ActionEntry::builder(action_name)
            .activate(move |_: &SimpleActionGroup, _, _| {
                action();
            })
            .build();
        actions.add_action_entries([action]);

        item
    }
}
