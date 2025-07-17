mod scene1;
mod scene2;

use std::{cell::RefCell, rc::Rc};

use dropbear_engine::{input, scene, WindowConfiguration};

use crate::scene1::TestingScene1;

fn main() {
    let config = WindowConfiguration {
        width: 1280u32,
        height: 720u32,
        title: "Eucalyptus, built with dropbear",
    };

    let _app = dropbear_engine::run_app!(config, |mut scene_manager, mut input_manager| {
        let testing_scene = Rc::new(RefCell::new(TestingScene1::new()));
        let scene2 = Rc::new(RefCell::new(crate::scene2::TestingScene1::new()));

        scene::add_scene_with_input(&mut scene_manager, &mut input_manager, testing_scene, "testing_scene_1");
        scene::add_scene_with_input(&mut scene_manager, &mut input_manager, scene2, "testing_scene_2");

        scene_manager.switch("testing_scene_1");

        (scene_manager, input_manager)
    })
    .unwrap();
}
