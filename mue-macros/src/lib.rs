use std::iter;

use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, FnArg, ItemFn, Meta, Pat, Type};

#[proc_macro_attribute]
pub fn node(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let macroquad = match crate_name("mue-macroquad").expect("mue-macroquad not found") {
        FoundCrate::Itself => quote! { crate },
        FoundCrate::Name(name) => quote! { ::#name },
    };
    let option = quote! { ::std::option::Option };

    let mut input = parse_macro_input!(input as ItemFn);
    let vis = &input.vis;
    let ident = &input.sig.ident;

    enum Arg {
        Style(Type),
        Prop {
            ident: syn::Ident,
            ty: Type,
            default: Option<proc_macro2::TokenStream>,
        },
    }

    let args: Vec<_> = input
        .sig
        .inputs
        .iter_mut()
        .map(|f| match f {
            FnArg::Receiver(_) => panic!("Node functions cannot have `self` parameter"),
            FnArg::Typed(pat_type) => {
                let Pat::Ident(pat) = pat_type.pat.as_ref() else {
                    panic!("Node function parameters must be simple identifiers");
                };
                let ident = &pat.ident;
                if ident == "style" {
                    return Arg::Style((*pat_type.ty).clone());
                }
                let inner_ty = (*pat_type.ty).clone();
                let ty: Type = syn::parse_quote! { ::mue_core::Prop<#inner_ty> };
                *pat_type.ty = ty.clone();

                let mut default = None;
                pat_type.attrs.retain_mut(|attr| {
                    if let Meta::Path(path) = &attr.meta {
                        if path.is_ident("default") {
                            default = Some(quote! { Default::default() });
                            return false;
                        }
                    }
                    if let Meta::List(list) = &attr.meta {
                        if list.path.is_ident("default") {
                            default = Some(list.tokens.clone());
                            return false;
                        }
                    }
                    true
                });
                Arg::Prop {
                    ident: ident.clone(),
                    ty: inner_ty,
                    default,
                }
            }
        })
        .collect();

    let fields = args
        .iter()
        .map(|arg| match arg {
            Arg::Style(ty) => {
                quote! { style: #ty }
            }
            Arg::Prop { ident, ty, default } => {
                if default.is_some() {
                    quote! { #ident: #option<::mue_core::Prop<#ty>> }
                } else {
                    quote! { #ident: ::mue_core::Prop<#ty> }
                }
            }
        })
        .chain(iter::once(
            quote! { children: #option<::mue_core::Owned<#macroquad::node::Children>> },
        ));
    let setters = args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Prop { ident, ty, default } => {
                let mut value = quote! { ::mue_core::IntoProp::into_prop(value) };
                if default.is_some() {
                    value = quote! { Some(#value) };
                }
                Some(quote! {
                    pub fn #ident(mut self, value: impl ::mue_core::IntoProp<#ty>) -> Self {
                        self.#ident = #value;
                        self
                    }
                })
            }
            _ => None,
        })
        .chain(iter::once(quote! {
            pub fn children(mut self, children: impl #macroquad::node::IntoChildren) -> Self {
                self.children = Some(#macroquad::node::IntoChildren::into_children(children));
                self
            }
        }));

    let new_args: Vec<_> = args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Prop { ident, ty, default } => {
                if default.is_none() {
                    Some(quote! { #ident: impl ::mue_core::IntoProp<#ty> })
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();
    let new_arg_names = args.iter().filter_map(|arg| match arg {
        Arg::Prop { ident, default, .. } => {
            if default.is_none() {
                Some(quote! { #ident })
            } else {
                None
            }
        }
        _ => None,
    });
    let prop_struct_init = args
        .iter()
        .map(|arg| match arg {
            Arg::Style(_) => quote! { style: #macroquad::Style::default() },
            Arg::Prop { ident, default, .. } => {
                if default.is_none() {
                    quote! { #ident: ::mue_core::IntoProp::into_prop(#ident) }
                } else {
                    quote! { #ident: None }
                }
            }
        })
        .chain(iter::once(quote! { children: None }));

    let invoke_args = args.iter().map(|arg| match arg {
        Arg::Style(_) => quote! { self.style },
        Arg::Prop { ident, default, .. } => {
            let default = default
                .as_ref()
                .map(|d| quote! { .unwrap_or_else(|| ::mue_core::Prop::Static(#d)) });
            quote! { self.#ident #default }
        }
    });

    let builder_name = syn::Ident::new(
        &format!(
            "{}Builder",
            input.sig.ident.to_string().to_upper_camel_case()
        ),
        input.sig.ident.span(),
    );

    let mut style_derive = quote! {};
    if args.iter().any(|arg| matches!(arg, Arg::Style(_))) {
        style_derive = quote! {
            impl #macroquad::Styleable for #builder_name {
                fn style_mut(&mut self) -> &mut #macroquad::Style {
                    &mut self.style
                }
            }
        };
    }

    quote! {
        pub struct #builder_name {
            #( #fields ),*
        }

        impl #builder_name {
            pub fn new(#( #new_args ),*) -> Self {
                Self {
                    #( #prop_struct_init ),*
                }
            }

            #( #setters )*
        }

        impl #macroquad::IntoNode for #builder_name {
            fn into_node(self) -> #macroquad::Node {
                #input

                let children = self
                    .children
                    .unwrap_or_else(|| #macroquad::node::IntoChildren::into_children(()));
                #macroquad::Node::build_with_children(children, move || {
                    #ident(#( #invoke_args ),*)
                })
            }
        }

        #style_derive

        #vis fn #ident(#( #new_args ),*) -> #builder_name {
            #builder_name::new(#( #new_arg_names ),*)
        }
    }
    .into()
}

#[proc_macro_derive(Properties, attributes(init))]
pub fn derive_properties(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => panic!("Properties can only be derived for structs with named fields"),
        },
        _ => panic!("Properties can only be derived for structs"),
    };

    // For Default impl, we need to reconstruct the original struct fields
    // because the user's struct already has the original types
    let field_defaults: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident;
            let field_type = &f.ty;
            quote! {
                #field_name: <#field_type>::default()
            }
        })
        .collect();

    // Generate setter methods - fields stay as original type, setters accept Into<T>
    let setters: Vec<_> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident;
            let field_type = &f.ty;
            quote! {
                fn #field_name(mut self, value: impl Into<#field_type>) -> Self {
                    self.#field_name = value.into();
                    self
                }
            }
        })
        .collect();

    quote! {
        impl #impl_generics Default for #name #ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #( #field_defaults ),*
                }
            }
        }

        impl #impl_generics #name #ty_generics #where_clause {
            pub fn new() -> Self {
                Self::default()
            }

            #( #setters )*
        }
    }
    .into()
}
