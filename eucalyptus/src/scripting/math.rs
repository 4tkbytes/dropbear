use rustyscript::{Module, Runtime};

pub fn register_math_functions(runtime: &mut Runtime) -> anyhow::Result<()> {
    // runtime.load_module_async(module)
    let module = Module::load_dir("typescript")?;
    runtime.load_modules(module, side_modules)
    log::info!("[Script] Initialised math module");
}