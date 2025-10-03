use super::*;
use crate::editor::ViewportMode;
use std::{collections::HashSet, sync::LazyLock};

use crate::APP_INFO;
use crate::editor::component::InspectableComponent;
use dropbear_engine::{
    entity::Transform,
    lighting::{Light, LightComponent},
};
use egui;
use egui::{CollapsingHeader};
use egui_dock_fork::TabViewer;
use egui_extras;
use eucalyptus_core::states::{Node, RESOURCES, ResourceType};
use log;
use parking_lot::Mutex;
use transform_gizmo_egui::{EnumSet, Gizmo, GizmoConfig, GizmoExt, GizmoMode, math::DVec3};
use dropbear_engine::utils::{ResourceReference, ResourceReferenceType};
use eucalyptus_core::spawn::{push_pending_spawn, PendingSpawn};

pub struct EditorTabViewer<'a> {
    pub view: egui::TextureId,
    pub nodes: Vec<EntityNode>,
    pub tex_size: Extent3d,
    pub gizmo: &'a mut Gizmo,
    pub world: &'a mut World,
    pub selected_entity: &'a mut Option<hecs::Entity>,
    pub viewport_mode: &'a mut ViewportMode,
    pub undo_stack: &'a mut Vec<UndoableAction>,
    pub signal: &'a mut Signal,
    pub gizmo_mode: &'a mut EnumSet<GizmoMode>,
    pub editor_mode: &'a mut EditorState,
    pub active_camera: &'a mut Arc<Mutex<Option<hecs::Entity>>>,
}

