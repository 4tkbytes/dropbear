mod scene1;
mod menu;

use std::{cell::RefCell, rc::Rc};

use dropbear_engine::{scene, WindowConfiguration};

use crate::scene1::TestingScene1;

fn main() {
    let config = WindowConfiguration {
        width: 1280u32,
        height: 720u32,
        title: "Eucalyptus, built with dropbear",
    };

    let _app = dropbear_engine::run_app!(config, |mut scene_manager, mut input_manager| {
        let testing_scene = Rc::new(RefCell::new(TestingScene1::new()));
        let main_menu = Rc::new(RefCell::new(menu::MainMenu::new()));

        scene::add_scene_with_input(&mut scene_manager, &mut input_manager, testing_scene, "testing_scene_1");
        scene::add_scene_with_input(&mut scene_manager, &mut input_manager, main_menu, "main_menu");

        scene_manager.switch("main_menu");

        (scene_manager, input_manager)
    })
    .unwrap();
}
