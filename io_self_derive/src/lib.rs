use crate::attr::{Endian, Opts, VariantOpts};
use darling::{FromDeriveInput, FromVariant};
use proc_macro2::{self, Ident, TokenStream};
use quote::{quote, quote_spanned};
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Index, Type};

mod attr;

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

    let built = read_self_body(&name, &input.data, opts);

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

fn read_for_type(name: &Type, endian: Option<Endian>) -> TokenStream {
    match name {
        Type::Array(arr) => {
            let arr_type = &*arr.elem;
            let arr_len = &arr.len;
            let read_element = read_for_type(arr_type, endian);
            quote_spanned! {
                name.span() =>
                unsafe {
                    use std::mem::MaybeUninit;
                    let mut array = MaybeUninit::<[MaybeUninit<#arr_type>; #arr_len]>::uninit().assume_init();

                    for item in array.iter_mut().take(#arr_len) {
                        item.write(#read_element);
                    }

                    (&array as *const _ as *const #arr).read()
                }
            }
        }
        Type::Tuple(tuple) => {
            let fields = tuple.elems.iter().map(|f| read_for_type(f, endian));
            quote_spanned!(name.span() => ( #(#fields,)*) )
        }
        x => {
            let approach = match endian {
                None => quote!(io_self::ReadSelf),
                Some(Endian::Little) => {
                    quote!(io_self::derive_util::ReadSelfEndian<io_self::derive_util::LittleEndian>)
                }
                Some(Endian::Big) => {
                    quote!(io_self::derive_util::ReadSelfEndian<io_self::derive_util::BigEndian>)
                }
            };

            quote_spanned! {x.span() => <#x as #approach>::read_from(buffer)? }
        }
    }
}

fn derive_read_fields(data_fields: &Fields, endian: Option<Endian>) -> TokenStream {
    match data_fields {
        Fields::Named(fields) => {
            let assigned_fields = fields.named.iter().map(|f| {
                let name = &f.ident;
                let formula = read_for_type(&f.ty, endian);
                quote_spanned!(f.span() => #name: #formula)
            });

            quote_spanned!(data_fields.span() => { #(#assigned_fields,)* })
        }
        Fields::Unnamed(fields) => {
            let assigned_fields = fields.unnamed.iter().map(|f| read_for_type(&f.ty, endian));
            quote_spanned!(data_fields.span() => ( #(#assigned_fields,)*) )
        }
        Fields::Unit => quote_spanned!(data_fields.span() => ),
    }
}

fn read_self_body(name: &Ident, data: &Data, opts: Opts) -> TokenStream {
    match data {
        Data::Struct(struct_data) => {
            let fields = derive_read_fields(&struct_data.fields, opts.endianness());
            quote_spanned!(name.span() => #name #fields)
        }
        Data::Union(_) => panic!("Unable to derive for union"),
        Data::Enum(enum_data) => {
            let tag_type = opts
                .tag_type()
                .expect("Enums must have a tag type to distinguish variants");
            let endian = opts.endianness();

            let tag = read_for_type(&tag_type, endian);

            let variants = enum_data.variants.iter().map(|variant| {
                let opts = VariantOpts::from_variant(variant).expect("Unexpect attribute fields");
                let tag = opts.tag();

                let variant_name = &variant.ident;
                let fields = derive_read_fields(&variant.fields, endian);
                quote!(#tag => #name::#variant_name #fields)
            });

            if let Some(prefix_type) = opts.length_prefix_type() {
                let read_prefix = read_for_type(&prefix_type, endian);
                let read_len = try_from(&parse_quote!(usize), &prefix_type, &read_prefix);

                quote_spanned! {name.span() => {
                    let mut element_buffer = vec![0u8; #read_len];
                    buffer.read_exact(&mut element_buffer)?;
                    let mut cursor = ::std::io::Cursor::new(element_buffer);
                    let buffer = &mut cursor;

                    match #tag {
                        #(#variants,)*
                        x => return Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, format!("Invalid tag value: {:?}", x))),
                    }
                }}
            } else {
                quote_spanned!(name.span() =>
                    match #tag {
                        #(#variants,)*
                        x => return Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, format!("Invalid tag value: {:?}", x))),
                    }
                )
            }
        }
    }
}

fn write_for_type(ty: &Type, name: &TokenStream, endian: Option<Endian>) -> TokenStream {
    match ty {
        Type::Array(arr) => {
            let arr_type = &*arr.elem;
            let item = quote!(item);
            let write_element = write_for_type(arr_type, &item, endian);

            quote_spanned!( name.span() => for #item in #name { #write_element } )
        }
        Type::Tuple(tuple) => {
            let fields = tuple.elems.iter().enumerate().map(|(idx, f)| {
                let index = Index::from(idx);
                let item_name = quote!(#name.#index);
                write_for_type(f, &item_name, endian)
            });
            quote_spanned!(name.span() => #(#fields)* )
        }
        x => {
            let approach = match endian {
                None => quote!(io_self::WriteSelf),
                Some(Endian::Little) => quote!(
                    io_self::derive_util::WriteSelfEndian<io_self::derive_util::LittleEndian>
                ),
                Some(Endian::Big) => {
                    quote!(io_self::derive_util::WriteSelfEndian<io_self::derive_util::BigEndian>)
                }
            };

            quote_spanned! {x.span() => <#x as #approach>::write_to(#name, buffer)?; }
        }
    }
}

const TUPLE_NAME_PLACEHOLDER: &[&str] = &[
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z",
];

fn derive_field_match(data_fields: &Fields) -> TokenStream {
    match data_fields {
        Fields::Named(fields) => {
            let field_names = fields.named.iter().map(|f| &f.ident);
            quote_spanned!(data_fields.span() => { #(#field_names,)* })
        }
        Fields::Unnamed(fields) => {
            let assigned_fields = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(idx, _)| TokenStream::from_str(TUPLE_NAME_PLACEHOLDER[idx]).unwrap());
            quote_spanned!(data_fields.span() => ( #(#assigned_fields),* ) )
        }
        Fields::Unit => quote_spanned!(data_fields.span() => ),
    }
}

fn derive_write_fields(
    data_fields: &Fields,
    name: &TokenStream,
    endian: Option<Endian>,
    use_placeholders: bool,
) -> TokenStream {
    match data_fields {
        Fields::Named(fields) => {
            let assigned_fields = fields.named.iter().map(|f| {
                let ident = &f.ident;
                if use_placeholders {
                    write_for_type(&f.ty, &quote!(#ident), endian)
                } else {
                    write_for_type(&f.ty, &quote!(#name.#ident), endian)
                }
            });

            quote_spanned!(data_fields.span() => #(#assigned_fields)*)
        }
        Fields::Unnamed(fields) => {
            let assigned_fields = fields.unnamed.iter().enumerate().map(|(idx, f)| {
                let path = if use_placeholders {
                    TokenStream::from_str(TUPLE_NAME_PLACEHOLDER[idx]).unwrap()
                } else {
                    let index = Index::from(idx);
                    quote!(#name.#index)
                };
                write_for_type(&f.ty, &path, endian)
            });
            quote_spanned!(data_fields.span() => #(#assigned_fields)* )
        }
        Fields::Unit => quote_spanned!(data_fields.span() => ),
    }
}

fn write_self_body(name: &Ident, data: &Data, opts: Opts) -> TokenStream {
    match data {
        Data::Struct(struct_data) => {
            let parent = quote!(&self);
            derive_write_fields(&struct_data.fields, &parent, opts.endianness(), false)
        }
        Data::Union(_) => panic!("Unable to derive for union"),
        Data::Enum(enum_data) => {
            let tag_type = opts
                .tag_type()
                .expect("Enums must have a tag type to distinguish variants");
            let endian = opts.endianness();

            let variants = enum_data.variants.iter().map(|variant| {
                let opts = VariantOpts::from_variant(variant).expect("Unexpect attribute fields");
                let tag = opts.tag();
                let write_tag = write_for_type(&tag_type, &quote!(&variant_tag), endian);

                let variant_name = &variant.ident;
                let variant_match = derive_field_match(&variant.fields);
                let fields = derive_write_fields(&variant.fields, &quote!(), endian, true);
                quote! {
                    #name::#variant_name #variant_match => {
                        let variant_tag: #tag_type = #tag;
                        #write_tag
                        #fields
                    }
                }
            });

            if let Some(prefix_type) = opts.length_prefix_type() {
                let body_len = try_from(
                    &prefix_type,
                    &parse_quote!(usize),
                    &quote!(obj_buffer.len()),
                );
                let write_prefix = write_for_type(&prefix_type, &quote!(&#body_len), endian);

                quote_spanned! { name.span() =>
                    let mut obj_buffer = Vec::new();
                    { // Use temporary scope to re-use buffer ident
                        let mut seekable_buffer = ::std::io::Cursor::new(&mut obj_buffer);
                        let buffer = &mut seekable_buffer;
                        match self { #(#variants,)* }
                    }

                    #write_prefix
                    buffer.write_all(&obj_buffer[..])?;
                }
            } else {
                quote_spanned!(name.span() => match self { #(#variants,)* })
            }
        }
    }
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

    let built = write_self_body(&name, &input.data, opts);

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

fn try_from(ty: &Type, from_ty: &Type, expr: &TokenStream) -> TokenStream {
    quote! {
        match <#ty as ::std::convert::TryFrom<#from_ty>>::try_from(#expr) {
            Ok(v) => v,
            Err(e) => return Err(::std::io::Error::new(::std::io::ErrorKind::Other, e)),
        }
    }
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
    }
}
