use rustyscript::Runtime;

use crate::scripting::ScriptableModule;
pub struct Camera;

impl ScriptableModule for Camera {
    fn register(_runtime: &mut Runtime) -> anyhow::Result<()> {
        Ok(())
    }
}