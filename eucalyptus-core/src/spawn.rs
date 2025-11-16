use crate::states::{SceneEntity};
use dropbear_engine::future::{FutureHandle, FutureQueue};
use dropbear_engine::graphics::SharedGraphicsContext;
use parking_lot::Mutex;
use std::sync::{Arc, LazyLock};

/// All spawns that are waiting to be spawned in.
pub static PENDING_SPAWNS: LazyLock<Mutex<Vec<PotentialSpawn>>> = LazyLock::new(|| Mutex::new(Vec::new()));

pub struct PotentialSpawn {
    pub entity: SceneEntity,
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
pub fn push_pending_spawn(spawn: SceneEntity) {
    log::debug!("Pushing spawn");
    PENDING_SPAWNS.lock().push(PotentialSpawn { entity: spawn, handle: None });
}
