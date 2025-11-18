use super::*;
use crate::editor::component::InspectableComponent;
use crate::editor::{
    ViewportMode,
    console_error::{ConsoleItem, ErrorLevel},
};
use crate::plugin::PluginRegistry;
use dropbear_engine::utils::ResourceReference;
use dropbear_engine::{
    entity::{LocalTransform, MeshRenderer, Transform, WorldTransform},
    lighting::{Light, LightComponent},
};
use egui::{self, CollapsingHeader, Id, Margin, RichText};
use egui_dock::TabViewer;
use egui_extras;
use egui_ltreeview::{Action, NodeBuilder, TreeView, TreeViewBuilder};
use eucalyptus_core::hierarchy::Parent;
use eucalyptus_core::spawn::push_pending_spawn;
use eucalyptus_core::states::{
    File, Label, ModelProperties, Node, RESOURCES, ResourceType, SceneEntity,
    SceneMeshRendererComponent,
};
use eucalyptus_core::traits::Component;
use eucalyptus_core::{APP_INFO, utils::ResolveReference};
use log;
use parking_lot::Mutex;
use std::ffi::OsStr;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::{collections::HashMap, path::PathBuf, sync::LazyLock};
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoConfig, GizmoExt, GizmoMode, math::DVec3};

pub struct EditorTabViewer<'a> {
    pub view: egui::TextureId,
    pub tex_size: Extent3d,
    pub gizmo: &'a mut Gizmo,
    pub world: &'a mut World,
    pub selected_entity: &'a mut Option<Entity>,
    pub viewport_mode: &'a mut ViewportMode,
    pub undo_stack: &'a mut Vec<UndoableAction>,
    pub signal: &'a mut Signal,
    pub gizmo_mode: &'a mut EnumSet<GizmoMode>,
    pub editor_mode: &'a mut EditorState,
    pub active_camera: &'a mut Arc<Mutex<Option<Entity>>>,
    pub plugin_registry: &'a mut PluginRegistry,
    pub build_logs: &'a mut Vec<String>,
    // "wah wah its unsafe, its using raw pointers" shut the fuck up if it breaks i will know
    pub editor: *mut Editor,
    pub mel_counter: i64,
}

