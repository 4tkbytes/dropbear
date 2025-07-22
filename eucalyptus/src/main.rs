mod editor;
mod menu;
mod scene1;

use std::{cell::RefCell, rc::Rc};

use dropbear_engine::{WindowConfiguration, scene, tokio};

use crate::scene1::TestingScene1;

#[tokio::main]
async fn main() {
    let config = WindowConfiguration {
        title: "Eucalyptus, built with dropbear",
        windowed_mode: dropbear_engine::WindowedModes::Maximised,
        read_from_config: None,
    };

    let _app = dropbear_engine::run_app!(config, |mut scene_manager, mut input_manager| {
        let testing_scene = Rc::new(RefCell::new(TestingScene1::new()));
        let main_menu = Rc::new(RefCell::new(menu::MainMenu::new()));
        let editor = Rc::new(RefCell::new(editor::Editor::new()));

        scene::add_scene_with_input(
            &mut scene_manager,
            &mut input_manager,
            testing_scene,
            "testing_scene_1",
        );
        scene::add_scene_with_input(
            &mut scene_manager,
            &mut input_manager,
            main_menu,
            "main_menu",
        );
        scene::add_scene_with_input(&mut scene_manager, &mut input_manager, editor, "editor");

        scene_manager.switch("main_menu");

        (scene_manager, input_manager)
    })
    .unwrap();
}
