extern crate synstructure;
#[macro_use]
extern crate quote;
extern crate proc_macro2;

fn is_skipped(binding: &synstructure::BindingInfo) -> bool {
    binding.ast().attrs.iter().any(|attr| {
        if let syn::Meta::List(meta_list) = &attr.meta {
            meta_list.path.is_ident("trace") && meta_list.tokens.to_string() == "skip"
        } else {
            false
        }
    })
}

fn derive_trace(mut s: synstructure::Structure) -> proc_macro2::TokenStream {
    s.filter(|binding| !is_skipped(binding));

    let body = s.each(|binding| {
        quote! {
            #binding.trace();
        }
    });
    s.gen_impl(quote! {
        extern crate gc;
        gen impl gc::Trace for @Self {
            fn trace(&self) {
                match *self { #body }
            }
        }
    })
}

synstructure::decl_derive!([Trace, attributes(trace) ] => derive_trace);
