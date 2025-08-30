use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use syn::{visit::Visit, DeriveInput, ItemImpl, File};

/// Configuration for the Gleam bindings generator
#[derive(Debug)]
pub struct GleamBindingsConfig {
    /// Output directory for generated bindings (default: "env!(CARGO_TARGET_DIR)")
    pub output_dir: PathBuf,
    /// Module name for the generated bindings (default: "bindings")
    pub module_name: String,
    /// Whether to include documentation comments (default: true)
    pub include_docs: bool,
    /// Custom type mappings (Rust type -> Gleam type)
    pub type_mappings: HashMap<String, String>,
}

impl Default for GleamBindingsConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(env!("OUT_DIR")).parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().join("generated").join("gleek"),
            module_name: "bindings".to_string(),
            include_docs: true,
            type_mappings: HashMap::new(),
        }
    }
}

/// Builder for GleamBindingsConfig
impl GleamBindingsConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn output_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.output_dir = dir.into();
        self
    }
    
    pub fn module_name<S: Into<String>>(mut self, name: S) -> Self {
        self.module_name = name.into();
        self
    }
    
    pub fn include_docs(mut self, include: bool) -> Self {
        self.include_docs = include;
        self
    }
    
    pub fn add_type_mapping<S1: Into<String>, S2: Into<String>>(mut self, rust_type: S1, gleam_type: S2) -> Self {
        self.type_mappings.insert(rust_type.into(), gleam_type.into());
        self
    }
}

/// Main function to generate Gleam bindings from Rust source files
pub fn generate_gleam_bindings(config: GleamBindingsConfig) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&config.output_dir)?;
    
    let src_dir = Path::new("src");
    let rust_files = find_rust_files(src_dir)?;
    
    let mut all_bindings = String::new();
    let mut has_bindings = false;
    
    for file_path in rust_files {
        println!("Checking file {}", file_path.display());
        let content = fs::read_to_string(&file_path)?;
        
        match syn::parse_file(&content) {
            Ok(ast) => {
                let bindings = extract_bindings_from_file(&ast, &config)?;
                if !bindings.is_empty() {
                    all_bindings.push_str(&bindings);
                    all_bindings.push('\n');
                    has_bindings = true;
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e);
                continue;
            }
        }
    }
    if has_bindings {
        let output_file = config.output_dir.join(format!("{}.gleam", config.module_name));
        
        let header = generate_gleam_header(&config);
        let full_content = format!("{}\n{}", header, all_bindings);
        
        fs::write(&output_file, full_content)?;
        println!("Generated Gleam bindings: {}", output_file.display());
    } else {
        println!("No gleek_export or gleek_impl attributes found in source files");
    }
    
    Ok(())
}

/// Find all .rs files recursively in a directory
fn find_rust_files(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut rust_files = Vec::new();
    
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                rust_files.extend(find_rust_files(&path)?);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                rust_files.push(path);
            }
        }
    }
    
    Ok(rust_files)
}

/// Extract gleek bindings from a parsed Rust file
fn extract_bindings_from_file(file: &File, config: &GleamBindingsConfig) -> Result<String, Box<dyn std::error::Error>> {
    let mut visitor = GleekVisitor::new(config);
    visitor.visit_file(file);
    
    let mut result = String::new();
    
    for binding in visitor.type_bindings {
        result.push_str(&binding);
        result.push_str("\n\n");
    }
    
    for binding in visitor.impl_bindings {
        result.push_str(&binding);
        result.push_str("\n\n");
    }
    
    Ok(result)
}

struct GleekVisitor<'a> {
    config: &'a GleamBindingsConfig,
    type_bindings: Vec<String>,
    impl_bindings: Vec<String>,
}

impl<'a> GleekVisitor<'a> {
    fn new(config: &'a GleamBindingsConfig) -> Self {
        Self {
            config,
            type_bindings: Vec::new(),
            impl_bindings: Vec::new(),
        }
    }
    
    fn has_gleek_export_attr(&self, attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            attr.path().is_ident("gleek_export")
        })
    }
    
    fn has_gleek_impl_attr(&self, attrs: &[syn::Attribute]) -> bool {
        attrs.iter().any(|attr| {
            attr.path().is_ident("gleek_impl")
        })
    }
}

impl<'a> Visit<'_> for GleekVisitor<'a> {
    fn visit_derive_input(&mut self, node: &DeriveInput) {
        if self.has_gleek_export_attr(&node.attrs) {
            let binding = generate_gleam_type(node, self.config);
            self.type_bindings.push(binding);
        }
        
        syn::visit::visit_derive_input(self, node);
    }
    
    fn visit_item_impl(&mut self, node: &ItemImpl) {
        if self.has_gleek_impl_attr(&node.attrs) {
            let binding = generate_gleam_impl(node, self.config);
            self.impl_bindings.push(binding);
        }
        
        syn::visit::visit_item_impl(self, node);
    }
}

