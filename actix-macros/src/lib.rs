extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::Attribute;

fn find_status_code(attrs: &Vec<Attribute>) -> Option<proc_macro2::TokenStream> {
    attrs
        .iter()
        .find_map(|attr| -> Option<proc_macro2::TokenStream> {
            if !attr.path().is_ident("status_code") {
                return None;
            }

            let meta = attr.meta.require_name_value().ok()?;

            let code = meta.value.to_token_stream().to_string();
            let trimmed_code = code.trim_matches('"');

            if let Ok(numeric_code) = trimmed_code.parse::<u16>() {
                Some(quote! { actix_web::http::StatusCode::from_u16(#numeric_code).unwrap() })
            } else {
                let string_code: proc_macro2::TokenStream =
                    trimmed_code.to_string().parse().unwrap();
                
                Some(quote! { #string_code })
            }
        })
}

fn impl_rem(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let variants = if let syn::Data::Enum(data) = &ast.data {
        &data.variants
    } else {
        panic!("Only enums are supported");
    };

    let mut status_code_arms: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .map(|v| -> Option<proc_macro2::TokenStream> {
            let status_code = find_status_code(&v.attrs)?;
            let variant_name = &v.ident;

            Some(quote! { #name::#variant_name => #status_code, })
        })
        .filter(|v| v.is_some())
        .map(|v| v.unwrap())
        .collect();

    if status_code_arms.len() < variants.len() {
        let status_code = find_status_code(&ast.attrs)
            .unwrap_or_else(|| quote! { actix_web::http::StatusCode::INTERNAL_SERVER_ERROR });

        status_code_arms.push(quote! { _ => #status_code });
    }

    TokenStream::from(quote! {
        impl actix_web::ResponseError for #name {
            fn status_code(&self) -> actix_web::http::StatusCode {
                match self {
                    #(#status_code_arms)*
                }
            }

            fn error_response(&self) -> actix_web::HttpResponse<BoxBody> {
                actix_web::HttpResponse::build(self.status_code()).json(crate::utility::error::ResponseErrorMessage::new(self.clone()))
            }
        }
    })
}

#[proc_macro_derive(ResponseErrorMessage, attributes(status_code))]
pub fn rem_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_rem(&ast)
}
