use crate::scene::SceneEntity;
use dropbear_engine::future::{FutureHandle, FutureQueue};
use dropbear_engine::graphics::SharedGraphicsContext;
use parking_lot::Mutex;
use std::sync::{Arc, LazyLock};

/// All spawns that are waiting to be spawned in.
pub static PENDING_SPAWNS: LazyLock<Mutex<Vec<PendingSpawn>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// A spawn that's waiting to be added into the world.
#[derive(Clone, Debug)]
pub struct PendingSpawn {
    pub scene_entity: SceneEntity,
    /// An optional future handle to an object.
    ///
    /// If one is specified, it is assumed that the returned object is a [`MeshRenderer`](dropbear_engine::entity::MeshRenderer).
    ///
    /// If one is NOT specified, it will be created based off the information provided. It is **recommended** to set it to [`None`].
    pub handle: Option<FutureHandle>,
}

/// Extension trait for checking the editor (or anything else) for any spawns
/// that are waiting to be polled and added
pub trait PendingSpawnController {
    /// Checks up on the spawn list and spawns them into the world after it has been
    /// asynchronously loaded.
    ///
    /// This is expected to be run on the update loop.
    fn check_up(
        &mut self,
        graphics: Arc<SharedGraphicsContext>,
        queue: Arc<FutureQueue>,
    ) -> anyhow::Result<()>;
}

/// Helper function to spawn a [`PendingSpawn`]
pub fn push_pending_spawn(spawn: PendingSpawn) {
    log::debug!("Pushing spawn");
    PENDING_SPAWNS.lock().push(spawn);
}
