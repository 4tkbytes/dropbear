use anyhow::anyhow;
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{LocalTransform, MeshRenderer, WorldTransform};
use dropbear_engine::scene::SceneCommand;
use egui::Context;
use egui_toast::{Toast, ToastOptions, Toasts};
use eucalyptus_core::camera::CameraComponent;
use eucalyptus_core::hierarchy::Parent;
use eucalyptus_core::states::{CameraConfig, Label, ModelProperties, PROJECT, ProjectConfig, SceneEntity, SceneMeshRendererComponent, ScriptComponent};
use eucalyptus_core::traits::Component;
use eucalyptus_core::utils::ProjectProgress;
use git2::Repository;
use hecs::{Entity, World};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

/// Helper function to collect all components from an entity into a component vector
pub fn collect_entity_components(world: &World, entity_id: Entity) -> Vec<Box<dyn Component>> {
    let mut components: Vec<Box<dyn Component>> = Vec::new();

    if let Ok(mut query) = world.query_one::<&LocalTransform>(entity_id) {
        if let Some(lt) = query.get() {
            components.push(Box::new(*lt));
        }
    }

    if let Ok(mut query) = world.query_one::<&WorldTransform>(entity_id) {
        if let Some(wt) = query.get() {
            components.push(Box::new(*wt));
        }
    }

    if let Ok(mut query) = world.query_one::<&MeshRenderer>(entity_id) {
        if let Some(renderer) = query.get() {
            let mesh_comp = SceneMeshRendererComponent {
                model: renderer.handle().path.clone(),
                material_overrides: renderer.material_overrides().to_vec(),
            };
            components.push(Box::new(mesh_comp));
        }
    }

    if let Ok(mut query) = world.query_one::<&ModelProperties>(entity_id) {
        if let Some(props) = query.get() {
            components.push(Box::new(props.clone()));
        }
    }

    if let Ok(mut query) = world.query_one::<&ScriptComponent>(entity_id) {
        if let Some(script) = query.get() {
            components.push(Box::new(script.clone()));
        }
    }

    if let Ok(mut query) = world.query_one::<(&Camera, &CameraComponent)>(entity_id) {
        if let Some((camera, component)) = query.get() {
            let camera_config = CameraConfig::from_ecs_camera(camera, component);
            components.push(Box::new(camera_config));
        }
    }

    if let Ok(mut query) = world.query_one::<&dropbear_engine::lighting::LightComponent>(entity_id) {
        if let Some(light_comp) = query.get() {
            components.push(Box::new(light_comp.clone()));
        }
    }

    components
}

/// Collect a single entity as a SceneEntity (without recursing into children)
pub fn collect_entity(world: &World, entity_id: Entity) -> Option<SceneEntity> {
    let entity_label = if let Ok(mut query) = world.query_one::<&Label>(entity_id) {
        query.get().cloned()
    } else {
        return None;
    }?;

    let components = collect_entity_components(world, entity_id);

    Some(SceneEntity {
        label: entity_label,
        components,
        parent: Label::default(),
        children: Vec::new(),
    })
}

/// Recursively collect an entity and all its children as SceneEntity objects
pub fn collect_entity_recursive(world: &World, entity_id: Entity) -> Option<SceneEntity> {
    let entity_label = if let Ok(mut query) = world.query_one::<&Label>(entity_id) {
        query.get().cloned()
    } else {
        return None;
    }?;

    let components = collect_entity_components(world, entity_id);

    let mut children_entities = Vec::new();
    if let Ok(mut query) = world.query_one::<&Parent>(entity_id) {
        if let Some(parent_component) = query.get() {
            for &child_entity in parent_component.children() {
                if let Some(child_scene_entity) = collect_entity_recursive(world, child_entity) {
                    log::debug!("Recursively saved child '{}' of '{}'", child_scene_entity.label, entity_label);
                    children_entities.push(child_scene_entity);
                } else {
                    log::warn!("Unable to collect child entity {:?} for parent '{}'", child_entity, entity_label);
                }
            }
        }
    }

    Some(SceneEntity {
        label: entity_label,
        components,
        parent: Label::default(),
        children: children_entities,
    })
}


pub fn show_new_project_window<F>(
    ctx: &Context,
    show_new_project: &mut bool,
    project_name: &mut String,
    project_path: &mut Option<PathBuf>,
    on_create: F,
) where
    F: FnOnce(&str, &PathBuf),
{
    let screen_size = egui::vec2(400.0, 220.0);

    let mut open = *show_new_project;
    egui::Window::new("Create new project")
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .fixed_size(screen_size)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Project Name:");
                ui.add_space(5.0);

                ui.text_edit_singleline(project_name);
                ui.add_space(10.0);

                ui.heading("Project Location: ");
                ui.add_space(5.0);

                if let Some(path) = project_path {
                    ui.label(format!("Chosen location: {}", path.display()));
                    ui.add_space(5.0);
                }

                ui.add_space(5.0);
                if ui.button("Choose Location").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .set_title("Save Project")
                        .set_file_name(project_name.clone())
                        .pick_folder()
                {
                    *project_path = Some(path);
                }

                let can_create = project_path.is_some() && !project_name.is_empty();
                if ui
                    .add_enabled(can_create, egui::Button::new("Create Project"))
                    .clicked()
                {
                    if let Some(path) = project_path {
                        on_create(project_name, path);
                    }
                    ui.ctx().request_repaint();
                }
            });
        });
    *show_new_project = open;
}

