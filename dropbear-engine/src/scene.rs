use crate::graphics::Graphics;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub trait Scene {
    fn load(&mut self, graphics: &mut Graphics);
    fn update(&mut self, dt: f32, graphics: &mut Graphics);
    fn render(&mut self, graphics: &mut Graphics);
    fn exit(&mut self);
}

pub type SceneImpl = Rc<RefCell<dyn Scene>>;

pub struct Manager {
    current_scene: Option<String>,
    next_scene: Option<String>,
    scenes: HashMap<String, SceneImpl>,
    scene_input_map: HashMap<String, String>,
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

    pub fn add(&mut self, name: &str, scene: Rc<RefCell<dyn Scene>>) {
        self.scenes.insert(name.to_string(), scene);
    }

    pub fn attach_input(&mut self, scene_name: &str, input_name: &str) {
        self.scene_input_map
            .insert(scene_name.to_string(), input_name.to_string());
    }

    pub fn update(&mut self, dt: f32, graphics: &mut Graphics) {
        // transition scene
        if let Some(next_scene_name) = self.next_scene.take() {
            if let Some(current_scene_name) = &self.current_scene {
                if let Some(scene) = self.scenes.get_mut(current_scene_name) {
                    scene.borrow_mut().exit();
                }
            }
            if let Some(scene) = self.scenes.get_mut(&next_scene_name) {
                scene.borrow_mut().load(graphics);
            }
            self.current_scene = Some(next_scene_name);
        }

        // update scene
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get_mut(scene_name) {
                scene.borrow_mut().update(dt, graphics);
            }
        }
    }

    pub fn render(&mut self, graphics: &mut Graphics) {
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get_mut(scene_name) {
                scene.borrow_mut().render(graphics);
            }
        }
    }

    pub fn has_scene(&self) -> bool {
        self.current_scene.is_some()
    }

    pub fn get_current_scene(&self) -> Option<(&String, &SceneImpl)> {
        if let Some(scene_name) = &self.current_scene {
            if let Some(scene) = self.scenes.get(scene_name) {
                return Some((scene_name, scene));
            }
            return None;
        }
        None
    }
}
