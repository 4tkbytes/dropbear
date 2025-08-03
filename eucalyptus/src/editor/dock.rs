use super::*;
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};

use dropbear_engine::entity::Transform;
use egui::{self};
use egui_dock_fork::TabViewer;
use egui_extras;
use egui_toast_fork::{Toast, ToastKind};
use log;
use serde::{Deserialize, Serialize};
use transform_gizmo_egui::{
    Gizmo, GizmoConfig, GizmoExt, GizmoMode,
    math::{DMat4, DVec3},
};

use crate::{
    APP_INFO,
    states::{EntityNode, Node, RESOURCES, ResourceType},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EditorTab {
    AssetViewer,       // bottom side,
    ResourceInspector, // left side,
    ModelEntityList,   // right side,
    Viewport,          // middle,
}

pub struct EditorTabViewer<'a> {
    pub view: egui::TextureId,
    pub nodes: Vec<EntityNode>,
    pub tex_size: Extent3d,
    pub gizmo: &'a mut Gizmo,
    pub camera: &'a mut Camera,
    pub resize_signal: &'a mut (bool, u32, u32),
    pub world: &'a mut hecs::World,
    pub selected_entity: &'a mut Option<hecs::Entity>,
}

impl<'a> EditorTabViewer<'a> {
    fn screen_to_world_coords(
        &self,
        screen_pos: egui::Pos2,
        viewport_rect: egui::Rect,
    ) -> (glam::DVec3, glam::DVec3) {
        let viewport_width = viewport_rect.width() as f64;
        let viewport_height = viewport_rect.height() as f64;

        let ndc_x = 2.0 * (screen_pos.x as f64 - viewport_rect.min.x as f64) / viewport_width - 1.0;
        let ndc_y =
            1.0 - 2.0 * (screen_pos.y as f64 - viewport_rect.min.y as f64) / viewport_height;

        let inv_view = self.camera.view_mat.inverse();
        let inv_proj = self.camera.proj_mat.inverse();

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

    fn debug_camera_state(&self) {
        log::debug!("Camera state:");
        log::debug!("  Eye: {:?}", self.camera.eye);
        log::debug!("  Target: {:?}", self.camera.target);
        log::debug!("  Up: {:?}", self.camera.up);
        log::debug!("  FOV Y: {}", self.camera.fov_y);
        log::debug!("  Aspect: {}", self.camera.aspect);
        log::debug!("  Z Near: {}", self.camera.znear);
        log::debug!("  Proj Mat finite: {}", self.camera.proj_mat.is_finite());
        log::debug!("  View Mat finite: {}", self.camera.view_mat.is_finite());
    }
}

pub static TABS_GLOBAL: LazyLock<Mutex<INeedABetterNameForThisStruct>> =
    LazyLock::new(|| Mutex::new(INeedABetterNameForThisStruct::default()));

pub(crate) struct INeedABetterNameForThisStruct {
    show_context_menu: bool,
    context_menu_pos: egui::Pos2,
    context_menu_tab: Option<EditorTab>,
}

impl Default for INeedABetterNameForThisStruct {
    fn default() -> Self {
        Self {
            show_context_menu: Default::default(),
            context_menu_pos: Default::default(),
            context_menu_tab: Default::default(),
        }
    }
}

impl INeedABetterNameForThisStruct {}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = EditorTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            EditorTab::Viewport => "Viewport".into(),
            EditorTab::ModelEntityList => "Model/Entity List".into(),
            EditorTab::AssetViewer => "Asset Viewer".into(),
            EditorTab::ResourceInspector => "Resource Inspector".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let mut cfg = TABS_GLOBAL.lock().unwrap();

        ui.ctx().input(|i| {
            if i.pointer.button_pressed(egui::PointerButton::Secondary) {
                if let Some(pos) = i.pointer.hover_pos() {
                    if ui.available_rect_before_wrap().contains(pos) {
                        cfg.show_context_menu = true;
                        cfg.context_menu_pos = pos;
                        cfg.context_menu_tab = Some(tab.clone());
                    }
                }
            }
        });

