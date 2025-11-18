use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

/// A `derive` macro that converts a struct to a usable Component.
///
/// You have to implement `serde::Serialize`, `serde::Deserialize` and `Clone` for the
/// struct to be usable (it will throw errors anyway).
///
/// # Usage
/// ```
/// use dropbear_derive::Component;
///
/// #[derive(Serialize, Deserialize, Clone, Component)] // required to be implemented
/// struct MyComponent {
///     value1: String,
///     value2: i32,
/// }
/// ```
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        #[typetag::serde]
        impl Component for #name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn clone_component(&self) -> Box<dyn Component> {
                Box::new(self.clone())
            }

            fn type_name(&self) -> &'static str {
                #name_str
            }
        }
    };

    TokenStream::from(expanded)
}
