use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// A `derive` macro that converts a struct to a usable [SerializableComponent].
///
/// You have to implement `serde::Serialize`, `serde::Deserialize` and `Clone` for the
/// struct to be usable (it will throw errors anyway).
///
/// # Usage
/// ```
/// use dropbear_macro::SerializableComponent;
///
/// #[derive(Serialize, Deserialize, Clone, SerializableComponent)] // required to be implemented
/// struct MyComponent {
///     value1: String,
///     value2: i32,
/// }
/// ```
#[proc_macro_derive(SerializableComponent)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        #[typetag::serde]
        impl SerializableComponent for #name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn clone_boxed(&self) -> Box<dyn SerializableComponent> {
                Box::new(self.clone())
            }

            fn type_name(&self) -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}