/// Converts a click on a screen (like a viewport) coordinate relative to the world
#[allow(dead_code)]
pub fn screen_to_world_coords(
    camera: &Camera,
    screen_pos: egui::Pos2,
    viewport_rect: egui::Rect,
) -> (glam::DVec3, glam::DVec3) {
    let viewport_width = viewport_rect.width() as f64;
    let viewport_height = viewport_rect.height() as f64;

    let ndc_x = 2.0 * (screen_pos.x as f64 - viewport_rect.min.x as f64) / viewport_width - 1.0;
    let ndc_y = 1.0 - 2.0 * (screen_pos.y as f64 - viewport_rect.min.y as f64) / viewport_height;

    let inv_view = camera.view_mat.inverse();
    let inv_proj = camera.proj_mat.inverse();

    let clip_near = glam::DVec4::new(ndc_x, ndc_y, 0.0, 1.0);
    let clip_far = glam::DVec4::new(ndc_x, ndc_y, 1.0, 1.0);

    let view_near = inv_proj * clip_near;
    let view_far = inv_proj * clip_far;

    let world_near = inv_view * glam::DVec4::new(view_near.x, view_near.y, view_near.z, 1.0);
    let world_far = inv_view * glam::DVec4::new(view_far.x, view_far.y, view_far.z, 1.0);

    let world_near = world_near.truncate() / world_near.w;
    let world_far = world_far.truncate() / world_far.w;

    (world_near, world_far)
}

/// Start creating a new project in a background thread.
/// Returns a Receiver for progress updates.
pub fn start_project_creation(
    project_name: String,
    project_path: Option<PathBuf>,
) -> Option<Receiver<ProjectProgress>> {
    let (tx, rx) = mpsc::channel();
    let project_path = project_path.clone();

    std::thread::spawn(move || {
        let folders = [
            ("git", 0.1, "Creating a git folder..."),
            ("src", 0.2, "Creating src folder..."),
            ("resources/models", 0.4, "Creating models folder..."),
            ("resources/shaders", 0.6, "Creating shader folder..."),
            ("resources/textures", 0.8, "Creating textures folder..."),
            ("src2", 0.9, "Creating project config file..."),
        ];

        if let Some(path) = &project_path {
            for (folder, progress, message) in folders {
                tx.send(ProjectProgress::Step {
                    _progress: progress,
                    _message: message.to_string(),
                })
                .ok();

                let full_path = path.join(folder);
                let result: anyhow::Result<()> = if folder == "src" {
                    if !full_path.exists() {
                        fs::create_dir(&full_path)
                            .map_err(|e| anyhow::anyhow!(e))
                            .map(|_| ())
                    } else {
                        Ok(())
                    }
                } else if folder == "git" {
                    match Repository::init(path) {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            if matches!(e.code(), git2::ErrorCode::Exists) {
                                Ok(())
                            } else {
                                Err(anyhow!(e))
                            }
                        }
                    }
                } else if folder == "src2" {
                    if let Some(path) = &project_path {
                        let mut config = ProjectConfig::new(project_name.clone(), path);
                        let _ = config.write_to_all();
                        let mut global = PROJECT.write();
                        *global = config;
                        Ok(())
                    } else {
                        Err(anyhow!("Project path not found"))
                    }
                } else if !full_path.exists() {
                    fs::create_dir_all(&full_path)
                        .map_err(|e| anyhow!(e))
                        .map(|_| ())
                } else {
                    Ok(())
                };
                if let Err(e) = result {
                    tx.send(ProjectProgress::Error(e.to_string())).ok();
                }
            }
            tx.send(ProjectProgress::Step {
                _progress: 1.0,
                _message: "Project creation complete!".to_string(),
            })
            .ok();
            tx.send(ProjectProgress::Done).ok();
        }
    });

    Some(rx)
}

#[allow(dead_code)]
pub fn open_project(
    scene_command: &mut SceneCommand,
    toast: &mut Toasts,
) -> Result<Option<SceneCommand>, String> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Eucalyptus Project Configuration Files", &["eucp"])
        .pick_file()
    {
        match ProjectConfig::read_from(&path) {
            Ok(config) => {
                let mut global = PROJECT.write();
                *global = config;
                *scene_command = SceneCommand::SwitchScene("editor".to_string());
                Ok(Some(SceneCommand::SwitchScene("editor".to_string())))
            }
            Err(e) => {
                if e.to_string().contains("missing field") {
                    toast.add(Toast {
                        kind: egui_toast::ToastKind::Error,
                        text: "Project version is not up to date.".into(),
                        options: ToastOptions::default()
                            .duration_in_seconds(5.0)
                            .show_progress(true),
                        ..Default::default()
                    });
                }
                Err(format!("Failed to load project: {e}"))
            }
        }
    } else {
        Err("File dialog returned None".to_string())
    }
}
