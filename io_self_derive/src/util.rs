use proc_macro2::{self, TokenStream};
use quote::quote;
use syn::Type;


pub fn try_from(ty: &Type, from_ty: &Type, expr: &TokenStream) -> TokenStream {
    quote! {
        match <#ty as ::std::convert::TryFrom<#from_ty>>::try_from(#expr) {
            Ok(v) => v,
            Err(e) => return Err(::std::io::Error::new(::std::io::ErrorKind::Other, e)),
        }
    }
}