impl<'a> EditorTabViewer<'a> {
    fn spawn_entity_at_pos(
        &mut self,
        asset: &DraggedAsset,
        position: DVec3,
        properties: Option<ModelProperties>,
    ) -> anyhow::Result<()> {
        let transform = Transform { position, ..Default::default() };
        {
            if let Some(props) = properties {
                push_pending_spawn(PendingSpawn {
                    asset_path: asset.path.clone(),
                    asset_name: asset.name.clone(),
                    transform,
                    properties: props,
                    handle: None,
                });
            } else {
                push_pending_spawn(PendingSpawn {
                    asset_path: asset.path.clone(),
                    asset_name: asset.name.clone(),
                    transform,
                    properties: ModelProperties::default(),
                    handle: None,
                });
            }
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub struct DraggedAsset {
    pub name: String,
    pub path: ResourceReference,
}

pub static TABS_GLOBAL: LazyLock<Mutex<StaticallyKept>> =
    LazyLock::new(|| Mutex::new(StaticallyKept::default()));

// Separate static for REPL to avoid deadlocks
pub static KOTLIN_REPL: LazyLock<Mutex<Option<crate::editor::repl::KotlinREPL>>> =
    LazyLock::new(|| Mutex::new(None));

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
}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            EditorTab::Viewport => "Viewport".into(),
            EditorTab::ModelEntityList => "Model/Entity List".into(),
            EditorTab::AssetViewer => "Asset Viewer".into(),
            EditorTab::ResourceInspector => "Resource Inspector".into(),
            EditorTab::KotlinREPL => "Kotlin REPL".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut cfg = TABS_GLOBAL.lock();

        ui.ctx().input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Secondary)
                && let Some(pos) = i.pointer.hover_pos()
                    && ui.available_rect_before_wrap().contains(pos) {
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

                let image_response = ui.interact(
                    image_rect,
                    ui.id().with("viewport_image"),
                    egui::Sense::click_and_drag(),
                );

                if image_response.clicked() {
                    let active_cam = self.active_camera.lock();
                    if let Some(active_camera) = *active_cam {
                        {
                            if let Ok(mut q) = self.world.query_one::<(
                                &Camera,
                                &CameraComponent,
                                // Option<&CameraFollowTarget>,
                            )>(active_camera)
                            {
                                if let Some((camera, _)) = q.get() {
                                    if let Some(click_pos) =
                                        ui.ctx().input(|i| i.pointer.interact_pos())
                                    {
                                        let viewport_rect = image_response.rect;
                                        let local_pos = click_pos - viewport_rect.min;

                                        let ndc_x = (2.0 * local_pos.x / viewport_rect.width()) - 1.0;
                                        let ndc_y = 1.0 - (2.0 * local_pos.y / viewport_rect.height());

                                        let view_matrix = glam::DMat4::look_at_lh(
                                            camera.eye,
                                            camera.target,
                                            camera.up,
                                        );

                                        let proj_matrix = glam::DMat4::perspective_lh(
                                            camera.fov_y.to_radians(),
                                            camera.aspect,
                                            camera.znear,
                                            camera.zfar,
                                        );

                                        if !view_matrix.is_finite() {
                                            log::error!("Invalid view matrix");
                                            return;
                                        }
                                        if !proj_matrix.is_finite() {
                                            log::error!("Invalid projection matrix");
                                            return;
                                        }

                                        let view_proj = proj_matrix * view_matrix;
                                        let inv_view_proj = view_proj.inverse();

                                        if !inv_view_proj.is_finite() {
                                            log::error!("Cannot invert view-projection matrix");
                                            return;
                                        }

                                        let ray_start_ndc =
                                            glam::DVec4::new(ndc_x as f64, ndc_y as f64, 0.0, 1.0);
                                        let ray_end_ndc =
                                            glam::DVec4::new(ndc_x as f64, ndc_y as f64, 1.0, 1.0);

                                        let ray_start_world = inv_view_proj * ray_start_ndc;
                                        let ray_end_world = inv_view_proj * ray_end_ndc;

                                        if ray_start_world.w == 0.0 || ray_end_world.w == 0.0 {
                                            log::error!("Invalid homogeneous coordinates");
                                            return;
                                        }

                                        let ray_start = ray_start_world.truncate() / ray_start_world.w;
                                        let ray_end = ray_end_world.truncate() / ray_end_world.w;

                                        if !ray_start.is_finite() || !ray_end.is_finite() {
                                            log::error!(
                                            "Invalid ray points - start: {:?}, end: {:?}",
                                            ray_start,
                                            ray_end
                                        );
                                            return;
                                        }

                                        let ray_direction = (ray_end - ray_start).normalize();

                                        if !ray_direction.is_finite() {
                                            log::error!("Invalid ray direction: {:?}", ray_direction);
                                            return;
                                        }

                                        let (selected_entity_id, entity_count) = {
                                            let mut closest_distance = f64::INFINITY;
                                            let mut selected_entity_id: Option<hecs::Entity> = None;
                                            let mut entity_count = 0;

                                            {
                                                for (entity_id, (transform, _)) in
                                                    self.world.query::<(&Transform, &AdoptedEntity)>().iter()
                                                {
                                                    entity_count += 1;
                                                    let entity_pos = transform.position;
                                                    let sphere_radius = transform.scale.max_element() * 1.5;
                                                    let to_sphere = entity_pos - ray_start;
                                                    let projection = to_sphere.dot(ray_direction);
                                                    if projection > 0.0 {
                                                        let closest_point =
                                                            ray_start + ray_direction * projection;
                                                        let distance_to_sphere =
                                                            (closest_point - entity_pos).length();
                                                        if distance_to_sphere <= sphere_radius {
                                                            let discriminant = sphere_radius * sphere_radius
                                                                - distance_to_sphere * distance_to_sphere;
                                                            if discriminant >= 0.0 {
                                                                let intersection_distance =
                                                                    projection - discriminant.sqrt();
                                                                if intersection_distance < closest_distance {
                                                                    closest_distance = intersection_distance;
                                                                    selected_entity_id = Some(entity_id);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            (selected_entity_id, entity_count)
                                        };

                                        log::debug!("Total entities checked: {}", entity_count);

                                        if !matches!(self.editor_mode, EditorState::Playing) {
                                            if let Some(entity_id) = selected_entity_id {
                                                *self.selected_entity = Some(entity_id);
                                                log::debug!("Selected entity: {:?}", entity_id);
                                            } else if entity_count == 0 {
                                                log::debug!("No entities in world to select");
                                            } else {
                                                log::debug!(
                                                    "No entity hit by ray (checked {} entities)",
                                                    entity_count
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    log_once::warn_once!(
                                    "Unable to fetch the query result of camera: {:?}",
                                    active_camera
                                )
                                }
                            } else {
                                log_once::warn_once!(
                                "Unable to query camera, component and option<camerafollowtarget> for active camera: {:?}",
                                active_camera
                            );
                            }
                        }
                    } else {
                        log_once::warn_once!("No active camera found");
                    }
                }

                let snapping = ui.input(|input| input.modifiers.shift);

                // Note to self: fuck you >:(
                // Note to self: ok wow thats pretty rude im trying my best ＞﹏＜
                // Note to self: finally holy shit i got it working
                let active_cam = self.active_camera.lock();
                if let Some(active_camera) = *active_cam {
                    let camera_data = {
                        if let Ok(mut q) = self.world.query_one::<(&Camera, &CameraComponent)>(active_camera) {
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
                    && let Some(entity_id) = self.selected_entity {
                        {
                            if let Ok(mut q) = self.world
                                .query_one::<&mut Transform>(*entity_id)
                                && let Some(transform) = q.get() {
                                    let was_focused = cfg.is_focused;
                                    cfg.is_focused = self.gizmo.is_focused();

                                    if cfg.is_focused && !was_focused {
                                        cfg.old_pos = *transform;
                                    }

                                    let gizmo_transform =
                                        transform_gizmo_egui::math::Transform::from_scale_rotation_translation(
                                            transform.scale,
                                            transform.rotation,
                                            transform.position,
                                        );

                                    if let Some((_result, new_transforms)) =
                                        self.gizmo.interact(ui, &[gizmo_transform])
                                        && let Some(new_transform) = new_transforms.first() {
                                            transform.position = new_transform.translation.into();
                                            transform.rotation = new_transform.rotation.into();
                                            transform.scale = new_transform.scale.into();
                                        }

                                    if was_focused && !cfg.is_focused {
                                        let transform_changed = cfg.old_pos.position != transform.position
                                            || cfg.old_pos.rotation != transform.rotation
                                            || cfg.old_pos.scale != transform.scale;

                                        if transform_changed {
                                            UndoableAction::push_to_undo(
                                                self.undo_stack,
                                                UndoableAction::Transform(
                                                    *entity_id,
                                                    cfg.old_pos,
                                                ),
                                            );
                                            log::debug!("Pushed transform action to stack");
                                        }
                                    }
                                }
                        }
                    }
            }
            EditorTab::ModelEntityList => {
                ui.label("Model/Entity List");
                show_entity_tree(
                    ui,
                    &mut self.nodes,
                    self.selected_entity,
                    "Model Entity Asset List",
                );
            }
            EditorTab::AssetViewer => {
                egui_extras::install_image_loaders(ui.ctx());

                let mut assets: Vec<(String, String, PathBuf, ResourceType)> = Vec::new();
                {
                    let res = RESOURCES.read();
                    egui_extras::install_image_loaders(ui.ctx());

                    fn recursive_search_nodes_and_attach_thumbnail(
                        res: &Vec<Node>,
                        assets: &mut Vec<(String, String, PathBuf, ResourceType)>,
                        logged: &mut HashSet<String>,
                    ) {
                        for node in res {
                            match node {
                                Node::File(file) => {
                                    if !logged.contains(&file.name) {
                                        logged.insert(file.name.clone());
                                        log::debug!(
                                            "Adding image for {} of type {}",
                                            file.name,
                                            file.resource_type.as_ref().unwrap()
                                        );
                                    }
                                    if let Some(ref res_type) = file.resource_type {
                                        match res_type {
                                            ResourceType::Model => {
                                                let ad_dir = app_dirs2::get_app_root(
                                                    app_dirs2::AppDataType::UserData,
                                                    &APP_INFO,
                                                )
                                                .unwrap();

                                                let model_thumbnail =
                                                    ad_dir.join(format!("{}.png", file.name));

                                                if !model_thumbnail.exists() {
                                                    // gen image
                                                    log::debug!(
                                                        "Model thumbnail [{}] does not exist, generating one now",
                                                        file.name
                                                    );
                                                    let mut model = match model_to_image::ModelToImageBuilder::new(&file.path)
                                                    .with_size((600, 600))
                                                    .build() {
                                                        Ok(v) => v,
                                                        Err(e) => panic!("Error occurred while loading file from path: {}", e),
                                                    };
                                                    if let Err(e) =
                                                        model.render().unwrap().write_to(Some(
                                                            &ad_dir
                                                                .join(format!("{}.png", file.name)),
                                                        ))
                                                    {
                                                        log::error!(
                                                            "Failed to write model thumbnail for {}: {}",
                                                            file.name,
                                                            e
                                                        );
                                                    }
                                                }

                                                let image_uri =
                                                    model_thumbnail.to_string_lossy().to_string();

                                                assets.push((
                                                    format!("file://{}", image_uri),
                                                    file.name.clone(),
                                                    file.path.clone(),
                                                    res_type.clone(),
                                                ))
                                            }
                                            ResourceType::Texture => assets.push((
                                                format!(
                                                    "file://{}",
                                                    file.path.to_string_lossy()
                                                ),
                                                file.name.clone(),
                                                file.path.clone(),
                                                res_type.clone(),
                                            )),
                                            _ => {
                                                if file
                                                    .path
                                                    .clone()
                                                    .extension()
                                                    .unwrap()
                                                    .to_str()
                                                    .unwrap()
                                                    .contains("euc")
                                                {
                                                    continue;
                                                }
                                                assets.push((
                                                    "NO_TEXTURE".into(),
                                                    file.name.clone(),
                                                    file.path.clone(),
                                                    res_type.clone(),
                                                ))
                                            }
                                        }
                                    }
                                }
                                Node::Folder(folder) => {
                                    recursive_search_nodes_and_attach_thumbnail(
                                        &folder.nodes,
                                        assets,
                                        logged,
                                    )
                                }
                            }
                        }
                    }

                    let mut logged = LOGGED.lock();
                    recursive_search_nodes_and_attach_thumbnail(
                        &res.nodes,
                        &mut assets,
                        &mut logged,
                    );
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let max_columns = 6;
                    let available_width = ui.clip_rect().width() - ui.spacing().indent;
                    let margin = 16.0;
                    let usable_width = available_width - margin;
                    let label_height = 20.0;
                    let padding = 8.0;
                    let min_card_width = 60.0;

                    let mut columns = max_columns;
                    for test_columns in (1..=max_columns).rev() {
                        let card_width = usable_width / test_columns as f32;
                        if card_width >= min_card_width {
                            columns = test_columns;
                            break;
                        }
                    }

                    let card_width = usable_width / columns as f32;
                    let image_size = (card_width - label_height - padding).max(32.0);
                    let card_height = image_size + label_height + padding;

                    for row_start in (0..assets.len()).step_by(columns) {
                        let row_end = (row_start + columns).min(assets.len());
                        let row_items = &mut assets[row_start..row_end];

                        ui.horizontal(|ui| {
                            ui.set_max_width(usable_width);

                            egui_dnd::dnd(ui, format!("asset_row_{}", row_start / columns))
                                .show_vec(
                                    row_items,
                                    |ui, (image, asset_name, asset_path, asset_type), handle, state| {
                                        let card_size = egui::vec2(card_width, card_height);
                                        handle.ui(ui, |ui| {
                                            let (rect, card_response) = ui.allocate_exact_size(
                                                card_size,
                                                egui::Sense::click(),
                                            );

                                            let mut card_ui = ui.new_child(
                                                egui::UiBuilder::new().max_rect(rect).layout(
                                                    egui::Layout::top_down(egui::Align::Center),
                                                ),
                                            );

                                            let image_response = card_ui.add_sized(
                                                [image_size, image_size],
                                                egui::ImageButton::new(image.clone()).frame(false),
                                            );

                                            let is_hovered = card_response.hovered() || image_response.hovered() || state.dragged;
                                            let is_d_clicked = card_response.double_clicked() || image_response.double_clicked();

                                            if is_d_clicked
                                                && matches!(asset_type, ResourceType::Model) {
                                                    let mut spawn_position = DVec3::default();
                                                    {
                                                        let active_cam = self.active_camera.lock();
                                                        if let Some(active_camera) = *active_cam {
                                                            if let Ok(mut q) = self.world.query_one::<(&Camera, &CameraComponent)>(active_camera) {
                                                                if let Some((camera, _)) = q.get() {
                                                                    spawn_position = camera.eye;
                                                                } else {
                                                                    log_once::warn_once!("Unable to fetch the query result of camera: {:?}", active_camera)
                                                                }
                                                            } else {
                                                                log_once::warn_once!("Unable to query camera, component and option<camerafollowtarget> for active camera: {:?}", active_camera);
                                                            }
                                                        } else {
                                                            log_once::warn_once!("No active camera found");
                                                        }
                                                    }

                                                    let asset = DraggedAsset {
                                                        name: asset_name.clone(),
                                                        path: if asset_path.starts_with("resources/") {
                                                            ResourceReference {
                                                                ref_type: ResourceReferenceType::File(
                                                                    asset_path.strip_prefix("resources/")
                                                                        .unwrap_or(&asset_path)
                                                                        .to_string_lossy()
                                                                        .to_string()
                                                                ),
                                                            }
                                                        } else {
                                                            ResourceReference::from_path(asset_path.clone()).unwrap_or_else(|_e| {
                                                                log::warn!("Unable to create ResourceReference from path: {:?}", asset_path);
                                                                Default::default() 
                                                            })
                                                        },
                                                    };

                                                    match self.spawn_entity_at_pos(&asset, spawn_position, None) {
                                                        Ok(()) => {
                                                            log::debug!("double click spawned {} at camera pos {:?}",
                                                                asset.name, spawn_position
                                                            );

                                                            success!("Spawned {} at camera", asset.name);
                                                        }
                                                        Err(e) => {
                                                            log::error!(
                                                            "Failed to spawn {} at camera: {}",
                                                            asset.name,
                                                            e);

                                                            fatal!("Failed to spawn {}: {}",
                                                                        asset.name, e);
                                                        }
                                                    }
                                                }

                                            if is_hovered || state.dragged {
                                                ui.painter().rect_filled(
                                                    rect,
                                                    6.0,
                                                    if state.dragged {
                                                        egui::Color32::from_rgb(80, 80, 100)
                                                    } else {
                                                        egui::Color32::from_rgb(60, 60, 80)
                                                    },
                                                );
                                            }

                                            card_ui.vertical_centered(|ui| {
                                                ui.label(
                                                    egui::RichText::new(asset_name.clone())
                                                        .strong()
                                                        .color(egui::Color32::WHITE),
                                                );
                                            });
                                        });
                                    },
                                );
                        });
                        ui.add_space(8.0);
                    }
                });
            }
            EditorTab::ResourceInspector => {
                if let Some(entity) = self.selected_entity {
                    if let Ok(mut q) = self.world.query_one::<(
                        &mut AdoptedEntity,
                        Option<&mut Transform>,
                        Option<&mut ModelProperties>,
                        Option<&mut ScriptComponent>,
                        Option<&mut Camera>,
                        Option<&mut CameraComponent>,
                        // Option<&mut CameraFollowTarget>,
                    )>(*entity)
                    {
                        if let Some((
                            e,
                            transform,
                            _props,
                            script,
                            camera,
                            camera_component,
                            // follow_target,
                        )) = q.get() {
                            // entity
                            e.inspect(
                                entity,
                                &mut cfg,
                                ui,
                                self.undo_stack,
                                self.signal,
                                &mut String::new(),
                            );
                            // transform
                            if let Some(t) = transform {
                                t.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    &mut Arc::make_mut(&mut e.model).label,
                                );
                            }

                            // properties
                            if let Some(props) = _props {
                                props.inspect(entity, &mut cfg, ui, self.undo_stack, self.signal, &mut Arc::make_mut(&mut e.model).label);
                            }

                            // camera
                            if let (Some(camera), Some(camera_component)) = (camera, camera_component) {
                                ui.separator();
                                CollapsingHeader::new("Camera Components")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.button("Remove Component ❌").clicked() {
                                                // if let Some(target) = &follow_target {
                                                //     *self.signal = Signal::RemoveComponent(
                                                //         *entity,
                                                //         Box::new(ComponentType::Camera(
                                                //             Box::new(camera.clone()),
                                                //             camera_component.clone(),
                                                //             Some((*target).to_owned()),
                                                //         )),
                                                //     );
                                                // } else {
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

                                    // if let Some(target) = follow_target {
                                    //     target.inspect(
                                    //         entity,
                                    //         &mut cfg,
                                    //         ui,
                                    //         self.undo_stack,
                                    //         self.signal,
                                    //         &mut camera.label.clone(),
                                    //     );
                                    // }

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
                                    } else {
                                        #[allow(clippy::collapsible_else_if)] // I don't even know why the fuck clippy wants to flag this...
                                        if ui.button("Set as initial camera").clicked() {
                                            log::debug!("'Set as initial camera' button clicked");
                                            if matches!(camera_component.camera_type, CameraType::Debug) {
                                                warn!("Cannot set any cameras of type 'Debug' to initial camera");
                                            } else {
                                                success!("Set {} at the starting camera. When you start your game, \
                                                expect to see through this camera!", camera.label);
                                                camera_component.starting_camera = true;
                                            }
                                        }
                                    }

                                    // if ui.button("Create offset").clicked() {
                                    //     local_insert_follow_target = Some(*entity);
                                    // }
                                });
                            }

                            // scripting
                            if let Some(script) = script {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Remove Component ❌").clicked() {
                                            *self.signal = Signal::RemoveComponent(*entity, Box::new(ComponentType::Script(script.clone())))
                                        }
                                    });
                                });

                                script.inspect(
                                    entity,
                                    &mut cfg,
                                    ui,
                                    self.undo_stack,
                                    self.signal,
                                    &mut Arc::make_mut(&mut e.model).label,
                                );
                            }

                            if let Some(t) = cfg.label_last_edit
                                && t.elapsed() >= Duration::from_millis(500) {
                                    if let Some(ent) = cfg.old_label_entity.take()
                                        && let Some(orig) = cfg.label_original.take() {
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

                    // if let Some(e) = local_insert_follow_target {
                    //     match self.world.insert_one(e, CameraFollowTarget::default()) {
                    //         Ok(_) => {
                    //             success!("Added CameraFollowTarget");
                    //         }
                    //         Err(e) => {
                    //             warn!("Unable to add CameraFollowTarget: {}", e);
                    //         }
                    //     }
                    // }

                    // lighting system
                    if let Ok(mut q) = self.world
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
                                && t.elapsed() >= Duration::from_millis(500) {
                                    if let Some(ent) = cfg.old_label_entity.take()
                                        && let Some(orig) = cfg.label_original.take() {
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
                    if let Ok(mut q) =
                        self.world.query_one::<(
                            &mut Camera,
                            &mut CameraComponent,
                            // Option<&mut CameraFollowTarget>,
                        )>(*entity)
                        && let Some((camera, camera_component)) = q.get()
                        && self.world.get::<&AdoptedEntity>(*entity).is_err() {
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
                            // if let Some(target) = follow_target {
                            //     target.inspect(
                            //         entity,
                            //         &mut cfg,
                            //         ui,
                            //         self.undo_stack,
                            //         self.signal,
                            //         &mut camera.label.clone(),
                            //     );
                            // }

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
            EditorTab::KotlinREPL => {
                // Use separate static to avoid deadlock with TABS_GLOBAL
                let mut repl_lock = KOTLIN_REPL.lock();
                if repl_lock.is_none() {
                    *repl_lock = Some(crate::editor::repl::KotlinREPL::new());
                }
                
                if let Some(repl) = repl_lock.as_mut() {
                    repl.ui(ui);
                }
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
                        EditorTab::KotlinREPL => {
                            // No context menu actions for REPL tab yet
                        }
                    }
                })
            });

            if let Some(action) = menu_action
                && Some(tab.clone()) == cfg.context_menu_tab {
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
                                    if let Ok(mut q) = self.world
                                        .query_one::<&AdoptedEntity>(*entity)
                                        && q.get().is_some() {
                                            log::debug!("Queried selected entity, it is an entity");
                                            *self.signal =
                                                Signal::AddComponent(*entity, EntityType::Entity);
                                        }
                                }

                                {
                                    if let Ok(mut q) = self.world
                                        .query_one::<&Light>(*entity)
                                        && q.get().is_some() {
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
                                if let Ok(mut q) = self.world
                                    .query_one::<&ScriptComponent>(*entity)
                                {
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
                                    warn!(
                                        "Selected entity does not have a script component to remove"
                                    );
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
                && cfg.show_context_menu && Some(tab.clone()) == cfg.context_menu_tab
                    && ui
                        .ctx()
                        .input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
                        && let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos())
                            && !rect.contains(pos) {
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
                        let models_dir = project.project_path.clone()
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
                        let textures_dir = project.project_path.clone()
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
                    let resources_dir =
                        project.project_path.clone().join("resources");
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
