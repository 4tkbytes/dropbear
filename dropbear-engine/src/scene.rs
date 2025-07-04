use std::collections::HashMap;

pub trait Scene {
    fn load(&mut self);
    fn update(&mut self, dt: f32);
    fn render(&mut self);
    fn exit(&mut self);
}

pub struct Manager {
    current_scene: Option<String>,
    next_scene: Option<String>,
    scenes: HashMap<String, Box<dyn Scene>>,
    scene_input_map: HashMap<String, String>, // Maps scene name to input handler name
}

impl Manager {
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            current_scene: None,
            next_scene: None,
            scene_input_map: HashMap::new(),
        }
    }

    pub fn switch(&mut self, name: &str) {
        if self.scenes.contains_key(name) {
            self.next_scene = Some(name.to_string());
        }
    }

    pub fn add(&mut self, name: &str, scene: Box<dyn Scene>) {
        self.scenes.insert(name.to_string(), scene);
    }

    pub fn attach_input(&mut self, scene_name: &str, input_name: &str) {
        self.scene_input_map.insert(scene_name.to_string(), input_name.to_string());
    }

    pub fn update(&mut self, dt: f32) {
        // Handle scene transitions
        if let Some(next_scene_name) = self.next_scene.take() {
            if let Some(current_scene_name) = &self.current_scene {
                if let Some(scene) = self.scenes.get_mut(current_scene_name) {
                    scene.exit();
                }
            }
            if let Some(scene) = self.scenes.get_mut(&next_scene_name) {
                scene.load();
            }
            self.current_scene = Some(next_scene_name);
        }

        // Update current scene
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get_mut(scene_name) {
                scene.update(dt);
            }
        }
    }

    pub fn render(&mut self) {
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get_mut(scene_name) {
                scene.render();
            }
        }
    }

    pub fn has_scene(&self) -> bool {
        self.current_scene.is_some()
    }
}