fn generate_gleam_type(ast: &DeriveInput, config: &GleamBindingsConfig) -> String {
    let name = &ast.ident;
    let doc_comment = if config.include_docs {
        extract_doc_comments(&ast.attrs)
    } else {
        String::new()
    };
    
    let type_def = match &ast.data {
        syn::Data::Struct(data_struct) => {
            let mut gleam_fields = Vec::new();
            
            for field in &data_struct.fields {
                if let Some(field_name) = &field.ident {
                    let gleam_type = rust_type_to_gleam(&field.ty, config);
                    gleam_fields.push(format!("  {}: {}", field_name, gleam_type));
                }
            }
            
            format!(
                "// dropbear-engine type binding\npub type {} {{\n  {}({})\n}}",
                name,
                name,
                gleam_fields.join(", ")
            )
        }
        syn::Data::Enum(data_enum) => {
            let mut variants = Vec::new();
            
            for variant in &data_enum.variants {
                let variant_name = &variant.ident;
                let variant_doc = if config.include_docs {
                    extract_doc_comments(&variant.attrs)
                } else {
                    String::new()
                };
                
                let variant_def = match &variant.fields {
                    syn::Fields::Unit => {
                        format!("  {}", variant_name)
                    }
                    syn::Fields::Unnamed(fields) => {
                        let types: Vec<String> = fields.unnamed.iter()
                            .map(|f| rust_type_to_gleam(&f.ty, config))
                            .collect();
                        format!("  {}({})", variant_name, types.join(", "))
                    }
                    syn::Fields::Named(fields) => {
                        let field_types: Vec<String> = fields.named.iter()
                            .map(|f| {
                                let name = f.ident.as_ref().unwrap();
                                let gleam_type = rust_type_to_gleam(&f.ty, config);
                                format!("{}: {}", name, gleam_type)
                            })
                            .collect();
                        format!("  {}({})", variant_name, field_types.join(", "))
                    }
                };
                
                if !variant_doc.is_empty() {
                    variants.push(format!("  {}\n{}", variant_doc, variant_def));
                } else {
                    variants.push(variant_def);
                }
            }
            
            format!(
                "// dropbear-engine type binding\npub type {} {{\n{}\n}}",
                name,
                variants.join("\n")
            )
        }
        syn::Data::Union(_) => {
            format!("// Union types not supported in Gleam: {}", name)
        }
    };
    
    if !doc_comment.is_empty() {
        format!("{}\n{}", doc_comment, type_def)
    } else {
        type_def
    }
}

fn generate_gleam_impl(ast: &ItemImpl, config: &GleamBindingsConfig) -> String {
    let mut gleam_functions = Vec::new();
    
    let type_name = match &*ast.self_ty {
        syn::Type::Path(type_path) => {
            type_path.path.segments.last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }
        _ => "Unknown".to_string()
    };
    
    for item in &ast.items {
        if let syn::ImplItem::Fn(method) = item {
            let gleam_fn = generate_gleam_function(method, &type_name, config);
            gleam_functions.push(gleam_fn);
        }
    }
    
    gleam_functions.join("\n\n")
}

fn generate_gleam_function(func: &syn::ImplItemFn, type_name: &str, config: &GleamBindingsConfig) -> String {
    let fn_name = &func.sig.ident;
    let mut params = Vec::new();
    let mut is_method = false;
    let doc_comment = if config.include_docs {
        extract_doc_comments(&func.attrs)
    } else {
        String::new()
    };
    
    for input in &func.sig.inputs {
        match input {
            syn::FnArg::Receiver(receiver) => {
                is_method = true;
                if receiver.mutability.is_some() {
                    params.push(format!("self: {}", type_name));
                } else {
                    params.push(format!("self: {}", type_name));
                }
            }
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = &pat_ident.ident;
                    let param_type = rust_type_to_gleam(&pat_type.ty, config);
                    params.push(format!("{}: {}", param_name, param_type));
                }
            }
        }
    }
    
    let return_type = match &func.sig.output {
        syn::ReturnType::Default => "Nil".to_string(),
        syn::ReturnType::Type(_, ty) => rust_type_to_gleam(ty, config),
    };
    
    let external_name = if is_method {
        format!("{}_{}", type_name.to_lowercase(), fn_name)
    } else {
        fn_name.to_string()
    };
    
    let function_def = format!(
        "@external(javascript, \"dropbear-engine\", \"{}\")\npub fn {}({}) -> {}",
        external_name,
        fn_name,
        params.join(", "),
        return_type
    );
    
    if !doc_comment.is_empty() {
        format!("{}\n{}", doc_comment, function_def)
    } else {
        function_def
    }
}

