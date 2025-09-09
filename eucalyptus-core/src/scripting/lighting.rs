use rustyscript::Runtime;

use crate::scripting::ScriptableModule;

pub struct Lighting;

impl ScriptableModule for Lighting {
    fn register(_runtime: &mut Runtime) -> anyhow::Result<()> {
        Ok(())
    }
}