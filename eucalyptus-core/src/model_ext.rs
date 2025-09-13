use std::{collections::HashMap, hash::{DefaultHasher, Hash, Hasher}, path::PathBuf, sync::Arc};

use dropbear_engine::{graphics::Graphics, model::Model};
use parking_lot::Mutex;
use rayon::prelude::*;


lazy_static::lazy_static! {
    pub static ref GLOBAL_MODEL_LOADER: MultiModelLoader = MultiModelLoader::new();
}

#[derive(Clone)]
pub enum ModelLoadingStatus {
    NotLoaded,
    Processing,
    Loaded,
    Failed(String),
}

pub trait ModelType: Send + Sync {
    fn load(&self, graphics: &Graphics) -> anyhow::Result<Model>;
    fn get_id(&self) -> String;
}

pub struct ModelHandle {
    pub status: ModelLoadingStatus,
    pub id: u64,
}

pub struct MultiModelLoader {
    models: Arc<Mutex<Vec<Box<dyn ModelType>>>>,
    handles: Arc<Mutex<HashMap<u64, ModelHandle>>>,
    loaded_models: Arc<Mutex<HashMap<u64, Model>>>,
}

impl MultiModelLoader {
    /// Creates a new instance of a multimodel loader
    pub fn new() -> Self {
        Self {
            models: Arc::new(Mutex::new(Vec::new())),
            handles: Arc::new(Mutex::new(HashMap::new())),
            loaded_models: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Pushes a model into the global model loader queue
    pub fn push(&self, model: Box<dyn ModelType>) -> ModelHandle {
        let mut hasher = DefaultHasher::new();
        model.get_id().hash(&mut hasher);
        let id = hasher.finish();

        let handle = ModelHandle {
            status: ModelLoadingStatus::NotLoaded,
            id,
        };

        {
            let mut models = self.models.lock();
            models.push(model);
        }

        {
            let mut handles = self.handles.lock();
            handles.insert(id, ModelHandle {
                status: ModelLoadingStatus::NotLoaded,
                id,
            });
        }

        handle
    }

    /// Processes all models in the model loader queue
    pub fn process(&self, graphics: &Graphics) {
        let models = {
            let mut models_guard = self.models.lock();
            std::mem::take(&mut *models_guard)
        };

        if models.is_empty() {
            return;
        }

        {
            let mut handles = self.handles.lock();
            for handle in handles.values_mut() {
                handle.status = ModelLoadingStatus::Processing;
            }
        }

        let results: Vec<(u64, anyhow::Result<Model>)> = models
            .into_par_iter()
            .map(|model| {
                let mut hasher = DefaultHasher::new();
                model.get_id().hash(&mut hasher);
                let id = hasher.finish();
                
                let result = model.load(graphics);
                (id, result)
            })
            .collect();

        {
            let mut handles = self.handles.lock();
            let mut loaded_models = self.loaded_models.lock();
            
            for (id, result) in results {
                if let Some(handle) = handles.get_mut(&id) {
                    match result {
                        Ok(model) => {
                            handle.status = ModelLoadingStatus::Loaded;
                            loaded_models.insert(id, model); // Store the loaded model
                        }
                        Err(error) => {
                            handle.status = ModelLoadingStatus::Failed(format!("{}", error));
                        }
                    }
                }
            }
        }
    }

    /// Exchanges a model handle to a model. This function deleted the handle from memory if loaded and returns the model,
    /// else returns an error based on its status (but still keep the model). 
    pub fn exchange(&self, handle: ModelHandle) -> anyhow::Result<Model> {
        match handle.status {
            ModelLoadingStatus::Loaded => {
                let mut loaded_models = self.loaded_models.lock();
                if let Some(model) = loaded_models.remove(&handle.id) {
                    {
                        let mut handles = self.handles.lock();
                        handles.remove(&handle.id);
                    }
                    Ok(model)
                } else {
                    anyhow::bail!("Model with handle ID {} was marked as loaded but not found in storage", handle.id)
                }
            }
            ModelLoadingStatus::Failed(error) => {
                anyhow::bail!("Model loading failed: {}", error)
            }
            ModelLoadingStatus::Processing => {
                anyhow::bail!("Model is still processing, cannot exchange yet")
            }
            ModelLoadingStatus::NotLoaded => {
                anyhow::bail!("Model has not been processed yet")
            }
        }
    }

    /// Alternative to [`crate::model_ext::MultiModelLoader::exchange`], allowing you to use a handle_id instead 
    /// of a [`crate::model_ext::ModelHandle`]
    pub fn exchange_by_id(&self, handle_id: u64) -> anyhow::Result<Model> {
        let status = {
            let handles = self.handles.lock();
            handles.get(&handle_id)
                .map(|h| h.status.clone())
                .ok_or_else(|| anyhow::anyhow!("Handle with ID {} not found", handle_id))?
        };

        match status {
            ModelLoadingStatus::Loaded => {
                let mut loaded_models = self.loaded_models.lock();
                if let Some(model) = loaded_models.remove(&handle_id) {
                    // Also remove the handle since it's been consumed
                    {
                        let mut handles = self.handles.lock();
                        handles.remove(&handle_id);
                    }
                    Ok(model)
                } else {
                    anyhow::bail!("Model with handle ID {} was marked as loaded but not found in storage", handle_id)
                }
            }
            ModelLoadingStatus::Failed(error) => {
                anyhow::bail!("Model loading failed: {}", error)
            }
            ModelLoadingStatus::Processing => {
                anyhow::bail!("Model is still processing, cannot exchange yet")
            }
            ModelLoadingStatus::NotLoaded => {
                anyhow::bail!("Model has not been processed yet")
            }
        }
    }

    /// Fetches the status of the handle
    pub fn get_status(&self, handle_id: u64) -> Option<ModelLoadingStatus> {
        let handles = self.handles.lock();
        handles.get(&handle_id).map(|v| v.status.clone())
    }

    /// Clears completed handles
    pub fn clear_completed(&self) {
        let mut handles = self.handles.lock();
        handles.retain(|_, handle| {
            matches!(handle.status, ModelLoadingStatus::NotLoaded | ModelLoadingStatus::Processing)
        });
    }
}

pub struct PendingModel {
    pub path: Option<PathBuf>,
    pub bytes: Option<Vec<u8>>,
    pub label: String,
    pub model_type: ModelLoadType,
}

pub enum ModelLoadType {
    File,
    Memory,
}

impl ModelType for PendingModel {
    fn load(&self, graphics: &Graphics) -> anyhow::Result<Model> {
        match self.model_type {
            ModelLoadType::File => {
                if let Some(path) = &self.path {
                    log::debug!("Loading model from file: {}", path.display());
                    let _model = Model::load(graphics, path, Some(&self.label))?;
                    Ok(_model)
                } else {
                    anyhow::bail!("Unable to find path in pending model: {}", self.label)
                }
            }
            ModelLoadType::Memory => {
                if let Some(bytes) = &self.bytes {
                    log::debug!("Loading model from memory: {} bytes", bytes.len());
                    let _model = dropbear_engine::model::Model::load_from_memory(
                        graphics, 
                        bytes.clone(), 
                        Some(&self.label)
                    )?;
                    Ok(_model)
                } else {
                    anyhow::bail!("Unable to get bytes in pending model: {}", self.label)
                }
            }
        }
    }

    fn get_id(&self) -> String {
        format!("{}_{}", self.label, match self.model_type {
            ModelLoadType::File => "file",
            ModelLoadType::Memory => "memory",
        })
    }
}

