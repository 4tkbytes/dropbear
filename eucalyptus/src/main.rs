mod scene1;
use std::{cell::RefCell, rc::Rc};

use dropbear_engine::WindowConfiguration;

use crate::scene1::TestingScene1;

fn main() {
    let config = WindowConfiguration {
        width: 1280u32,
        height: 720u32,
        title: "Eucalyptus, built with dropbear",
    };

    let _app = dropbear_engine::run_app!(config, |scene_manager, input_manager| {
        let testing_scene = Rc::new(RefCell::new(TestingScene1::new()));

        scene_manager.add("testing_scene", testing_scene.clone());
        input_manager.add_keyboard("testing_scene_keyboard", testing_scene.clone());
        input_manager.add_mouse("testing_scene_mouse", testing_scene.clone());

        scene_manager.attach_input("testing_scene", "testing_scene_keyboard");
        scene_manager.attach_input("testing_scene", "testing_scene_mouse");
        scene_manager.switch("testing_scene");
    })
    .unwrap();
}
