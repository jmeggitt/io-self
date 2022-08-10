use darling::{FromDeriveInput, FromField, FromVariant};
use proc_macro2::{self, TokenStream};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::Type;

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(io_self), forward_attrs(allow, doc, cfg))]
pub struct Opts {
    endian: Option<String>,
    tag: Option<Type>,
    length_prefix: Option<Type>,
}

impl Opts {

    pub fn length_prefix_type(&self) -> Option<&Type> {
        self.length_prefix.as_ref()
    }

    pub fn endianness(&self) -> Option<Endian> {
        match self.endian.as_ref()?.to_ascii_lowercase().as_ref() {
            "little" | "le" | "l" => Some(Endian::Little),
            "big" | "be" | "b" => Some(Endian::Big),
            x => panic!("Unknown endian format: {:?}", x),
        }
    }

    pub fn trait_usage(&self, read: bool) -> TokenStream {
        match (read, self.endianness()) {
            (false, None) => quote!(::io_self::WriteSelf),
            (true, None) => quote!(::io_self::ReadSelf),
            (false, Some(Endian::Little)) => quote!(
                ::io_self::derive_util::WriteSelfEndian<::io_self::derive_util::LittleEndian>
            ),
            (true, Some(Endian::Little)) => {
                quote!(::io_self::derive_util::ReadSelfEndian<::io_self::derive_util::LittleEndian>)
            }
            (false, Some(Endian::Big)) => {
                quote!(::io_self::derive_util::WriteSelfEndian<::io_self::derive_util::BigEndian>)
            }
            (true, Some(Endian::Big)) => {
                quote!(::io_self::derive_util::ReadSelfEndian<::io_self::derive_util::BigEndian>)
            }
        }
    }

    pub fn tag_type(&self) -> Option<&Type> {
        self.tag.as_ref()
    }
}

#[derive(FromVariant, Default)]
#[darling(default, attributes(io_self), forward_attrs(allow, doc, cfg))]
pub struct VariantOpts {
    tag: String,
}

impl VariantOpts {
    pub fn tag(&self) -> TokenStream {
        TokenStream::from_str(&self.tag).unwrap()
    }
}

#[derive(Copy, Clone)]
pub enum Endian {
    Little,
    Big,
}

impl ToTokens for Endian {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Endian::Little => tokens.extend(quote!(io_self::derive_util::byteorder::LittleEndian)),
            Endian::Big => tokens.extend(quote!(io_self::derive_util::byteorder::BigEndian)),
        }
    }
}

#[derive(FromField, Default)]
#[darling(default, attributes(io_self), forward_attrs(allow, doc, cfg))]
pub struct FieldOpts {
    length_prefix: Option<String>,
    endian: Option<String>,
    read_fn: Option<String>,
    write_fn: Option<String>,
}

impl FieldOpts {
    pub fn read_fn(&self) -> Option<TokenStream> {
        let func = TokenStream::from_str(self.read_fn.as_ref()?)
            .expect("Unable to tokenize read_fn");

        Some(quote!{ (#func)(buffer)? })
    }


    pub fn write_fn(&self, name: &TokenStream) -> Option<TokenStream> {
        let func = TokenStream::from_str(self.write_fn.as_ref()?)
            .expect("Unable to tokenize read_fn");

        Some(quote!{ (#func)(#name, buffer)? })
    }

    pub fn with_endian(&mut self, opts: &Opts) {
        if self.endian.is_none() {
            self.endian = opts.endian.clone();
        }
    }

    pub fn length_prefix_type(&self) -> Option<Type> {
        let prefix = TokenStream::from_str(self.length_prefix.as_ref()?)
            .expect("Unable to tokenize field length prefix");
        Some(syn::parse2(prefix).expect("Expected type"))
    }

    pub fn endianness(&self) -> Option<Endian> {
        match self.endian.as_ref()?.to_ascii_lowercase().as_ref() {
            "little" | "le" | "l" => Some(Endian::Little),
            "big" | "be" | "b" => Some(Endian::Big),
            x => panic!("Unknown endian format: {:?}", x),
        }
    }


    pub fn trait_usage(&self, read: bool) -> TokenStream {
        match (read, self.endianness()) {
            (false, None) => quote!(::io_self::WriteSelf),
            (true, None) => quote!(::io_self::ReadSelf),
            (false, Some(Endian::Little)) => quote!(
                ::io_self::derive_util::WriteSelfEndian<::io_self::derive_util::LittleEndian>
            ),
            (true, Some(Endian::Little)) => {
                quote!(::io_self::derive_util::ReadSelfEndian<::io_self::derive_util::LittleEndian>)
            }
            (false, Some(Endian::Big)) => {
                quote!(::io_self::derive_util::WriteSelfEndian<::io_self::derive_util::BigEndian>)
            }
            (true, Some(Endian::Big)) => {
                quote!(::io_self::derive_util::ReadSelfEndian<::io_self::derive_util::BigEndian>)
            }
        }
    }
}
