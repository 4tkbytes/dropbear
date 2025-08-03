use super::*;
use std::{
    collections::HashSet, fs, sync::{LazyLock, Mutex}
};

use dropbear_engine::{
    entity::Transform, graphics::NO_TEXTURE
};
use egui;
use egui_extras;
use log;
use egui_dock_fork::TabViewer;
use egui_toast_fork::{Toast, ToastKind};
use nalgebra::{UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use transform_gizmo_egui::{mint::RowMatrix4, Gizmo, GizmoConfig, GizmoExt, GizmoMode};

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

                if self.tex_size.width != new_tex_height || self.tex_size.height != new_tex_height {
                    // log::debug!("Sending resize signal");
                    *self.resize_signal = (true, new_tex_width, new_tex_height);

                    self.tex_size.width = new_tex_width;
                    self.tex_size.height = new_tex_height;

                    let new_aspect = new_tex_width as f32 / new_tex_height as f32;
                    self.camera.aspect = new_aspect;
                }

                // TODO: Figure out how to get the guizmos working because this is fucking annoying to deal with
                // Note to self: fuck you >:(
                // Note to self: ok wow thats pretty rude im trying my best ＞﹏＜
                self.gizmo.update_config(GizmoConfig {
                    view_matrix: RowMatrix4::from(Into::<[[f64; 4]; 4]>::into(self.camera.view_mat.cast::<f64>())),
                    projection_matrix: RowMatrix4::from(Into::<[[f64; 4]; 4]>::into(self.camera.view_mat.cast::<f64>())),
                    viewport: ui.clip_rect(),
                    modes: GizmoMode::all(),
                    orientation: transform_gizmo_egui::GizmoOrientation::Global,
                    ..Default::default()
                });                

                let image_response = ui.add(
                    egui::Image::new((self.view, [self.tex_size.width as f32, self.tex_size.height as f32].into()))
                        .sense(egui::Sense::click_and_drag())
                );
                log::debug!("Viewport rect: {:?}", image_response.rect);
                log::debug!("Camera matrices - View: {:?}, Proj: {:?}", self.camera.view_mat, self.camera.proj_mat);

                if let Some(entity_id) = self.selected_entity {
        if let Ok(transform) = self.world.query_one_mut::<&mut Transform>(*entity_id) {
            let gizmo_transform = transform_gizmo_egui::math::Transform::from_scale_rotation_translation(
                transform.scale.cast::<f64>(), 
                transform.rotation.cast::<f64>(), 
                transform.position.cast::<f64>(),
            );

            if let Some((result, new_transforms)) = self.gizmo.interact(ui, &[gizmo_transform]) {
                if let Some(new_transform) = new_transforms.first() {
                    transform.position = Vector3::from([
                        new_transform.translation.x as f32,
                        new_transform.translation.y as f32,
                        new_transform.translation.z as f32,
                    ]);
                    transform.rotation = UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                        new_transform.rotation.s as f32,
                        new_transform.rotation.v.x as f32, 
                        new_transform.rotation.v.y as f32, 
                        new_transform.rotation.v.z as f32
                    ));
                    transform.scale = Vector3::from([
                        new_transform.scale.x as f32,
                        new_transform.scale.y as f32,
                        new_transform.scale.z as f32,
                    ]);
                    log::debug!("Gizmo updated entity {:?}: {:?}", entity_id, result);
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

                let mut assets: Vec<(egui::Image, String)> = Vec::new();
                {
                    let res = RESOURCES.read().unwrap();

                    fn recursive_search_nodes_and_attach_thumbnail(
                        res: &Vec<Node>,
                        assets: &mut Vec<(egui::Image, String)>,
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
                                                    ad_dir.join("cube_thumbnail.png");
                                                let file_name_osstr =
                                                    model_thumbnail.file_name().unwrap();
                                                let file_name =
                                                    file_name_osstr.to_str().unwrap().to_string();
                                                let image = egui::Image::from_bytes(
                                                    file_name.clone(),
                                                    fs::read(&model_thumbnail).unwrap(),
                                                );

                                                // let mut model = match model_to_image::ModelToImageBuilder::new(&file.path)
                                                //     .with_size((800, 800))
                                                //     .build() {
                                                //         Ok(v) => v,
                                                //         Err(e) => panic!("Error occurred while loading file from path: {}", e),
                                                //     };
                                                // let _ = model.render();
                                                // let buffer = model.output();
                                                // let bytes = buffer.bytes()
                                                //     .filter_map(Result::ok)
                                                //     .collect::<Vec<u8>>();
                                                // let file_name =
                                                //     file.name.clone();
                                                // let image = egui::Image::from_bytes(
                                                //     file_name,
                                                //     bytes,
                                                // );
                                                assets.push((image, file.name.clone()))
                                            }
                                            ResourceType::Texture => {
                                                let image = egui::Image::from_bytes(
                                                    file.name.clone(),
                                                    std::fs::read(&file.path)
                                                        .unwrap_or(NO_TEXTURE.to_vec()),
                                                );
                                                assets.push((image, file.name.clone()))
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
                                                let image = egui::Image::from_bytes(
                                                    file.name.clone(),
                                                    NO_TEXTURE,
                                                );
                                                assets.push((image, file.name.clone()))
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
                    let columns = 6;
                    let available_width = ui.available_width();
                    let min_spacing = 8.0;
                    let max_spacing = 30.0;
                    let label_height = 20.0;
                    let padding = 8.0; // check it out

                    let card_width = ((available_width - max_spacing * (columns as f32 - 1.0))
                        / columns as f32)
                        .max(32.0);
                    let image_size = card_width - label_height;
                    let spacing = ((available_width - columns as f32 * card_width)
                        / (columns as f32 - 1.0))
                        .clamp(min_spacing, max_spacing);
                    let card_height = image_size + label_height + padding;

                    egui::Grid::new("asset_grid")
                        .num_columns(columns)
                        .min_col_width(card_width)
                        .max_col_width(card_width)
                        .spacing([spacing, spacing])
                        .show(ui, |ui| {
                            for (i, (image, asset_name)) in assets.iter().enumerate() {
                                let card_size = egui::vec2(card_width, card_height);
                                let (rect, card_response) =
                                    ui.allocate_exact_size(card_size, egui::Sense::click());

                                let mut card_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(rect)
                                        .layout(egui::Layout::top_down(egui::Align::Center)),
                                );

                                let image_response = card_ui.add(
                                    egui::ImageButton::new(
                                        image.clone().max_size([image_size, image_size].into()),
                                    )
                                    .frame(false),
                                );

                                let is_hovered =
                                    card_response.hovered() || image_response.hovered();

                                if is_hovered {
                                    ui.painter().rect_filled(
                                        rect,
                                        6.0, // corner radius
                                        egui::Color32::from_rgb(60, 60, 80),
                                    );
                                }

                                card_ui.vertical_centered(|ui| {
                                    ui.label(
                                        egui::RichText::new(asset_name)
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    );
                                });

                                if (i + 1) % columns == 0 {
                                    ui.end_row();
                                }
                            }
                        });
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
