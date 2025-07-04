mod scene1;
use dropbear_engine::{input, WindowConfiguration};

fn main() {
    let config = WindowConfiguration {
        width: 1280.0,
        height: 720.0,
        title: "Eucalyptus, built with dropbear"
    };

    let _app = dropbear_engine::run_app!(config, |scene_manager, input_manager| {
        let testing_scene = scene1::TestingScene1::new();
        let testing_keyboard = scene1::TestingScene1::new();
        let testing_mouse = scene1::TestingScene1::new();
        
        scene_manager.add("testing_scene", Box::new(testing_scene));
        input_manager.add_keyboard("testing_scene_keyboard", Box::new(testing_keyboard));
        input_manager.add_mouse("testing_scene_mouse", Box::new(testing_mouse));
        
        scene_manager.attach_input("testing_scene", "testing_scene_keyboard");
        scene_manager.attach_input("testing_scene", "testing_scene_mouse");
        scene_manager.switch("testing_scene");
    }).unwrap();
}
