use crate::attr::{FieldOpts, Opts, VariantOpts};
use darling::{FromField, FromVariant};
use proc_macro2::{self, Ident, TokenStream};
use quote::{quote, quote_spanned};
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{parse_quote, Data, Fields, Index, Type};

use crate::util;

const TUPLE_NAME_PLACEHOLDER: &[&str] = &[
    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
    "t", "u", "v", "w", "x", "y", "z",
];



pub fn build_write(name: &Ident, data: &Data, opts: Opts) -> TokenStream {
    match data {
        Data::Struct(struct_data) => {
            let parent = quote!(&self);
            derive_write_fields(&struct_data.fields, &parent, &opts, false)
        }
        Data::Union(_) => panic!("Unable to derive for union"),
        Data::Enum(enum_data) => {
            let tag_type = opts
                .tag_type()
                .expect("Enums must have a tag type to distinguish variants");

            let variants = enum_data.variants.iter().map(|variant| {
                let tag = VariantOpts::from_variant(variant)
                    .expect("Unexpect attribute fields")
                    .tag();
                let write_tag = write_for_type(&tag_type, &quote!(&variant_tag), &opts.trait_usage(false), None);

                let variant_name = &variant.ident;
                let variant_match = derive_field_match(&variant.fields);
                let fields = derive_write_fields(&variant.fields, &quote!(), &opts, true);
                quote! {
                    #name::#variant_name #variant_match => {
                        let variant_tag: #tag_type = #tag;
                        #write_tag
                        #fields
                    }
                }
            });

            if let Some(prefix_type) = opts.length_prefix_type() {
                let body_len = util::try_from(
                    &prefix_type,
                    &parse_quote!(usize),
                    &quote!(obj_buffer.len()),
                );
                let write_prefix = write_for_type(&prefix_type, &quote!(&#body_len), &opts.trait_usage(false), None);

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


fn write_for_type(ty: &Type, name: &TokenStream, approach: &TokenStream, prefix_length: Option<Type>) -> TokenStream {
    if let Some(prefix) = prefix_length {
        return quote_spanned!(ty.span() =>
            ::io_self::derive_util::write_with_prefix::<#prefix, #ty, _, _, _, _>(
                #name,
                buffer,
                <_ as #approach>::write_to,
                <_ as #approach>::write_to)?;
        );
    }

    match ty {
        Type::Array(arr) => {
            let arr_type = &*arr.elem;
            let item = quote!(item);
            let write_element = write_for_type(arr_type, &item, approach, None);

            quote_spanned!( name.span() => for #item in #name { #write_element } )
        }
        Type::Tuple(tuple) => {
            let fields = tuple.elems.iter().enumerate().map(|(idx, f)| {
                let index = Index::from(idx);
                let item_name = quote!(#name.#index);
                write_for_type(f, &item_name, approach, None)
            });
            quote_spanned!(name.span() => #(#fields)* )
        }
        x => quote_spanned! {x.span() => <#x as #approach>::write_to(#name, buffer)?; }
    }
}

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
    opts: &Opts,
    use_placeholders: bool,
) -> TokenStream {
    match data_fields {
        Fields::Named(fields) => {
            let assigned_fields = fields.named.iter().map(|f| {
                let ident = &f.ident;
                let ident = if use_placeholders {
                    quote!(#ident)
                } else {
                    quote!(#name.#ident)
                };

                let mut field_opts = FieldOpts::from_field(f).expect("Unexpect attribute fields");
                field_opts.with_endian(opts);

                match field_opts.write_fn(&ident) {
                    Some(v) => v,
                    None => write_for_type(&f.ty, &ident, &opts.trait_usage(false), field_opts.length_prefix_type()),
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
                let mut field_opts = FieldOpts::from_field(f).expect("Unexpect attribute fields");
                field_opts.with_endian(opts);

                match field_opts.write_fn(&path) {
                    Some(v) => v,
                    None => write_for_type(&f.ty, &path, &opts.trait_usage(false), field_opts.length_prefix_type()),
                }
            });
            quote_spanned!(data_fields.span() => #(#assigned_fields)* )
        }
        Fields::Unit => quote_spanned!(data_fields.span() => ),
    }
}