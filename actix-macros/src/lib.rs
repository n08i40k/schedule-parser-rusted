extern crate proc_macro;

use proc_macro::TokenStream;

mod response_error_message {
    use proc_macro::TokenStream;
    use quote::{ToTokens, quote};
    use syn::Attribute;

    pub fn find_status_code(attrs: &Vec<Attribute>) -> Option<proc_macro2::TokenStream> {
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

    pub fn fmt(ast: &syn::DeriveInput) -> TokenStream {
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
}

mod responder_json {
    use proc_macro::TokenStream;
    use quote::quote;

    pub fn fmt(ast: &syn::DeriveInput) -> TokenStream {
        let name = &ast.ident;

        TokenStream::from(quote! {
            impl ::actix_web::Responder for #name {
                type Body = ::actix_web::body::EitherBody<::actix_web::body::BoxBody>;
            
                fn respond_to(self, _: &::actix_web::HttpRequest) -> ::actix_web::HttpResponse<Self::Body> {
                    match ::serde_json::to_string(&self) {
                        Ok(body) => ::actix_web::HttpResponse::Ok()
                            .json(body)
                            .map_into_left_body(),
            
                        Err(err) => ::actix_web::HttpResponse::from_error(
                            ::actix_web::error::JsonPayloadError::Serialize(err),
                        )
                        .map_into_right_body(),
                    }
                }
            }
        })
    }
}

mod into_iresponse {
    use proc_macro::TokenStream;
    use quote::quote;

    pub fn fmt(ast: &syn::DeriveInput) -> TokenStream {
        let name = &ast.ident;

        TokenStream::from(quote! {
            impl<E> ::core::convert::Into<crate::routes::schema::IResponse<#name, E>> for #name
            where
                E: ::serde::ser::Serialize
                    + ::utoipa::PartialSchema
                    + ::core::clone::Clone
                    + crate::routes::schema::HttpStatusCode,
            {
                fn into(self) -> crate::routes::schema::IResponse<#name, E> {
                    crate::routes::schema::IResponse(Ok(self))
                }
            }
        })
    }
}

#[proc_macro_derive(ResponseErrorMessage, attributes(status_code))]
pub fn rem_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    response_error_message::fmt(&ast)
}

#[proc_macro_derive(ResponderJson)]
pub fn responser_json_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    responder_json::fmt(&ast)
}

#[proc_macro_derive(IntoIResponse)]
pub fn into_iresponse_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    into_iresponse::fmt(&ast)
}
