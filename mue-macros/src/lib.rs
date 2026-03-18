use heck::ToUpperCamelCase;
use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Meta, Pat, Type};

#[proc_macro_attribute]
pub fn node(_attr: TokenStream, item: TokenStream) -> TokenStream {
    make_fn(item, true)
}

#[proc_macro_attribute]
pub fn component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    make_fn(item, false)
}

fn make_fn(input: TokenStream, node: bool) -> TokenStream {
    let macroquad = match crate_name("mue-macroquad").expect("mue-macroquad not found") {
        FoundCrate::Itself => quote! { crate },
        FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote! { ::#ident }
        }
    };
    let option = quote! { ::std::option::Option };
    let prop = quote! { ::mue_core::prop::Prop };
    let into = quote! { ::std::convert::Into };

    let mut input = parse_macro_input!(input as ItemFn);
    let (impl_generics, ty_generics, where_clause) = input.sig.generics.split_for_impl();
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
                let ty: Type = syn::parse_quote! { #prop<#inner_ty> };
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

    let fields =
        args.iter()
            .map(|arg| match arg {
                Arg::Style(ty) => {
                    quote! { style: #ty }
                }
                Arg::Prop { ident, ty, default } => {
                    if default.is_some() {
                        quote! { #ident: #option<#prop<#ty>> }
                    } else {
                        quote! { #ident: #prop<#ty> }
                    }
                }
            })
            .chain(node.then(
                || quote! { children: #option<::mue_core::Owned<#macroquad::node::Children>> },
            ));
    let setters = args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Prop { ident, ty, default } => {
                let mut value = quote! { #into::into(value) };
                if default.is_some() {
                    value = quote! { Some(#value) };
                }
                Some(quote! {
                    pub fn #ident(mut self, value: impl #into<#prop<#ty>>) -> Self {
                        self.#ident = #value;
                        self
                    }
                })
            }
            _ => None,
        })
        .chain(node.then(|| {
            quote! {
                pub fn children(mut self, children: impl #macroquad::node::IntoChildren) -> Self {
                    self.children = Some(#macroquad::node::IntoChildren::into_children(children));
                    self
                }
            }
        }));

    let new_args: Vec<_> = args
        .iter()
        .filter_map(|arg| match arg {
            Arg::Prop { ident, ty, default } => {
                if default.is_none() {
                    Some(quote! { #ident: impl #into<#prop<#ty>> })
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
            Arg::Style(_) => quote! { style: #macroquad::style::Style::default() },
            Arg::Prop { ident, default, .. } => {
                if default.is_none() {
                    quote! { #ident: #into::into(#ident) }
                } else {
                    quote! { #ident: None }
                }
            }
        })
        .chain(node.then(|| quote! { children: None }));

    let invoke_args = args.iter().map(|arg| match arg {
        Arg::Style(_) => quote! { self.style },
        Arg::Prop { ident, default, .. } => {
            let default = default
                .as_ref()
                .map(|d| quote! { .unwrap_or_else(|| #prop::Static(#d)) });
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

    let into_node = if node {
        quote! {
            let children = self
                .children
                .unwrap_or_else(|| #macroquad::node::IntoChildren::into_children(()));
            #macroquad::node::Node::build_with_children(children, move || {
                #ident(#( #invoke_args ),*)
            })
        }
    } else {
        quote! {
            let result = #ident(#( #invoke_args ),*);
            #macroquad::node::IntoNode::into_node(result)
        }
    };

    let mut style_derive = quote! {};
    if args.iter().any(|arg| matches!(arg, Arg::Style(_))) {
        style_derive = quote! {
            impl #impl_generics #macroquad::style::Styleable for #builder_name #ty_generics #where_clause {
                fn style_mut(&mut self) -> &mut #macroquad::style::Style {
                    &mut self.style
                }
            }
        };
    }

    quote! {
        pub struct #builder_name #ty_generics #where_clause {
            #( #fields ),*
        }

        impl #impl_generics #builder_name #ty_generics #where_clause {
            pub fn new(#( #new_args ),*) -> Self {
                Self {
                    #( #prop_struct_init ),*
                }
            }

            #( #setters )*
        }

        impl #impl_generics #macroquad::node::IntoNode for #builder_name #ty_generics #where_clause {
            fn into_node(self) -> #macroquad::node::Node {
                #input
                #into_node
            }
        }

        #style_derive

        #vis fn #ident #impl_generics(#( #new_args ),*) -> #builder_name #ty_generics #where_clause {
            #builder_name::new(#( #new_arg_names ),*)
        }
    }
    .into()
}
