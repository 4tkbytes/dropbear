use std::path::PathBuf;

use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, DeriveInput, FnArg, ItemImpl, Pat, ReturnType, Type, parse_macro_input};

#[proc_macro_attribute]
pub fn gleek_export(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let gleam_code = generate_gleam_type(&ast);

    let doc_text = format!("\nGleamBindingStart\n{}\nGleamBindingEnd\n", gleam_code);

    let expanded = quote! {
        // #[doc = #doc_lit]
        #ast
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn gleek_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as ItemImpl);
    let gleam_code = generate_gleam_impl(&ast);

    let doc_text = format!("\nGleamBindingStart\n{}\nGleamBindingEnd\n", gleam_code);
    // let doc_lit = syn::Lit::Str::new(&doc_text, Span::call_site());

    let expanded = quote! {
        // #[doc = #doc_lit]
        #ast
    };

    TokenStream::from(expanded)
}

fn generate_gleam_type(ast: &DeriveInput) -> String {
    let name = &ast.ident;
    let doc_comment = extract_doc_comments(&ast.attrs);
    
    let type_def = match &ast.data {
        syn::Data::Struct(data_struct) => {
            let mut gleam_fields = Vec::new();
            
            for field in &data_struct.fields {
                if let Some(field_name) = &field.ident {
                    let gleam_type = rust_type_to_gleam(&field.ty);
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
                let variant_doc = extract_doc_comments(&variant.attrs);
                
                let variant_def = match &variant.fields {
                    syn::Fields::Unit => {
                        format!("  {}", variant_name)
                    }
                    syn::Fields::Unnamed(fields) => {
                        let types: Vec<String> = fields.unnamed.iter()
                            .map(|f| rust_type_to_gleam(&f.ty))
                            .collect();
                        format!("  {}({})", variant_name, types.join(", "))
                    }
                    syn::Fields::Named(fields) => {
                        let field_types: Vec<String> = fields.named.iter()
                            .map(|f| {
                                let name = f.ident.as_ref().unwrap();
                                let gleam_type = rust_type_to_gleam(&f.ty);
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
    
    let header = "// Auto-generated Gleam bindings for the dropbear engine\n// These are purely stub implementations and serve no purpose to help with LSP and type safety.\n
// When compiled as a javascript target will it serve any purpose.\n// TOUCH ME AT YOUR OWN WILL\n";
    
    if !doc_comment.is_empty() {
        format!("{}\n{}\n{}\n", header, doc_comment, type_def)
    } else {
        format!("{}\n{}\n", header, type_def)
    }
}

fn extract_doc_comments(attrs: &[Attribute]) -> String {
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

fn generate_gleam_impl(ast: &ItemImpl) -> String {
    let mut gleam_functions = Vec::new();
    
    let type_name = match &*ast.self_ty {
        Type::Path(type_path) => {
            type_path.path.segments.last()
                .map(|seg| seg.ident.to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        }
        _ => "Unknown".to_string()
    };
    
    for item in &ast.items {
        if let syn::ImplItem::Fn(method) = item {
            let gleam_fn = generate_gleam_function(method, &type_name);
            gleam_functions.push(gleam_fn);
        }
    }
    
    gleam_functions.join("\n\n")
}

fn generate_gleam_function(func: &syn::ImplItemFn, type_name: &str) -> String {
    let fn_name = &func.sig.ident;
    let mut params = Vec::new();
    let mut is_method = false;
    let doc_comment = extract_doc_comments(&func.attrs);
    
    for input in &func.sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                is_method = true;
                if receiver.mutability.is_some() {
                    // Mutable self
                    params.push(format!("self: {}", type_name));
                } else {
                    // Immutable self
                    params.push(format!("self: {}", type_name));
                }
            }
            FnArg::Typed(pat_type) => {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = &pat_ident.ident;
                    let param_type = rust_type_to_gleam(&pat_type.ty);
                    params.push(format!("{}: {}", param_name, param_type));
                }
            }
        }
    }
    
    let return_type = match &func.sig.output {
        ReturnType::Default => "Nil".to_string(),
        ReturnType::Type(_, ty) => rust_type_to_gleam(ty),
    };
    
    let external_name = if is_method {
        format!("{}_{}", type_name.to_lowercase(), fn_name)
    } else {
        fn_name.to_string()
    };
    
    let function_def = format!(
        "// External function stub - implementation provided by dropbear-engine\n@external(javascript, \"engine\", \"{}\")\npub fn {}({}) -> {}",
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

fn rust_type_to_gleam(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            if let Some(segment) = path.segments.last() {
                let type_name = segment.ident.to_string();
                
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
                                return format!("Option({})", rust_type_to_gleam(inner_ty));
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
                                    result_args.push(rust_type_to_gleam(ty));
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
                                return format!("List({})", rust_type_to_gleam(inner_ty));
                            }
                        }
                        "List(a)".to_string()
                    }
                    
                    // glam types
                    // a DVec3 uses f64, however we will make Vec use f32, then cast it back
                    "DVec3" => "Vec3".to_string(),
                    "DQuat" => "Quat".to_string(),
                    "DMat4" => "Mat4".to_string(),
                    
                    // any of the rest (assuming they have been defined)
                    _ => type_name,
                }
            } else {
                "Unknown".to_string()
            }
        }
        Type::Reference(type_ref) => {
            // for references, just use the inner type
            rust_type_to_gleam(&type_ref.elem)
        }
        Type::Tuple(type_tuple) => {
            let elem_types: Vec<String> = type_tuple.elems.iter()
                .map(rust_type_to_gleam)
                .collect();
            format!("#({})", elem_types.join(", "))
        }
        _ => "Unknown".to_string(),
    }
}