fn rust_type_to_gleam(ty: &syn::Type, config: &GleamBindingsConfig) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            let path = &type_path.path;
            if let Some(segment) = path.segments.last() {
                let type_name = segment.ident.to_string();
                
                if let Some(gleam_type) = config.type_mappings.get(&type_name) {
                    return gleam_type.clone();
                }
                
                match type_name.as_str() {
                    // primitives
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "Int".to_string(),
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "Int".to_string(),
                    "f32" | "f64" => "Float".to_string(),
                    "bool" => "Bool".to_string(),
                    "String" => "String".to_string(),
                    "str" => "String".to_string(),
                    
                    // option
                    "Option" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                return format!("Option({})", rust_type_to_gleam(inner_ty, config));
                            }
                        }
                        "Option(a)".to_string()
                    }
                    
                    // result
                    "Result" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            let mut result_args = Vec::new();
                            for arg in &args.args {
                                if let syn::GenericArgument::Type(ty) = arg {
                                    result_args.push(rust_type_to_gleam(ty, config));
                                }
                            }
                            if result_args.len() >= 2 {
                                return format!("Result({}, {})", result_args[0], result_args[1]);
                            }
                        }
                        "Result(a, b)".to_string()
                    }
                    
                    // vector
                    "Vec" => {
                        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                                return format!("List({})", rust_type_to_gleam(inner_ty, config));
                            }
                        }
                        "List(a)".to_string()
                    }
                    
                    // // glam types
                    // "DVec3" => "Vec3".to_string(),
                    // "DQuat" => "Quat".to_string(),
                    // "DMat4" => "Mat4".to_string(),
                    
                    // any of the rest (assuming they have been defined)
                    _ => type_name,
                }
            } else {
                "Unknown".to_string()
            }
        }
        syn::Type::Reference(type_ref) => {
            rust_type_to_gleam(&type_ref.elem, config)
        }
        syn::Type::Tuple(type_tuple) => {
            let elem_types: Vec<String> = type_tuple.elems.iter()
                .map(|t| rust_type_to_gleam(t, config))
                .collect();
            format!("#({})", elem_types.join(", "))
        }
        _ => "Unknown".to_string(),
    }
}

fn extract_doc_comments(attrs: &[syn::Attribute]) -> String {
    let mut doc_lines = Vec::new();
    
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Ok(meta) = attr.meta.require_name_value() {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &meta.value {
                    let comment = lit_str.value();
                    let trimmed = comment.trim();
                    if !trimmed.is_empty() {
                        doc_lines.push(format!("/// {}", trimmed));
                    }
                }
            }
        }
    }
    
    if doc_lines.is_empty() {
        String::new()
    } else {
        doc_lines.join("\n")
    }
}

/// Generate the Gleam file header
fn generate_gleam_header(config: &GleamBindingsConfig) -> String {
    format!(
        r#"// Auto-generated Gleam bindings for the dropbear engine
// These are purely stub implementations and serve no purpose to help with LSP and type safety.
// When compiled as a javascript target will it serve any purpose.
// TOUCH ME AT YOUR OWN WILL

// Generated from Rust source files with gleek_export and gleek_impl attributes
// Output directory: {}
// Module: {}"#,
        config.output_dir.display(),
        config.module_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[gleek_proc_macro::gleek_export]
    /// A struct used to store a vector of 3 values. Can be useful for position. 
    pub struct Vector3 {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }

    #[gleek_proc_macro::gleek_impl]
    impl Vector3 {
        /// Creates a new instance of a Vector3
        pub fn new(x: f32, y: f32, z: f32) -> Self {
            Self {
                x, y, z
            }
        }
    }
    
    #[test]
    fn test_config_builder() {
        let config = GleamBindingsConfig::new()
            .output_dir("custom/output")
            .module_name("my_bindings")
            .include_docs(false)
            .add_type_mapping("MyType", "CustomGleamType");
        
        assert_eq!(config.output_dir, PathBuf::from("custom/output"));
        assert_eq!(config.module_name, "my_bindings");
        assert!(!config.include_docs);
        assert_eq!(config.type_mappings.get("MyType"), Some(&"CustomGleamType".to_string()));
    }
    
    #[test]
    fn test_find_rust_files() {
        let temp_dir = TempDir::new().unwrap();
        let src_dir = temp_dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();
        
        // Create some test files
        fs::write(src_dir.join("lib.rs"), "// test").unwrap();
        fs::write(src_dir.join("main.rs"), "// test").unwrap();
        fs::write(src_dir.join("not_rust.txt"), "// test").unwrap();
        
        let sub_dir = src_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("mod.rs"), "// test").unwrap();
        
        let rust_files = find_rust_files(&src_dir).unwrap();
        assert_eq!(rust_files.len(), 3);
    }
}