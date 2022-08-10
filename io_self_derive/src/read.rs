use crate::attr::{FieldOpts, Opts, VariantOpts};
use darling::{FromField, FromVariant};
use proc_macro2::{self, Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_quote, Data, Fields, Type};

use crate::util;

pub fn read_for_type(name: &Type, approach: &TokenStream, prefix_length: Option<Type>) -> TokenStream {
    if let Some(prefix) = prefix_length {
        let read_len = read_for_type(&prefix, approach, None);
        let item_count = util::try_from(&parse_quote!(usize), &prefix, &quote!(raw_len));

        return quote_spanned!(name.span() => {
            let raw_len = #read_len;
            let length = #item_count;

            ::io_self::derive_util::read_with_length(buffer, length, <_ as #approach>::read_from)?
        });
    }

    match name {
        Type::Array(arr) => {
            let arr_type = &*arr.elem;
            let arr_len = &arr.len;
            let read_element = read_for_type(arr_type, approach, None);
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
            let fields = tuple.elems.iter().map(|f| read_for_type(f, approach, None));
            quote_spanned!(name.span() => ( #(#fields,)*) )
        }
        x => quote_spanned! {x.span() => <#x as #approach>::read_from(buffer)? },
    }
}

pub fn derive_read_fields(data_fields: &Fields, opts: &Opts) -> TokenStream {
    match data_fields {
        Fields::Named(fields) => {
            let assigned_fields = fields.named.iter().map(|f| {
                let mut field_opts = FieldOpts::from_field(f).expect("Unexpect attribute fields");
                field_opts.with_endian(opts);
                let name = &f.ident;
                let formula = read_for_type(&f.ty, &field_opts.trait_usage(true), field_opts.length_prefix_type());
                quote_spanned!(f.span() => #name: #formula)
            });

            quote_spanned!(data_fields.span() => { #(#assigned_fields,)* })
        }
        Fields::Unnamed(fields) => {
            let assigned_fields = fields
                .unnamed
                .iter()
                .map(|f| {
                    let mut field_opts = FieldOpts::from_field(f).expect("Unexpect attribute fields");
                    field_opts.with_endian(opts);

                    read_for_type(&f.ty, &field_opts.trait_usage(true), field_opts.length_prefix_type())
                });
            quote_spanned!(data_fields.span() => ( #(#assigned_fields,)*) )
        }
        Fields::Unit => quote_spanned!(data_fields.span() => ),
    }
}

pub fn read_self_body(name: &Ident, data: &Data, opts: Opts) -> TokenStream {
    match data {
        Data::Struct(struct_data) => {
            let fields = derive_read_fields(&struct_data.fields, &opts);
            quote_spanned!(name.span() => #name #fields)
        }
        Data::Union(_) => panic!("Unable to derive for union"),
        Data::Enum(enum_data) => {
            let tag_type = opts
                .tag_type()
                .expect("Enums must have a tag type to distinguish variants");

            let tag = read_for_type(&tag_type, &opts.trait_usage(true), None);

            let variants = enum_data.variants.iter().map(|variant| {
                let tag = VariantOpts::from_variant(variant)
                    .expect("Unexpect attribute fields")
                    .tag();

                let variant_name = &variant.ident;
                let fields = derive_read_fields(&variant.fields, &opts);
                quote!(#tag => #name::#variant_name #fields)
            });

            if let Some(prefix_type) = opts.length_prefix_type() {
                let read_prefix = read_for_type(&prefix_type, &opts.trait_usage(true), None);
                let read_len = util::try_from(&parse_quote!(usize), &prefix_type, &read_prefix);

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
