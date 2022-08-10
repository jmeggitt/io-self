use crate::attr::{Opts};
use darling::{FromDeriveInput};
use quote::{quote};
use syn::{parse_macro_input, parse_quote, DeriveInput, GenericParam};

mod attr;
mod read;
mod write;
mod util;

#[proc_macro_derive(ReadSelf, attributes(io_self))]
pub fn derive_read(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");

    let name = input.ident;

    let trait_bound = opts.trait_usage(true);
    for param in &mut input.generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(#trait_bound));
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let built = read::read_self_body(&name, &input.data, opts);

    proc_macro::TokenStream::from(quote! {
        impl #impl_generics ::io_self::ReadSelf for #name #ty_generics #where_clause {
            #[inline(always)]
            fn read_from<B>(buffer: &mut B) -> ::std::io::Result<Self>
                where B: ::std::io::Read + ::io_self::PositionAware {
                Ok(#built)
            }
        }
    })
}

#[proc_macro_derive(WriteSelf, attributes(io_self))]
pub fn derive_write(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");

    let name = input.ident;

    let trait_bound = opts.trait_usage(false);
    for param in &mut input.generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(#trait_bound));
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let built = write::write_self_body(&name, &input.data, opts);

    proc_macro::TokenStream::from(quote! {
        impl #impl_generics ::io_self::WriteSelf for #name #ty_generics #where_clause {
            #[inline(always)]
            fn write_to<B>(&self, buffer: &mut B) -> ::std::io::Result<()>
                where B: ::std::io::Write + ::io_self::PositionAware {
                #built;
                Ok(())
            }
        }
    })
}


#[cfg(test)]
mod tests {
    #[test]
    pub fn read_self() {
        let test_cases = trybuild::TestCases::new();
        test_cases.pass("tests/01-derive-empty.rs");
        test_cases.pass("tests/02-simple.rs");
        test_cases.pass("tests/03-array.rs");
        test_cases.pass("tests/04-simple-endian.rs");
        test_cases.pass("tests/05-array-endian.rs");
        test_cases.pass("tests/06-tagged-enum.rs");
        test_cases.pass("tests/07-length-prefix.rs");
        test_cases.pass("tests/08-prefixed-vec.rs");
        test_cases.pass("tests/09-field-specific-parsers.rs");
    }
}
