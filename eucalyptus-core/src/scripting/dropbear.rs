
use wasmer::FunctionEnv;

use crate::{input::InputState, scripting::{DropbearScriptingAPIContext, ScriptableModule, ScriptableModuleWithEnv}};

pub struct DropbearAPI;

impl ScriptableModule for DropbearAPI {
    type Data = DropbearScriptingAPIContext;

    fn register(data: &Self::Data, imports: &mut wasmer::Imports, store: &mut wasmer::Store) -> anyhow::Result<()> {
        let env = FunctionEnv::new(store, data.clone());

        InputState::register(&env, imports, store)?;

        Ok(())
    }

    fn module_name() -> &'static str {
        "dropbear"
    }
}