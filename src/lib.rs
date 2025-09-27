use proc_macro::TokenStream;
use quote::quote;
use syn::{
    DeriveInput, FnArg, ItemFn, Pat, PatType, PathArguments, ReturnType, Type, parse_macro_input,
};

#[proc_macro_attribute]
pub fn ask_handler(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let sig = &input.sig;
    let block = &input.block;

    let fn_name = &sig.ident;
    let inputs = &sig.inputs;
    let output = &sig.output;

    let mut actor_ty = None;
    let mut msg_ty = None;

    for arg in inputs {
        match arg {
            FnArg::Receiver(receiver) => actor_ty = Some(receiver.ty.clone()),
            FnArg::Typed(PatType { pat, ty, .. }) => {
                if let Pat::Ident(pat_ident) = pat.as_ref() {
                    let ident = pat_ident.ident.to_string();
                    if ident.as_str() == "msg" {
                        msg_ty = Some(ty.clone())
                    }
                }
            }
        }
    }

    let actor_ty = actor_ty.expect("Missing self: &Actor argument");
    let msg_ty = msg_ty.expect("Missing msg argument");

    let clean_actor_ty = strip_reference(&actor_ty);
    let clean_msg_ty = strip_reference(&msg_ty);

    let (resp_ty, err_ty) = extract_result_types(output);

    let expanded = quote! {
        #[async_trait::async_trait]
        impl ascolt::handler::AskHandlerTrait<#clean_msg_ty, #resp_ty, #err_ty> for #clean_actor_ty {
            async fn #fn_name(
                self: #actor_ty,
                msg: #msg_ty,
            ) -> Result<#resp_ty, #err_ty> {
                #block
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn tell_handler(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let sig = &input.sig;
    let block = &input.block;

    let fn_name = &sig.ident;
    let inputs = &sig.inputs;
    let output = &sig.output;

    let mut actor_ty = None;
    let mut msg_ty = None;

    for arg in inputs {
        match arg {
            FnArg::Receiver(receiver) => actor_ty = Some(receiver.ty.clone()),
            FnArg::Typed(PatType { pat, ty, .. }) => {
                if let Pat::Ident(pat_ident) = pat.as_ref() {
                    let ident = pat_ident.ident.to_string();
                    if ident.as_str() == "msg" {
                        msg_ty = Some(ty.clone())
                    }
                }
            }
        }
    }

    let actor_ty = actor_ty.expect("Missing self: &Actor argument");
    let msg_ty = msg_ty.expect("Missing msg argument");

    let clean_actor_ty = strip_reference(&actor_ty);
    let clean_msg_ty = strip_reference(&msg_ty);

    let (_, err_ty) = extract_result_types(output);

    let expanded = quote! {
        #[async_trait::async_trait]
        impl ascolt::handler::TellHandlerTrait<#clean_msg_ty, #err_ty> for #clean_actor_ty {
            async fn #fn_name(
                self: #actor_ty,
                msg: #msg_ty,
            ) -> Result<(), #err_ty> {
                #block
            }
        }
    };

    TokenStream::from(expanded)
}

fn strip_reference(ty: &syn::Type) -> &syn::Type {
    match ty {
        syn::Type::Reference(r) => strip_reference(&r.elem),
        _ => ty,
    }
}

fn extract_result_types(
    output: &ReturnType,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    match output {
        ReturnType::Type(_, ty) => {
            let type_path = match ty.as_ref() {
                Type::Path(tp) => tp,
                _ => panic!("Expected a path type (e.g. Result<T, E>)"),
            };

            let seg = type_path
                .path
                .segments
                .first()
                .expect("Expected a Result return type");

            if seg.ident != "Result" {
                panic!("Return type must be Result<T, E>");
            }

            let args = match &seg.arguments {
                PathArguments::AngleBracketed(args) => args,
                _ => panic!("Expected Result<T, E> with angle-bracketed args"),
            };

            let mut args_iter = args.args.iter();
            let resp = args_iter
                .next()
                .expect("Missing success type in Result<T, E>");
            let err = args_iter
                .next()
                .expect("Missing error type in Result<T, E>");

            (quote!(#resp), quote!(#err))
        }
        _ => panic!("Expected function to have a return type"),
    }
}

#[proc_macro_derive(Actor, attributes(actor))]
pub fn derive_actor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let mut error_ty = None;
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("actor")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("error") {
                let value: syn::Type = meta.value()?.parse()?;
                error_ty = Some(value);
                Ok(())
            } else {
                Err(meta.error("unsupported attribute"))
            }
        })
        .unwrap();
    }

    let error_ty = error_ty.expect("missing #[actor(error = ...)]");

    let expanded = quote! {
        impl ascolt::ActorTrait<#error_ty> for #name {}
    };

    TokenStream::from(expanded)
}
