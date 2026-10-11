use std::borrow::Cow;
use proc_macro2::{Ident, TokenStream};
use syn::{Error, LitStr, Path, Token, Type, parse::Parse, parse_quote};

#[derive(Debug, Clone)]
pub struct EndpointOptions<'a> {
    pub error_type: Option<Cow<'a, Type>>,
    pub ok_type: Option<Cow<'a, Type>>,
    pub method: Cow<'a, Path>,
    pub path: Option<Cow<'a, LitStr>>,
    // ok_wrapper: Option<Type>
}

impl<'a> Default for EndpointOptions<'a> {
    fn default() -> Self {
        Self {
            error_type: Default::default(),
            ok_type: Default::default(),
            method: Cow::Owned(parse_quote!(axum::routing::get)),
            path: Default::default(),
        }
    }
}

pub struct KVPair<V: Parse> {
    pub key: Ident,
    pub eq: Token![=],
    pub value: V,
}

impl<V: Parse> Parse for KVPair<V> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            eq: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl<'a> EndpointOptions<'a> {
    pub fn require_valid(&self, reference_span: proc_macro2::Span) -> syn::Result<()> {
        if self.error_type.is_none() {
            Err(Error::new(reference_span, "Error type is not specified"))
        } else if self.ok_type.is_none() {
            Err(Error::new(
                reference_span.into(),
                "Ok type is not specified",
            ))
        } else if self.path.is_none() {
            Err(Error::new(reference_span.into(), "path is not specified"))
        } else {
            Ok(())
        }
    }

    /// For each option does the following:
    /// 1. If the option is present in `another`, returns the populates its value from `another`
    /// 2. Otherwise, the option is populated from `self`
    pub fn patch_with(self, another: EndpointOptions<'a>) -> EndpointOptions<'a> {
        Self {
            error_type: another.error_type.or(self.error_type),
            ok_type: another.ok_type.or(self.ok_type),
            method: another.method,
            path: another.path.or(self.path), // ok_wrapper: another.ok_wrapper.or(self::ok)
        }
    }
}

impl<'a> Parse for EndpointOptions<'a> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut result = Self::default();
        let pairs = input.parse_terminated(KVPair::<TokenStream>::parse, Token![,])?;
        for kv in pairs {
            let k_ident = kv.key.to_string();
            match k_ident.as_str() {
                // TODO: is it possible to remove this allocation?
                "error" => result.error_type = Some(Cow::Owned(syn::parse2(kv.value)?)),
                "ok" => result.ok_type = Some(Cow::Owned(syn::parse2(kv.value)?)),
                "method" => result.method = Cow::Owned(syn::parse2(kv.value)?),
                "path" => result.path = Some(Cow::Owned(syn::parse2(kv.value)?)),
                _ => {
                    return Err(Error::new(
                        kv.key.span(),
                        format!("Unexpected identifier: {k_ident}"),
                    ));
                }
            }
        }
        Ok(result)
    }
}