        match tab {
            EditorTab::Viewport => {
                let size = ui.available_size();
                let new_tex_width = size.x.max(1.0) as u32;
                let new_tex_height = size.y.max(1.0) as u32;

                if self.tex_size.width != new_tex_width || self.tex_size.height != new_tex_height {
                    *self.resize_signal = (true, new_tex_width, new_tex_height);

                    self.tex_size.width = new_tex_width;
                    self.tex_size.height = new_tex_height;

                    let new_aspect = new_tex_width as f64 / new_tex_height as f64;
                    self.camera.aspect = new_aspect;
                }

                let image_response = ui.add(
                    egui::Image::new((
                        self.view,
                        [self.tex_size.width as f32, self.tex_size.height as f32].into(),
                    ))
                    .sense(egui::Sense::click_and_drag()),
                );

                if image_response.clicked() {
                    if let Some(click_pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                        self.debug_camera_state();

                        let (ray_origin, ray_dir) =
                            self.screen_to_world_coords(click_pos, image_response.rect);
                        log::debug!(
                            "Click pos: {:?}, viewport rect: {:?}",
                            click_pos,
                            image_response.rect
                        );
                        log::debug!("Ray origin: {:?}, direction: {:?}", ray_origin, ray_dir);
                    }
                }

                // TODO: Figure out how to get the guizmos working because this is fucking annoying to deal with
                // Note to self: fuck you >:(
                // Note to self: ok wow thats pretty rude im trying my best ＞﹏＜
                // Note to self: finally holy shit i got it working
                self.gizmo.update_config(GizmoConfig {
                    view_matrix: DMat4::look_at_lh(
                        DVec3::new(
                            self.camera.eye.x as f64,
                            self.camera.eye.y as f64,
                            self.camera.eye.z as f64,
                        ),
                        DVec3::new(
                            self.camera.target.x as f64,
                            self.camera.target.y as f64,
                            self.camera.target.z as f64,
                        ),
                        DVec3::new(
                            self.camera.up.x as f64,
                            self.camera.up.y as f64,
                            self.camera.up.z as f64,
                        ),
                    )
                    .into(),
                    projection_matrix: DMat4::perspective_infinite_reverse_lh(
                        self.camera.fov_y as f64,
                        self.camera.aspect as f64,
                        self.camera.znear as f64,
                    )
                    .into(),
                    viewport: image_response.rect,
                    modes: GizmoMode::all(),
                    orientation: transform_gizmo_egui::GizmoOrientation::Global,
                    ..Default::default()
                });

                if let Some(entity_id) = self.selected_entity {
                    if let Ok(transform) = self.world.query_one_mut::<&mut Transform>(*entity_id) {
                        let gizmo_transform =
                            transform_gizmo_egui::math::Transform::from_scale_rotation_translation(
                                transform.scale,
                                transform.rotation,
                                transform.position,
                            );

                        if let Some((_result, new_transforms)) =
                            self.gizmo.interact(ui, &[gizmo_transform])
                        {
                            if let Some(new_transform) = new_transforms.first() {
                                transform.position = new_transform.translation.into();
                                transform.rotation = new_transform.rotation.into();
                                transform.scale = new_transform.scale.into();
                            }
                        }
                    }
                }
            }
            EditorTab::ModelEntityList => {
                ui.label("Model/Entity List");
                // TODO: deal with show_entity_tree and figure out how to convert hecs::World
                // to EntityNodes and to write it to file
                show_entity_tree(
                    ui,
                    &mut self.nodes,
                    &mut self.selected_entity,
                    "Model Entity Asset List",
                );
            }
            EditorTab::AssetViewer => {
                egui_extras::install_image_loaders(ui.ctx());

                let mut assets: Vec<(String, String)> = Vec::new();
                {
                    let res = RESOURCES.read().unwrap();

                    fn recursive_search_nodes_and_attach_thumbnail(
                        res: &Vec<Node>,
                        assets: &mut Vec<(String, String)>,
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
                                            crate::states::ResourceType::Model => {
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
                                                // let image = egui::Image::from_uri(format!(
                                                //     "file://{}",
                                                //     image_uri
                                                // ));

                                                assets.push((
                                                    format!("file://{}", image_uri),
                                                    file.name.clone(),
                                                ))
                                            }
                                            ResourceType::Texture => {
                                                // let image = egui::Image::from_bytes(
                                                //     file.name.clone(),
                                                //     std::fs::read(&file.path)
                                                //         .unwrap_or(NO_TEXTURE.to_vec()),
                                                // );
                                                assets.push((
                                                    file.path.to_string_lossy().to_string(),
                                                    file.name.clone(),
                                                ))
                                            }
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
                                                // let image = egui::Image::from_bytes(
                                                //     file.name.clone(),
                                                //     NO_TEXTURE,
                                                // );
                                                assets
                                                    .push(("NO_TEXTURE".into(), file.name.clone()))
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

                    let mut logged = LOGGED.lock().unwrap();
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
                                .show_vec(row_items, |ui, (image, asset_name), handle, state| {
                                    let card_size = egui::vec2(card_width, card_height);
                                    handle.ui(ui, |ui| {
                                        let (rect, card_response) =
                                            ui.allocate_exact_size(card_size, egui::Sense::click());

                                        let mut card_ui = ui.new_child(
                                            egui::UiBuilder::new().max_rect(rect).layout(
                                                egui::Layout::top_down(egui::Align::Center),
                                            ),
                                        );

                                        let image_response = card_ui.add_sized(
                                            [image_size, image_size],
                                            egui::ImageButton::new(image.clone()).frame(false),
                                        );

                                        let is_hovered = card_response.hovered()
                                            || image_response.hovered()
                                            || state.dragged;

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
                                });
                        });
                        ui.add_space(8.0);
                    }
                });
            }
            EditorTab::ResourceInspector => {
                ui.label("Resource Inspector");
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

            // We'll store the popup rect here
            let mut popup_rect = None;

            area.show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // Save the rect of the popup for later hit-testing
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
                            if ui.selectable_label(false, "Inspect Resource").clicked() {
                                menu_action = Some(EditorTabMenuAction::InspectResource);
                            }
                        }
                        EditorTab::Viewport => {
                            ui.set_min_width(150.0);
                            if ui.selectable_label(false, "Viewport Option").clicked() {
                                menu_action = Some(EditorTabMenuAction::ViewportOption);
                            }
                        }
                    }
                })
            });

            if let Some(action) = menu_action {
                if Some(tab.clone()) == cfg.context_menu_tab {
                    match action {
                        EditorTabMenuAction::ImportResource => {
                            log::debug!("Import Resource clicked");

                            match import_object() {
                                Ok(_) => {
                                    if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                        toasts.add(Toast {
                                            kind: ToastKind::Success,
                                            text: "Resource(s) imported successfully!".into(),
                                            options: ToastOptions::default()
                                                .duration_in_seconds(3.0),
                                            ..Default::default()
                                        });
                                    }
                                }
                                Err(e) => {
                                    if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                        toasts.add(Toast {
                                            kind: ToastKind::Error,
                                            text: format!("Failed to import resource(s): {e}")
                                                .into(),
                                            options: ToastOptions::default()
                                                .duration_in_seconds(5.0),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::RefreshAssets => {
                            log::debug!("Refresh assets clicked");
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::AddEntity => {
                            log::debug!("Add Entity clicked");
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::DeleteEntity => {
                            log::debug!("Delete Entity clicked");
                            cfg.show_context_menu = false;
                            cfg.context_menu_tab = None;
                            return;
                        }
                        EditorTabMenuAction::InspectResource => {
                            log::debug!("Inspect Resource clicked");
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
                    }
                }
            }

            if let Some(rect) = popup_rect {
                if cfg.show_context_menu && Some(tab.clone()) == cfg.context_menu_tab {
                    if ui
                        .ctx()
                        .input(|i| i.pointer.button_clicked(egui::PointerButton::Primary))
                    {
                        if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                            if !rect.contains(pos) {
                                cfg.show_context_menu = false;
                                cfg.context_menu_tab = None;
                            }
                        }
                    }
                }
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
                    if let Some(project) = crate::states::PROJECT.read().ok() {
                        let models_dir = PathBuf::from(project.project_path.clone())
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
                    if let Some(project) = crate::states::PROJECT.read().ok() {
                        let textures_dir = PathBuf::from(project.project_path.clone())
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
                if let Some(project) = crate::states::PROJECT.read().ok() {
                    // everything else copies over to resources root dir
                    let resources_dir =
                        PathBuf::from(project.project_path.clone()).join("resources");
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
        let mut proj = PROJECT.write().unwrap();
        proj.write_to_all()?;
        Ok(())
    } else {
        return Err(anyhow::anyhow!("File dialogue returned None"));
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EditorTabMenuAction {
    ImportResource,
    RefreshAssets,
    AddEntity,
    DeleteEntity,
    InspectResource,
    ViewportOption,
}
