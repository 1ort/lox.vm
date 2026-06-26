extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Trace, attributes(trace))]
pub fn derive_trace(input: TokenStream) -> TokenStream {
    // 1. Parse source as AST
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // #[trace(skip)]
    fn is_skipped(field: &syn::Field) -> bool {
        field.attrs.iter().any(|attr| {
            if let syn::Meta::List(meta_list) = &attr.meta {
                meta_list.path.is_ident("trace") && meta_list.tokens.to_string() == "skip"
            } else {
                false
            }
        })
    }

    // 2. Generate fn trace() body
    let trace_body = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields) => {
                let field_idents = fields
                    .named
                    .iter()
                    .filter(|f| !is_skipped(f))
                    .map(|f| &f.ident);
                quote! {
                    #( self.#field_idents.trace(); )*
                }
            }
            Fields::Unnamed(fields) => {
                let field_indices = fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| !is_skipped(f))
                    .map(|(i, _)| syn::Index::from(i));
                quote! {
                    #( self.#field_indices.trace(); )*
                }
            }
            Fields::Unit => quote! {},
        },
        Data::Enum(data_enum) => {
            let variants = data_enum.variants.iter().map(|variant| {
                let variant_name = &variant.ident;

                match &variant.fields {
                    Fields::Named(fields) => {
                        let pat_fields = fields.named.iter().map(|f| {
                            let ident = &f.ident;
                            if is_skipped(f) {
                                quote! { #ident: _ }
                            } else {
                                quote! { #ident }
                            }
                        });

                        let active_idents = fields
                            .named
                            .iter()
                            .filter(|f| !is_skipped(f))
                            .map(|f| &f.ident);

                        quote! {
                            Self::#variant_name { #( #pat_fields ),* } => {
                                #( #active_idents.trace(); )*
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let mut pat_idents = Vec::new();
                        let mut active_idents = Vec::new();

                        for (i, f) in fields.unnamed.iter().enumerate() {
                            let ident = quote::format_ident!("field_{}", i);
                            if is_skipped(f) {
                                pat_idents.push(quote! { _ });
                            } else {
                                pat_idents.push(quote! { #ident });
                                active_idents.push(ident);
                            }
                        }

                        quote! {
                            Self::#variant_name( #( #pat_idents ),* ) => {
                                #( #active_idents.trace(); )*
                            }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            Self::#variant_name => {}
                        }
                    }
                }
            });
            quote! {
                match self {
                    #( #variants )*
                }
            }
        }
        Data::Union(_) => panic!("derive(Trace) is not supported for unions"),
    };

    let expanded = quote! {
        impl #impl_generics Trace for #name #ty_generics #where_clause {
            fn trace(&self) {
                #trace_body
            }
        }
    };

    TokenStream::from(expanded)
}
