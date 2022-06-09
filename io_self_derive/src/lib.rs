use std::str::FromStr;
use darling::{FromDeriveInput, FromVariant};
use proc_macro2::{self, Ident, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, GenericParam, parse_quote, Data, Fields, Type};

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(io_self), forward_attrs(allow, doc, cfg))]
struct Opts {
    endian: Option<String>,
    tagged: Option<String>,
    length_prefix: Option<String>,
}


#[derive(FromVariant, Default)]
#[darling(default, attributes(io_self), forward_attrs(allow, doc, cfg))]
struct VariantOpts {
    tag: Option<String>,
}

impl Opts {
    fn length_prefix_type(&self) -> Option<Type> {
        let prefix = TokenStream::from_str(self.length_prefix.as_ref()?).unwrap();
        Some(syn::parse2(prefix).expect("Expected type"))
    }

    fn endianness(&self) -> Option<Endian> {
        match self.endian.as_ref()?.to_ascii_lowercase().as_ref() {
            "little" | "le" | "l" => Some(Endian::Little),
            "big" | "be" | "b" => Some(Endian::Big),
            x => panic!("Unknown endian format: {:?}", x),
        }
    }
}

#[derive(Copy, Clone)]
enum Endian {
    Little,
    Big
}

impl ToTokens for Endian {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Endian::Little => tokens.extend(quote!(io_self::derive_util::byteorder::LittleEndian)),
            Endian::Big => tokens.extend(quote!(io_self::derive_util::byteorder::BigEndian)),
        }
    }
}

#[proc_macro_derive(ReadSelf, attributes(io_self))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");

    let name = input.ident;

    for param in &mut input.generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(::io_self::ReadSelf));
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
            quote_spanned!{
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
            quote_spanned! (name.span() => ( #(#fields,)*) )
        }
        Type::Ptr(_) => panic!("Unable to read into pointer!"),
        Type::Reference(_) => panic!("Unable to read into reference!"),
        Type::Slice(_) => panic!("Unable to read into dynamically sized type!"),
        Type::TraitObject(_) => panic!("Unable to read into trait object!"),
        Type::BareFn(_) => panic!("Unable to read into function type!"),
        Type::ImplTrait(_) => panic!("Unable to read into impl Trait type!"),
        Type::Never(_) => panic!("Unable to read into never type!"),
        x => {
            let approach = match endian {
                None => quote!(io_self::ReadSelf),
                Some(Endian::Little) => quote!(io_self::derive_util::ReadSelfEndian<io_self::derive_util::LittleEndian>),
                Some(Endian::Big) => quote!(io_self::derive_util::ReadSelfEndian<io_self::derive_util::BigEndian>),
            };

            quote_spanned! {x.span() => <#x as #approach>::read_from(buffer)? }
        },
    }.into()
}

fn derive_fields(data_fields: &Fields, endian: Option<Endian>) -> TokenStream {
    match data_fields {
        Fields::Named(fields) => {
            let assigned_fields = fields.named.iter().map(|f| {
                let name = &f.ident;
                let formula = read_for_type(&f.ty, endian);
                quote_spanned! (f.span() => #name: #formula)
            });

            quote_spanned! (data_fields.span() => { #(#assigned_fields,)* })
        }
        Fields::Unnamed(fields) => {
            let assigned_fields = fields.unnamed.iter().map(|f| read_for_type(&f.ty, endian));
            quote_spanned! (data_fields.span() => ( #(#assigned_fields,)*) )
        }
        Fields::Unit => quote_spanned! (data_fields.span() => ),
    }.into()
}

fn read_self_body(name: &Ident, data: &Data, opts: Opts) -> TokenStream {
    match data {
        Data::Struct(struct_data) => {
            let fields = derive_fields(&struct_data.fields, opts.endianness());
            quote_spanned!(name.span() => #name #fields)
        }
        Data::Union(_) => panic!("Unable to derive for union"),
        Data::Enum(enum_data) => {
            let tag = TokenStream::from_str(opts.tagged.as_ref().expect("Enums must have a tag type to distinguish variants")).unwrap();
            let tag_type: Type = syn::parse2(tag).expect("Expected type");
            let endian = opts.endianness();

            let tag = read_for_type(&tag_type, endian);

            let variants = enum_data.variants.iter().map(|variant| {
                let opts = VariantOpts::from_variant(variant).expect("Unexpect attribute fields");

                let tag = match opts.tag {
                    Some(v) => TokenStream::from_str(&v).unwrap(),
                    None => panic!("All enum variants must have a distinguishing tag")
                };

                let variant_name = &variant.ident;
                let fields = derive_fields(&variant.fields, endian);
                quote!(#tag => #name::#variant_name #fields)
            });

            let prefix = if let Some(prefix_type) = opts.length_prefix_type() {
                let read_prefix = read_for_type(&prefix_type, endian);

                quote!{
                    let buffer = &mut buffer.take(usize::from(#read_prefix));
                }
            } else {
                quote!()
            };

            quote_spanned!(name.span() =>
                #prefix
                match #tag {
                    #(#variants,)*
                    x => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Invalid tag value: {:?}", x))),
                }
            )
        },
    }.into()
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
    }
}