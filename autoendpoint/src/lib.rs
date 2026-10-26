use std::borrow::Cow;

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    Attribute, Error, FnArg, ItemTrait, LitStr, Meta, Pat, ReceiverKind, TraitItemFn,
    Type, parse_macro_input, spanned::Spanned,
};

mod opts;

use opts::EndpointOptions;

struct ParsedEndpoint {
    pub handler: TokenStream,
    pub route: TokenStream,
}

#[proc_macro_attribute]
pub fn endpoints(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let options = parse_macro_input!(attr as EndpointOptions);
    let mut target_trait = parse_macro_input!(item as ItemTrait);

    let mut functions = Vec::new();
    let mut routes = Vec::new();
    for item in &mut target_trait.items {
        match item {
            syn::TraitItem::Fn(fn_item) => {
                let handler_fn = match process_fn(fn_item, options.clone()) {
                    Ok(Some(f)) => f,
                    Ok(None) => continue,
                    Err(e) => return e.to_compile_error().into(),
                };
                functions.push(handler_fn.handler);
                routes.push(handler_fn.route);
            }
            _ => {}
        }
    }

    let trait_ident = &target_trait.ident;
    let struct_name = Ident::new(&format!("{}Endpoints", trait_ident), Span::call_site());
    let router = options.router();

    let new_struct = quote! {
        #target_trait

        #[derive(Default)]
        pub struct #struct_name<Repo: #trait_ident>(core::marker::PhantomData<Repo>);

        impl <Repo: #trait_ident> #struct_name<Repo> {
            #(#functions)*

            pub fn routes<State>() -> #router<State>
            where
                State : std::clone::Clone + std::marker::Send
                      + std::marker::Sync + 'static,
                Repo : axum::extract::FromRef<State>
            {
                #router::new()
                    #(#routes)*
            }
        }
    };

    println!("{new_struct}");

    new_struct.into()
}

fn process_fn(
    fn_item: &mut TraitItemFn,
    default_options: EndpointOptions,
) -> syn::Result<Option<ParsedEndpoint>> {
    let fn_span = fn_item.sig.ident.span();
    let fn_name = &fn_item.sig.ident;

    let Some(attr) = extract_attribute(&mut fn_item.attrs, "endpoint") else {
        return Ok(None); // Endpoint should not be created
    };

    // Gather endpoint options
    let mut options: EndpointOptions = if matches!(attr.meta, Meta::Path(_)) {
        default_options
    } else {
        default_options.patch_with(attr.parse_args()?)
    };
    if options.path.is_none() {
        options.path = Some(Cow::Owned(LitStr::new(
            &format!("/{fn_name}"),
            Span::call_site(),
        )));
    }
    options.require_valid(attr.span())?;

    // Gather extractors
    let receiver = fn_item
        .sig
        .receiver()
        .ok_or_else(|| Error::new(fn_span, "endpoint must have a receiver"))?;
    match receiver.kind {
        ReceiverKind::Reference(..) => (),
        ReceiverKind::Value => (),
        _ => {
            return Err(Error::new(
                receiver.span(),
                "endpoint receiver must be either reference or value",
            ));
        }
    }

    let mut endpoint_args = Vec::with_capacity(fn_item.sig.inputs.len() - 1);
    let mut endpoint_arg_names = Vec::with_capacity(endpoint_args.len());
    // TODO: is it a valid way to skip receiver?
    for arg in fn_item.sig.inputs.iter_mut().skip(1) {
        let FnArg::Typed(arg) = arg else {
            println!("Receiver");
            continue;
        };
        let extractor: Type = extract_attribute(&mut arg.attrs, "extract")
            .ok_or_else(|| {
                Error::new(
                    arg.span(),
                    "#[extract(...)] attribute is required on function arguments",
                )
            })?
            .parse_args()?;
        endpoint_args.push(make_endpoint_arg(&arg.pat, &arg.ty, extractor));
        match arg.pat.as_ref() {
            Pat::Ident(pat_ident) => endpoint_arg_names.push(pat_ident.ident.clone()),
            _ => return Err(Error::new(arg.pat.span(), "unsupported pattern")),
        }
    }

    let EndpointOptions {
        error_type,
        ok_type,
        method,
        path,
        ..
    } = options;
    let t_await = if fn_item.sig.asyncness.is_some() {
        quote! {.await}
    } else {
        quote! {}
    };

    let body = quote! {
        pub async fn #fn_name
        (
            axum::extract::State(repo): axum::extract::State<Repo>,
            #(#endpoint_args),*
        ) -> Result<#ok_type, #error_type>
        {
            Ok(#ok_type ::from(
                repo.#fn_name(#(#endpoint_arg_names),*)#t_await?
            ))
        }
    };
    let route = quote! {
        .route(#path, #method(Self::#fn_name))
    };

    Ok(Some(ParsedEndpoint {
        handler: body,
        route,
    }))
}

/// If present, removes attribute with given name from the vector and returns it
fn extract_attribute(attributes: &mut Vec<Attribute>, attr_name: &str) -> Option<syn::Attribute> {
    attributes
        .iter()
        .enumerate()
        .find_map(|(i, attr)| {
            if attr.path().is_ident(attr_name) {
                Some(i)
            } else {
                None
            }
        })
        .map(|i| attributes.remove(i))
}

fn make_endpoint_arg(ident: &Pat, ty: &Type, extractor: Type) -> TokenStream {
    quote! {
        #extractor(#ident): #extractor<#ty>
    }
}