impl<'a> EditorTabViewer<'a> {
    fn spawn_entity_at_pos(
        &mut self,
        asset: &DraggedAsset,
        position: DVec3,
        properties: Option<ModelProperties>,
    ) -> anyhow::Result<()> {
        let local_transform = LocalTransform::from_transform(Transform {
            position,
            ..Default::default()
        });
        let world_transform = WorldTransform::from_transform(local_transform.into_inner());

        let mut components: Vec<Box<dyn Component>> = Vec::new();

        components.push(Box::new(local_transform));
        components.push(Box::new(world_transform));

        let mesh_comp = SceneMeshRendererComponent {
            model: asset.path.clone(),
            material_overrides: Vec::new(),
        };
        components.push(Box::new(mesh_comp));

        let props = properties.unwrap_or_default();
        components.push(Box::new(props));

        let scene_entity = SceneEntity {
            label: Label::from(asset.name.clone()),
            components,
            parent: Label::default(),
            children: Vec::new(),
            id: None,
        };

        push_pending_spawn(scene_entity);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DraggedAsset {
    pub name: String,
    pub path: ResourceReference,
}

pub static TABS_GLOBAL: LazyLock<Mutex<StaticallyKept>> =
    LazyLock::new(|| Mutex::new(StaticallyKept::default()));

/// Variables kept statically.
///
/// The entire module (including the tab viewer) due to it
/// being part of an update/render function, therefore this is used to ensure
/// progress is not lost.
#[derive(Default)]
pub struct StaticallyKept {
    show_context_menu: bool,
    context_menu_pos: egui::Pos2,
    context_menu_tab: Option<EditorTab>,
    is_focused: bool,
    old_pos: Transform,
    pub(crate) scale_locked: bool,

    pub(crate) old_label_entity: Option<hecs::Entity>,
    pub(crate) label_original: Option<String>,
    pub(crate) label_last_edit: Option<Instant>,

    pub(crate) transform_old_entity: Option<hecs::Entity>,
    pub(crate) transform_original_transform: Option<Transform>,

    pub(crate) transform_in_progress: bool,
    pub(crate) transform_rotation_cache: HashMap<Entity, DVec3>,
}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            EditorTab::Viewport => "Viewport".into(),
            EditorTab::ModelEntityList => "Entity List".into(),
            EditorTab::AssetViewer => "Asset Viewer".into(),
            EditorTab::ResourceInspector => "Resource Inspector".into(),
            EditorTab::Plugin(dock_index) => {
                if let Some((_, plugin)) = self.plugin_registry.plugins.get_index_mut(*dock_index) {
                    plugin.display_name().into()
                } else {
                    "Unknown Plugin Name".into()
                }
            }
            EditorTab::ErrorConsole => "Error Console".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut cfg = TABS_GLOBAL.lock();

        ui.ctx().input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Secondary)
                && let Some(pos) = i.pointer.hover_pos()
                && ui.available_rect_before_wrap().contains(pos)
            {
                cfg.show_context_menu = true;
                cfg.context_menu_pos = pos;
                cfg.context_menu_tab = Some(tab.clone());
            }
        });

        match tab {
            EditorTab::Viewport => {
                log_once::debug_once!("Viewport focused");
                // ------------------- Template for querying active camera -----------------
                // if let Some(active_camera) = self.active_camera {
                //     if let Ok(mut q) = self.world.query_one::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>(*active_camera) {
                //         if let Some((camera, component, follow_target)) = q.get() {

                //         } else {
                //             log_once::warn_once!("Unable to fetch the query result of camera: {:?}", active_camera)
                //         }
                //     } else {
                //         log_once::warn_once!("Unable to query camera, component and option<camerafollowtarget> for active camera: {:?}", active_camera);
                //     }
                // } else {
                //     log_once::warn_once!("No active camera found");
                // }
                // -------------------------------------------------------------------------

                let available_rect = ui.available_rect_before_wrap();
                let available_size = available_rect.size();

                let tex_aspect = self.tex_size.width as f32 / self.tex_size.height as f32;
                let available_aspect = available_size.x / available_size.y;

                let (display_width, display_height) = if available_aspect > tex_aspect {
                    let height = available_size.y * 0.95;
                    let width = height * tex_aspect;
                    (width, height)
                } else {
                    let width = available_size.x * 0.95;
                    let height = width / tex_aspect;
                    (width, height)
                };

                let center_x = available_rect.center().x;
                let center_y = available_rect.center().y;

                let image_rect = egui::Rect::from_center_size(
                    egui::pos2(center_x, center_y),
                    egui::vec2(display_width, display_height),
                );

                let (_rect, _response) =
                    ui.allocate_exact_size(available_size, egui::Sense::click_and_drag());

                let _image_response = ui.allocate_rect(image_rect, egui::Sense::click_and_drag());

                ui.scope_builder(egui::UiBuilder::new().max_rect(image_rect), |ui| {
                    ui.add_sized(
                        [display_width, display_height],
                        egui::Image::new((self.view, [display_width, display_height].into()))
                            .fit_to_exact_size([display_width, display_height].into()),
                    )
                });

                // let image_response = ui.interact(
                //     image_rect,
                //     ui.id().with("viewport_image"),
                //     egui::Sense::click_and_drag(),
                // );

                let snapping = ui.input(|input| input.modifiers.shift);

                // Note to self: fuck you >:(
                // Note to self: ok wow thats pretty rude im trying my best ＞﹏＜
                // Note to self: finally holy shit i got it working
                let active_cam = self.active_camera.lock();
                if let Some(active_camera) = *active_cam {
                    let camera_data = {
                        if let Ok(mut q) = self
                            .world
                            .query_one::<(&Camera, &CameraComponent)>(active_camera)
                        {
                            let val = q.get();
                            if let Some(val) = val {
                                Some(val.0.clone())
                            } else {
                                log::warn!("Queried camera but unable to get value");

                                None
                            }
                        } else {
                            log::warn!("Unable to query camera");
                            None
                        }
                    };

                    if let Some(camera) = camera_data {
                        self.gizmo.update_config(GizmoConfig {
                            view_matrix: camera.view_mat.into(),
                            projection_matrix: camera.proj_mat.into(),
                            viewport: image_rect,
                            modes: *self.gizmo_mode,
                            orientation: transform_gizmo_egui::GizmoOrientation::Global,
                            snapping,
                            snap_distance: 1.0,
                            ..Default::default()
                        });
                    }
                }
                if !matches!(self.viewport_mode, ViewportMode::None)
                    && let Some(entity_id) = self.selected_entity
                {
                    let world_transform = self
                        .world
                        .query_one::<&WorldTransform>(*entity_id)
                        .ok()
                        .and_then(|mut query| query.get().copied());

                    let mut transform = world_transform
                        .map(|wt| wt.into_inner())
                        .or_else(|| {
                            self.world
                                .query_one::<&LocalTransform>(*entity_id)
                                .ok()
                                .and_then(|mut query| query.get().map(|lt| lt.inner().clone()))
                        })
                        .or_else(|| {
                            self.world
                                .query_one::<&Transform>(*entity_id)
                                .ok()
                                .and_then(|mut query| query.get().copied())
                        });

                    if let Some(mut transform) = transform {
                        let was_focused = cfg.is_focused;
                        cfg.is_focused = self.gizmo.is_focused();

                        if cfg.is_focused && !was_focused {
                            cfg.old_pos = transform;
                        }

                        let gizmo_transform =
                            transform_gizmo_egui::math::Transform::from_scale_rotation_translation(
                                transform.scale,
                                transform.rotation,
                                transform.position,
                            );

                        let mut updated_transform: Option<Transform> = None;

                        if let Some((_result, new_transforms)) =
                            self.gizmo.interact(ui, &[gizmo_transform])
                            && let Some(new_transform) = new_transforms.first()
                        {
                            transform.position = new_transform.translation.into();
                            transform.rotation = new_transform.rotation.into();
                            transform.scale = new_transform.scale.into();
                            updated_transform = Some(transform);
                        }

                        if let Some(new_transform) = updated_transform {
                            let mut applied_to_local = false;
                            {
                                if let Ok(mut query) =
                                    self.world.query_one::<&mut LocalTransform>(*entity_id)
                                    && let Some(local_transform) = query.get()
                                {
                                    *local_transform =
                                        LocalTransform::from_transform(new_transform);
                                    applied_to_local = true;
                                }
                            }

                            if !applied_to_local {
                                if let Ok(mut query) =
                                    self.world.query_one::<&mut WorldTransform>(*entity_id)
                                    && let Some(world_transform) = query.get()
                                {
                                    *world_transform =
                                        WorldTransform::from_transform(new_transform);
                                } else if let Ok(mut query) =
                                    self.world.query_one::<&mut Transform>(*entity_id)
                                    && let Some(existing_transform) = query.get()
                                {
                                    *existing_transform = new_transform;
                                }
                            }
                        }

                        if was_focused && !cfg.is_focused {
                            let transform_changed = cfg.old_pos.position != transform.position
                                || cfg.old_pos.rotation != transform.rotation
                                || cfg.old_pos.scale != transform.scale;

                            if transform_changed {
                                UndoableAction::push_to_undo(
                                    self.undo_stack,
                                    UndoableAction::Transform(*entity_id, cfg.old_pos),
                                );
                                log::debug!("Pushed transform action to stack");
                            }
                        }
                    }
                }
            }
            EditorTab::ModelEntityList => {
                let response = TreeView::new(Id::new("model entity list")).show(ui, |builder| {
                    let last_opened_scene = { PROJECT.read().last_opened_scene.clone() };

                    builder.dir(
                        -1,
                        format!("{}", last_opened_scene.unwrap_or("Scene".into())),
                    );

                    fn build_entity_tree(
                        builder: &mut egui_ltreeview::TreeViewBuilder<i32>,
                        entity: &SceneEntity,
                    ) {
                        let id = entity.id.unwrap().id() as i32;
                        builder.dir(id, &*entity.label);

                        for component in &entity.components {
                            builder.leaf(-1 * id, component.type_name());
                        }

                        for child in &entity.children {
                            build_entity_tree(builder, child);
                        }

                        builder.close_dir();
                    }

                    let root_entities: Vec<_> = self
                        .world
                        .query::<(&Label, Option<&Parent>)>()
                        .iter()
                        .filter_map(|(e, (l, p))| {
                            if p.is_none() {
                                Some((e, l.clone()))
                            } else {
                                None
                            }
                        })
                        .collect();

                    for (entity, _label) in root_entities {
                        if let Some(scene_entity) =
                            crate::utils::collect_entity_recursive(&self.world, entity)
                        {
                            build_entity_tree(builder, &scene_entity);
                        }
                    }

                    builder.close_dir()
                });

                for resp in response.1 {
                    match resp {
                        Action::SetSelected(id) => {
                            log::debug!("Entities {:?} have been selected", id);
                            let Some(actual_selected) = id.first() else {
                                continue;
                            };
                            if *actual_selected < 0 {
                                continue; // means that its the root scene dir in the tree
                            }
                            *self.selected_entity = Some(unsafe {
                                self.world.find_entity_from_id(*actual_selected as u32)
                            });
                        }
                        Action::Move(id) => {
                            log::debug!("Entities {:?} have been MOVED", id);
                        }
                        Action::Drag(id) => {
                            log::debug!("Entities {:?} have been DRAGGED", id);
                        }
                        Action::Activate(id) => {
                            log::debug!("Entities {:?} have been ACTIVATED", id);
                        }
                        Action::DragExternal(id) => {
                            log::debug!("Entities {:?} have been EXTERNALLY DRAGGED", id);
                        }
                        Action::MoveExternal(id) => {
                            log::debug!("Entities {:?} have been EXTERNALLY MOVED", id);
                        }
                    }
                }
            }
            EditorTab::AssetViewer => {
                let res = { RESOURCES.read().clone() };
                let mut old_depth: usize = 0;
                egui_ltreeview::TreeView::new(Id::new("asset viewer")).show(
                    ui,
                    |builder: &mut TreeViewBuilder<u64>| {
                        for entry in walkdir::WalkDir::new(res.path.clone()) {
                            let Ok(e) = entry else {
                                continue;
                            };
                            let depth = e.depth();

                            if depth < old_depth {
                                for _ in 0..(old_depth - depth) {
                                    builder.close_dir();
                                }
                            }
                            old_depth = depth;

                            let Some(file_name) = e.file_name().to_str() else {
                                continue;
                            };
                            let str_file_name = file_name.to_string();
                            let mut hasher = DefaultHasher::new();
                            file_name.hash(&mut hasher);
                            let hash = hasher.finish();

                            if e.file_type().is_dir() {
                                builder.node(
                                    NodeBuilder::dir(hash).label(str_file_name), // .icon(add_icon) todo: add folder icon
                                );
                            } else if e.file_type().is_file() {
                                builder.node(
                                    NodeBuilder::leaf(hash).label(str_file_name), // .icon(add_icon) todo: add folder icon
                                );
                            }
                        }
                    },
                );
            }
            EditorTab::ResourceInspector => {
                if let Some(entity) = self.selected_entity {
                    let mut local_set_initial_camera = false;
                    if let Ok(mut q) = self.world.query_one::<(
                        &mut Label,
                        &mut MeshRenderer,
                        Option<&mut LocalTransform>,
                        Option<&mut WorldTransform>,
                        Option<&mut ModelProperties>,
                        Option<&mut ScriptComponent>,
                        Option<&mut Camera>,
                        Option<&mut CameraComponent>,
                        Option<&mut LightComponent>,
                    )>(*entity)
                    {
                        if let Some((
                            label,
                            e,
                            lt,
                            wt,
                            _props,
                            script,
                            camera,
                            camera_component,
                            light_component,
                        )) = q.get()
                        {
                            // entity
                            e.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                label.as_mut_string(),
                            );
                            // transform
                            if let (Some(lt), Some(wt)) = (lt, wt) {
                                lt.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    label.as_mut_string(),
                                );

                                wt.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    label.as_mut_string(),
                                );
                            }

                            // properties
                            if let Some(props) = _props {
                                props.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    label.as_mut_string(),
                                );
                            }

                            // camera
                            if let (Some(camera), Some(camera_component)) =
                                (camera, camera_component)
                            {
                                ui.separator();
                                CollapsingHeader::new("Camera Components")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("Remove Component ❌").clicked() {
                                                    *self.signal = Signal::RemoveComponent(
                                                        *entity,
                                                        Box::new(ComponentType::Camera(
                                                            Box::new(camera.clone()),
                                                            camera_component.clone(),
                                                            // None,
                                                        )),
                                                    );
                                                // }
                                            }
                                        });
                                    });

                                    camera.inspect(
                                        entity,
                                        &mut cfg,
                                        ui,
                                        self.undo_stack,
                                        self.signal,
                                        &mut String::new(),
                                    );
                                    camera_component.camera_type = CameraType::Player; // it will always be a player if attached to an entity, never normal
                                    camera_component.inspect(
                                        entity,
                                        &mut cfg,
                                        ui,
                                        self.undo_stack,
                                        self.signal,
                                        &mut camera.label.clone(),
                                    );

                                    ui.separator();
                                    ui.label("Camera Controls:");
                                    let mut active_camera = self.active_camera.lock();

                                    if *active_camera == Some(*entity) {
                                        ui.label("Status: Currently viewing through camera");
                                    } else {
                                        ui.label("Status: Not viewing through this camera");
                                    }

                                    if ui.button("Set as active camera").clicked() {
                                        *active_camera = Some(*entity);
                                        log::info!("Currently viewing from camera angle '{}'", camera.label);
                                    }

                                    if camera_component.starting_camera {
                                        if ui.button("Stop making camera initial").clicked() {
                                            log::debug!("'Stop making camera initial' button clicked");
                                            camera_component.starting_camera = false;
                                            success!("Removed {} from starting camera", camera.label);
                                        }
                                    } else if ui.button("Set as initial camera").clicked() {
                                        log::debug!("'Set as initial camera' button clicked");
                                        if matches!(camera_component.camera_type, CameraType::Debug) {
                                            warn!("Cannot set any cameras of type 'Debug' to initial camera");
                                        } else {
                                            local_set_initial_camera = true
                                        }
                                    }
                                });
                            }

                            // scripting
                            if let Some(script) = script {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.button("Remove Component ❌").clicked() {
                                                *self.signal = Signal::RemoveComponent(
                                                    *entity,
                                                    Box::new(ComponentType::Script(script.clone())),
                                                )
                                            }
                                        },
                                    );
                                });

                                script.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    label.as_mut_string(),
                                );
                            }

                            // light component
                            if let Some(light) = light_component {
                                ui.separator();
                                CollapsingHeader::new("Light Component")
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("Remove Component ❌").clicked() {
                                                        *self.signal = Signal::RemoveComponent(
                                                            *entity,
                                                            Box::new(ComponentType::Light(
                                                                light.clone(),
                                                            )),
                                                        )
                                                    }
                                                },
                                            );
                                        });

                                        light.inspect(
                                            entity,
                                            &mut cfg,
                                            ui,
                                            self.undo_stack,
                                            self.signal,
                                            label.as_mut_string(),
                                        );
                                    });
                            }

                            if let Some(t) = cfg.label_last_edit
                                && t.elapsed() >= Duration::from_millis(500)
                            {
                                if let Some(ent) = cfg.old_label_entity.take()
                                    && let Some(orig) = cfg.label_original.take()
                                {
                                    UndoableAction::push_to_undo(
                                        self.undo_stack,
                                        UndoableAction::Label(ent, orig, EntityType::Entity),
                                    );
                                    log::debug!(
                                        "Pushed label change to undo stack after 500ms debounce period"
                                    );
                                }
                                cfg.label_last_edit = None;
                            }
                        }
                    } else {
                        log_once::debug_once!("Unable to query entity inside resource inspector");
                    }

                    if local_set_initial_camera {
                        for (id, comp) in self.world.query::<&mut CameraComponent>().iter() {
                            comp.starting_camera = false;
                            self.undo_stack
                                .push(UndoableAction::RemoveStartingCamera(id))
                        }

                        if let Ok(comp) = self.world.query_one_mut::<&mut CameraComponent>(*entity)
                        {
                            success!("This camera is currently set as the initial camera");
                            comp.starting_camera = true;
                        }
                    }

                    // lighting system
                    if let Ok(mut q) = self
                        .world
                        .query_one::<(&mut Light, &mut Transform, &mut LightComponent)>(*entity)
                    {
                        if let Some((light, transform, props)) = q.get() {
                            light.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                &mut String::new(),
                            );
                            transform.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                &mut light.label,
                            );
                            props.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                &mut light.label,
                            );
                            if let Some(t) = cfg.label_last_edit
                                && t.elapsed() >= Duration::from_millis(500)
                            {
                                if let Some(ent) = cfg.old_label_entity.take()
                                    && let Some(orig) = cfg.label_original.take()
                                {
                                    UndoableAction::push_to_undo(
                                        self.undo_stack,
                                        UndoableAction::Label(ent, orig, EntityType::Light),
                                    );
                                    log::debug!(
                                        "Pushed label change to undo stack after 500ms debounce period"
                                    );
                                }
                                cfg.label_last_edit = None;
                            }
                        }
                    } else {
                        log_once::debug_once!("Unable to query light inside resource inspector");
                    }

                    // camera
                    if let Ok(mut q) = self.world.query_one::<(
                        &mut Camera,
                        &mut CameraComponent,
                        // Option<&mut CameraFollowTarget>,
                    )>(*entity)
                        && let Some((camera, camera_component)) = q.get()
                        && self.world.get::<&MeshRenderer>(*entity).is_err()
                    {
                        ui.vertical(|ui| {
                            camera.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                &mut String::new(),
                            );
                            camera_component.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                &mut camera.label.clone(),
                            );

                            ui.separator();

                            let mut active_camera = self.active_camera.lock();

                            ui.label("Camera Controls:");
                            if *active_camera == Some(*entity) {
                                ui.label("Status: Currently viewing through camera");
                            } else {
                                ui.label("Status: Not viewing through this camera");
                            }

                            if ui.button("Set as active camera").clicked() {
                                *active_camera = Some(*entity);
                                log::info!("Currently viewing from camera angle '{}'", camera.label);
                            }

                            if camera_component.starting_camera {
                                if ui.button("Stop making camera initial").clicked() {
                                    log::debug!("'Stop making camera initial' button clicked");
                                    success!("Removed {} from starting camera", camera.label);
                                    camera_component.starting_camera = false;
                                }
                            } else {
                                #[allow(clippy::collapsible_else_if)]
                                if ui.button("Set as initial camera").clicked() {
                                    log::debug!("'Set as initial camera' button clicked");
                                    if matches!(camera_component.camera_type, CameraType::Debug) {
                                        info!("Cannot set any cameras of type 'Debug' to initial camera");
                                    } else {
                                        success!("Set {} at the starting camera. When you start your game, \
                                                expect to see through this camera!", camera.label);
                                        camera_component.starting_camera = true;
                                    }
                                }
                            }
                        });
                    }
                    ui.separator();
                } else {
                    ui.label("No entity selected, therefore no info to provide. Go on, what are you waiting for? Click an entity!");
                }
            }
            EditorTab::Plugin(dock_info) => {
                if self.editor.is_null() {
                    panic!("Editor pointer is null, unexpected behaviour");
                }
                let editor = unsafe { &mut *self.editor };
                if let Some((_, plugin)) = self.plugin_registry.plugins.get_index_mut(*dock_info) {
                    plugin.ui(ui, editor);
                } else {
                    ui.colored_label(
                        egui::Color32::RED,
                        format!("Plugin at index '{}' not found", *dock_info),
                    );
                }
            }
            EditorTab::ErrorConsole => {
                fn analyse_error(log: &Vec<String>) -> Vec<ConsoleItem> {
                    fn parse_compiler_location(
                        line: &str,
                    ) -> Option<(ErrorLevel, PathBuf, String)> {
                        let trimmed = line.trim_start();
                        let (error_level, rest) =
                            if let Some(r) = trimmed.strip_prefix("e: file:///") {
                                (ErrorLevel::Error, r)
                            } else if let Some(r) = trimmed.strip_prefix("w: file:///") {
                                (ErrorLevel::Warn, r)
                            } else {
                                return None;
                            };

                        let location = rest.split_whitespace().next()?;

                        let mut segments = location.rsplitn(3, ':');
                        let column = segments.next()?;
                        let row = segments.next()?;
                        let path = segments.next()?;

                        Some((error_level, PathBuf::from(path), format!("{row}:{column}")))
                    }

                    let mut list: Vec<ConsoleItem> = Vec::new();
                    let index = 0;
                    for line in log {
                        if line.contains("The required library") {
                            list.push(ConsoleItem {
                                error_level: ErrorLevel::Error,
                                msg: line.clone(),
                                file_location: None,
                                line_ref: None,
                                id: index + 1,
                            });
                        }

                        if let Some((error_level, path, loc)) = parse_compiler_location(line) {
                            list.push(ConsoleItem {
                                error_level,
                                msg: line.clone(),
                                file_location: Some(path),
                                line_ref: Some(loc),
                                id: index + 1,
                            });
                        }

                        // thats it for now
                    }
                    list
                }

                let logs = analyse_error(&self.build_logs);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if logs.is_empty() {
                            ui.label("Build output will appear here once available.");
                            return;
                        }

                        for item in &logs {
                            let (bg_color, text_color) = match item.error_level {
                                ErrorLevel::Error => (
                                    egui::Color32::from_rgb(60, 20, 20),
                                    egui::Color32::from_rgb(255, 200, 200),
                                ),
                                ErrorLevel::Warn => (
                                    egui::Color32::from_rgb(40, 40, 10),
                                    egui::Color32::from_rgb(255, 255, 200),
                                ),
                            };

                            let available_width = ui.available_width();
                            let frame = egui::Frame::new()
                                .inner_margin(Margin::symmetric(8, 6))
                                .fill(bg_color)
                                .stroke(egui::Stroke::new(1.0, text_color));

                            let response = frame
                                .show(ui, |ui| {
                                    ui.set_width(available_width - 10.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(&item.msg).color(text_color));
                                    });
                                })
                                .response;

                            if response.clicked() {
                                log::debug!("Log item clicked: {}", &item.id);
                                if let (Some(path), Some(loc)) =
                                    (&item.file_location, &item.line_ref)
                                {
                                    let location_arg = format!("{}:{}", path.display(), loc);

                                    match std::process::Command::new("code")
                                        .args(["-g", &location_arg])
                                        .spawn()
                                        .map(|_| ())
                                    {
                                        Ok(()) => {
                                            log::info!(
                                                "Launched Visual Studio Code at the error: {}",
                                                &location_arg
                                            );
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to open '{}' in VS Code: {}",
                                                location_arg, e
                                            );
                                        }
                                    }
                                }
                            }

                            ui.add_space(4.0);
                        }
                    });
            }
        }

        let mut menu_action: Option<EditorTabMenuAction> = None;
        let area = egui::Area::new("context_menu".into())
            .fixed_pos(cfg.context_menu_pos)
            .order(egui::Order::Foreground);

        if cfg.show_context_menu {
            let menu_tab = cfg
                .context_menu_tab
                .clone()
                .unwrap_or(EditorTab::ModelEntityList);

            let mut popup_rect = None;

            area.show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    popup_rect.replace(ui.max_rect());

                    match menu_tab {
                        EditorTab::AssetViewer => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Import resource").clicked() {
                                menu_action = Some(EditorTabMenuAction::ImportResource);
                            }
                            if ui.selectable_label(false, "Refresh assets").clicked() {
                                menu_action = Some(EditorTabMenuAction::RefreshAssets);
                            }
                        }
                        EditorTab::ModelEntityList => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Add Entity").clicked() {
                                menu_action = Some(EditorTabMenuAction::AddEntity);
                            }
                            if ui.selectable_label(false, "Delete Entity").clicked() {
                                menu_action = Some(EditorTabMenuAction::DeleteEntity);
                            }
                        }
                        EditorTab::ResourceInspector => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Add Component").clicked() {
                                menu_action = Some(EditorTabMenuAction::AddComponent);
                            }
                        }
                        EditorTab::Viewport => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Viewport Option").clicked() {
                                menu_action = Some(EditorTabMenuAction::ViewportOption);
                            }
                        }
                        EditorTab::ErrorConsole => {
                            ui.set_min_width(150.0);
                            ui.label("No actions available");
                        }
                        EditorTab::Plugin(dock_info) => {
                            if self.editor.is_null() {
                                panic!("Editor pointer is null, unexpected behaviour");
                            }

                            let editor = unsafe { &mut *self.editor };
                            if let Some((_, plugin)) =
                                self.plugin_registry.plugins.get_index_mut(dock_info)
                            {
                                plugin.context_menu(ui, editor);
                            } else {
                                ui.colored_label(
                                    egui::Color32::RED,
                                    format!("Plugin at index '{}' not found", dock_info),
                                );
                            }
                        }
                    }
                })
            });

            if let Some(action) = menu_action
                && Some(tab.clone()) == cfg.context_menu_tab
            {
                match action {
                    EditorTabMenuAction::ImportResource => {
                        log::debug!("Import Resource clicked");

                        match import_object() {
                            Ok(_) => {
                                success!("Resource(s) imported successfully!");
                            }
                            Err(e) => {
                                warn!("Failed to import resource(s): {e}");
                            }
                        }
                        cfg.show_context_menu = false;
                        cfg.context_menu_tab = None;
                        return;
                    }
                    EditorTabMenuAction::RefreshAssets => {
                        log::debug!("Refresh assets clicked");
                        {
                            let mut res = RESOURCES.write();
                            match res.update_mem() {
                                Ok(res_cfg) => {
                                    *res = res_cfg;
                                    success!("Assets refreshed successfully!");
                                }
                                Err(e) => {
                                    fatal!("Failed to refresh assets: {}", e);
                                }
                            }
                        }
                        cfg.show_context_menu = false;
                        cfg.context_menu_tab = None;
                        return;
                    }
                    EditorTabMenuAction::AddEntity => {
                        log::debug!("Add Entity clicked");
                        *self.signal = Signal::CreateEntity;
                        cfg.show_context_menu = false;
                        cfg.context_menu_tab = None;
                        return;
                    }
                    EditorTabMenuAction::DeleteEntity => {
                        log::debug!("Delete Entity clicked");
                        *self.signal = Signal::Delete;
                        cfg.show_context_menu = false;
                        cfg.context_menu_tab = None;
                        return;
                    }
                    EditorTabMenuAction::AddComponent => {
                        log::debug!("Add Component clicked");
                        if let Some(entity) = self.selected_entity {
                            {
                                if let Ok(mut q) = self.world.query_one::<&MeshRenderer>(*entity)
                                    && q.get().is_some()
                                {
                                    log::debug!("Queried selected entity, it is an entity");
                                    *self.signal =
                                        Signal::AddComponent(*entity, EntityType::Entity);
                                }
                            }

                            {
                                if let Ok(mut q) = self.world.query_one::<&Light>(*entity)
                                    && q.get().is_some()
                                {
                                    log::debug!("Queried selected entity, it is a light");
                                    *self.signal = Signal::AddComponent(*entity, EntityType::Light);
                                }
                            }
                        } else {
                            warn!(
                                "What are you adding a component to? Theres no entity selected..."
                            );
                        }

                        cfg.show_context_menu = false;
                        cfg.context_menu_tab = None;
                        return;
                    }
                    EditorTabMenuAction::ViewportOption => {
                        log::debug!("Viewport Option clicked");
                        cfg.show_context_menu = false;
                        cfg.context_menu_tab = None;
                        return;
                    }
                    EditorTabMenuAction::RemoveComponent => {
                        log::debug!("Remove Component clicked");
                        if let Some(entity) = self.selected_entity {
                            if let Ok(mut q) = self.world.query_one::<&ScriptComponent>(*entity) {
                                if let Some(script) = q.get() {
                                    log::debug!(
                                        "Queried selected entity, it has a script component"
                                    );
                                    *self.signal = Signal::RemoveComponent(
                                        *entity,
                                        Box::new(ComponentType::Script(script.clone())),
                                    );
                                }
                            } else {
                                warn!("Selected entity does not have a script component to remove");
                            }
                        } else {
                            panic!(
                                "Paradoxical error: Cannot remove a component when its not selected..."
                            );
                        }

                        cfg.show_context_menu = false;
                        cfg.context_menu_tab = None;
                        return;
                    }
                }
            }

            if let Some(rect) = popup_rect
                && cfg.show_context_menu
                && Some(tab.clone()) == cfg.context_menu_tab
                && ui
                    .ctx()
                    .input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
                && let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos())
                && !rect.contains(pos)
            {
                cfg.show_context_menu = false;
                cfg.context_menu_tab = None;
            }
        }
    }
}

