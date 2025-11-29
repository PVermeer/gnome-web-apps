use crate::application::App;
use libadwaita::{
    AlertDialog, ResponseAppearance,
    gio::{ActionEntry, Menu, MenuItem, SimpleActionGroup, prelude::ActionMapExtManual},
    gtk::{MenuButton, prelude::WidgetExt},
    prelude::{AdwDialogExt, AlertDialogExt},
};
use common::{
    assets,
    config::{self, OnceLockExt},
};
use std::rc::Rc;

pub struct AppMenu {
    pub button: MenuButton,
    menu: Menu,
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

        Self {
            button,
            menu,
            actions,
        }
    }

    pub fn init(&self, app: &Rc<App>) {
        app.window
            .adw_window
            .insert_action_group(Self::ACTION_LABEL, Some(&self.actions));

        let section_1 = Menu::new();
        let section_2 = Menu::new();

        let reset = self.build_reset(app);
        let about = self.build_about(app);

        section_1.append_item(&reset);
        section_2.append_item(&about);

        self.menu.append_section(None, &section_1);
        self.menu.append_section(None, &section_2);
    }

    fn build_about(&self, app: &Rc<App>) -> MenuItem {
        let app_clone = app.clone();
        self.build_menu_item(
            "About",
            ("about", move || {
                app_clone.window.show_about();
            }),
        )
    }

    fn build_reset(&self, app: &Rc<App>) -> MenuItem {
        let app_clone = app.clone();
        self.build_menu_item(
            "Reset app",
            ("reset_app", move || {
                let dialog_ok = "ok";
                let dialog_cancel = "cancel";

                let dialog = AlertDialog::builder()
                    .heading(format!("Reset {}?", config::APP_NAME.get_value()))
                    .body(
                        "This will reset the config files (e.g.: browser configs).\n\n\
                        It will not remove your create web apps.",
                    )
                    .build();

                dialog.add_response(dialog_cancel, "Cancel");
                dialog.add_response(dialog_ok, "I understand");
                dialog.set_response_appearance(dialog_cancel, ResponseAppearance::Suggested);
                dialog.set_default_response(Some(dialog_cancel));
                dialog.set_close_response(dialog_cancel);

                let app_clone_response = app_clone.clone();
                dialog.connect_response(Some(dialog_ok), move |_, _| {
                    if let Err(error) = assets::reset_config_files(&app_clone_response.dirs) {
                        app_clone_response.show_error(&error);
                    }
                    app_clone_response.clone().restart();
                });

                dialog.present(Some(&app_clone.window.adw_window));
            }),
        )
    }

    fn build_menu_item(
        &self,
        label: &str,
        (action_name, action): (&str, impl Fn() + 'static),
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
        self.actions.add_action_entries([action]);

        item
    }
}
