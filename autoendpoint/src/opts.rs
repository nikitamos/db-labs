use proc_macro2::{Ident, Span};
use std::borrow::Cow;
use syn::{
    Error, LitStr, Path, Result, Token, Type, TypePath,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};

#[derive(Debug, Clone)]
pub struct EndpointOptions<'a> {
    pub error_type: Option<Cow<'a, Type>>,
    pub ok_type: Option<Cow<'a, Type>>,
    pub method: Cow<'a, Path>,
    pub path: Option<Cow<'a, LitStr>>,
    pub router: Option<Path>, // ok_wrapper: Option<Type>
}

impl<'a> Default for EndpointOptions<'a> {
    fn default() -> Self {
        Self {
            error_type: Default::default(),
            ok_type: Default::default(),
            method: Cow::Owned(parse_quote!(axum::routing::get)),
            path: Default::default(),
            router: None,
        }
    }
}

#[derive(Debug)]
pub enum Value {
    Type(Type),
    Path(Path),
    String(LitStr),
}

impl Value {
    pub fn span(&self) -> Span {
        match self {
            Value::Type(ty) => ty.span(),
            Value::Path(path) => path.span(),
            Value::String(lit_str) => lit_str.span(),
        }
    }
    pub fn into_type(self) -> Result<Type> {
        if let Self::Type(v) = self {
            Ok(v)
        } else {
            Err(Error::new(self.span(), "expected type"))
        }
    }

    pub fn into_path(self) -> Result<Path> {
        if let Self::Path(p) | Self::Type(Type::Path(TypePath { path: p, .. })) = self {
            Ok(p)
        } else {
            Err(Error::new(self.span(), "expected path"))
        }
    }

    pub fn into_string(self) -> Result<LitStr> {
        if let Self::String(v) = self {
            Ok(v)
        } else {
            Err(Error::new(self.span(), "expected string"))
        }
    }
}

impl Parse for Value {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input
            .parse()
            .map(|x| Self::String(x))
            .or_else(|_| input.parse().map(Self::Type))
    }
}

#[derive(Debug)]
pub struct KVPair<V: Parse> {
    pub key: Ident,
    pub eq: syn::token::Eq,
    pub value: V,
}

// impl<V: Parse> KVPair<V> {
//     pub fn parse_named(name: &str) -> syn::Result<Self> {

//     }
// }

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
            Err(Error::new(reference_span, "error type is not specified"))
        } else if self.ok_type.is_none() {
            Err(Error::new(
                reference_span.into(),
                "ok type is not specified",
            ))
        } else if self.path.is_none() {
            Err(Error::new(reference_span, "endpoint path is not specified"))
        } else {
            Ok(())
        }
    }

    pub fn router(&self) -> Cow<'_, Path> {
        if let Some(ref p) = self.router {
            Cow::Borrowed(p)
        } else {
            Cow::Owned(parse_quote!(axum::Router))
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
            router: another.router.or(self.router),
        }
    }
}

impl<'a> Parse for EndpointOptions<'a> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut result = Self::default();
        let pairs = input.parse_terminated(KVPair::<Value>::parse, Token![,])?;
        for kv in pairs {
            let k_ident = kv.key.to_string();
            match k_ident.as_str() {
                // TODO: is it possible to remove this allocation?
                "error" => result.error_type = Some(Cow::Owned(kv.value.into_type()?)),
                "ok" => result.ok_type = Some(Cow::Owned(kv.value.into_type()?)),
                "method" => result.method = Cow::Owned(kv.value.into_path()?),
                "path" => result.path = Some(Cow::Owned(kv.value.into_string()?)),
                "router" => result.router = Some(kv.value.into_path()?),
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