pub(crate) fn import_object() -> anyhow::Result<()> {
    let model_ext = vec!["glb", "fbx", "obj"];
    let texture_ext = vec!["png"];

    let files = rfd::FileDialog::new()
        .add_filter("All Files", &["*"])
        .add_filter("Model", &model_ext)
        .add_filter("Texture", &texture_ext)
        .pick_files();
    if let Some(files) = files {
        for file in files {
            let ext = file.extension().unwrap().to_str().unwrap();
            let mut copied = false;
            for mde in model_ext.iter() {
                if ext.contains(mde) {
                    // copy over to models folder
                    {
                        let project = PROJECT.read();
                        let models_dir = project
                            .project_path
                            .clone()
                            .join("resources")
                            .join("models");
                        if !models_dir.exists() {
                            std::fs::create_dir_all(&models_dir)?;
                        }
                        let dest = models_dir.join(file.file_name().unwrap());
                        std::fs::copy(&file, &dest)?;
                        log::info!("Copied model file to {:?}", dest);
                        copied = true;
                    }
                }
            }
            for tex in texture_ext.iter() {
                if ext.contains(tex) {
                    // copy over to textures folder
                    {
                        let project = PROJECT.read();
                        let textures_dir = project
                            .project_path
                            .clone()
                            .join("resources")
                            .join("textures");
                        if !textures_dir.exists() {
                            std::fs::create_dir_all(&textures_dir)?;
                        }
                        let dest = textures_dir.join(file.file_name().unwrap());
                        std::fs::copy(&file, &dest)?;
                        log::info!("Copied texture file to {:?}", dest);
                        copied = true;
                    }
                }
            }

            if !copied {
                {
                    let project = PROJECT.read();
                    // everything else copies over to resources root dir
                    let resources_dir = project.project_path.clone().join("resources");
                    if !resources_dir.exists() {
                        std::fs::create_dir_all(&resources_dir)?;
                    }
                    let dest = resources_dir.join(file.file_name().unwrap());
                    std::fs::copy(&file, &dest)?;
                    log::info!("Copied other resource file to {:?}", dest);
                }
            }
        }
        // save it all to ensure the eucc recognises it
        let mut proj = PROJECT.write();
        proj.write_to_all()?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("File dialogue returned None"))
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
// todo: provide a purpose to RemoveComponent
pub enum EditorTabMenuAction {
    ImportResource,
    RefreshAssets,
    AddEntity,
    DeleteEntity,
    AddComponent,
    RemoveComponent,
    ViewportOption,
}
