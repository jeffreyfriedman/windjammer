//! Procedural macros for windjammer-ui
//!
//! Provides the `#[component]` attribute macro for defining UI components.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// Attribute macro for defining a UI component
///
/// # Example
///
/// ```ignore
/// use windjammer_ui::prelude::*;
///
/// #[component]
/// struct Counter {
///     count: i32,
/// }
///
/// impl Counter {
///     fn render(&self) -> VNode {
///         // ... render implementation
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Extract fields for state management
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "Component must be a struct with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "Component must be a struct")
                .to_compile_error()
                .into();
        }
    };

    // Generate field accessors
    let field_names: Vec<_> = fields.iter().filter_map(|f| f.ident.as_ref()).collect();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    // Generate the component implementation
    let expanded = quote! {
        #[derive(Clone)]
        #input

        impl #impl_generics #name #ty_generics #where_clause {
            /// Create a new component instance
            pub fn new() -> Self {
                Self {
                    #(#field_names: Default::default()),*
                }
            }

            /// Create a component with custom initial values
            pub fn with_state(#(#field_names: #field_types),*) -> Self {
                Self {
                    #(#field_names),*
                }
            }
        }

        impl #impl_generics Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                Self::new()
            }
        }

        // Note: Component trait implementation must be done manually by the user
        // with the render() method in the impl Component for #name block

        // Implement Send + Sync for cross-platform use
        unsafe impl #impl_generics Send for #name #ty_generics #where_clause {}
        unsafe impl #impl_generics Sync for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

/// Derive macro for component props
///
/// # Example
///
/// ```ignore
/// #[derive(Props)]
/// struct ButtonProps {
///     text: String,
///     onClick: Box<dyn Fn()>,
/// }
/// ```
#[proc_macro_derive(Props)]
pub fn derive_props(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics windjammer_ui::component::ComponentProps for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    // Tests for proc macros are typically done with trybuild in tests/ directory
}
