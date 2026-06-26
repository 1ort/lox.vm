extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(Trace)]
pub fn derive_trace(input: TokenStream) -> TokenStream {
    // 1. Parse source as AST
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // 2. Generate fn trace() body
    let trace_body = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields) => {
                let field_idents = fields.named.iter().map(|f| &f.ident);
                quote! {
                    #( self.#field_idents.trace(); )*
                }
            }
            Fields::Unnamed(fields) => {
                let field_indices = (0..fields.unnamed.len()).map(syn::Index::from);
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
                        let idents: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
                        quote! {
                            Self::#variant_name { #( #idents ),* } => {
                                #( #idents.trace(); )*
                            }
                        }
                    }
                    Fields::Unnamed(fields) => {
                        let idents: Vec<_> = (0..fields.unnamed.len())
                            .map(|i| quote::format_ident!("field_{}", i))
                            .collect();
                        quote! {
                            Self::#variant_name( #( #idents ),* ) => {
                                #( #idents.trace(); )*
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
