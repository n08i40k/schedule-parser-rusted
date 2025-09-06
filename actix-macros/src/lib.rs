extern crate proc_macro;

use proc_macro::TokenStream;

mod shared {
    use quote::{ToTokens, quote};
    use syn::{Attribute, DeriveInput};

    pub fn find_status_code(attrs: &[Attribute]) -> Option<proc_macro2::TokenStream> {
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

    pub fn get_arms(ast: &DeriveInput) -> Vec<proc_macro2::TokenStream> {
        let name = &ast.ident;

        let variants = if let syn::Data::Enum(data) = &ast.data {
            &data.variants
        } else {
            panic!("Only enums are supported");
        };

        let mut status_code_arms: Vec<proc_macro2::TokenStream> = variants
            .iter()
            .filter_map(|v| -> Option<proc_macro2::TokenStream> {
                let status_code = find_status_code(&v.attrs)?;
                let variant_name = &v.ident;

                Some(quote! { #name::#variant_name => #status_code, })
            })
            .collect();

        if status_code_arms.len() < variants.len() {
            let status_code = find_status_code(&ast.attrs)
                .unwrap_or_else(|| quote! { ::actix_web::http::StatusCode::INTERNAL_SERVER_ERROR });

            status_code_arms.push(quote! { _ => #status_code });
        }

        status_code_arms
    }
}

mod middleware_error {
    use proc_macro::TokenStream;
    use quote::quote;

    pub fn fmt(ast: &syn::DeriveInput) -> TokenStream {
        let name = &ast.ident;

        let status_code_arms = super::shared::get_arms(ast);

        TokenStream::from(quote! {
            impl ::actix_web::ResponseError for #name {
                fn status_code(&self) -> ::actix_web::http::StatusCode {
                    match self {
                        #(#status_code_arms)*
                    }
                }

                fn error_response(&self) -> ::actix_web::HttpResponse<BoxBody> {
                    ::actix_web::HttpResponse::build(self.status_code())
                        .json(crate::middlewares::error::MiddlewareError::new(self.clone()))
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
                    ::actix_web::HttpResponse::Ok()
                            .json(self)
                            .map_into_left_body()
                }
            }
        })
    }
}

mod ok_response {
    use proc_macro::TokenStream;
    use quote::quote;

    pub fn fmt(ast: &syn::DeriveInput) -> TokenStream {
        let name = &ast.ident;

        TokenStream::from(quote! {
            impl crate::routes::schema::PartialOkResponse for #name {}
        })
    }
}

mod err_response {
    use proc_macro::TokenStream;
    use quote::quote;

    pub fn fmt(ast: &syn::DeriveInput) -> TokenStream {
        let name = &ast.ident;

        let status_code_arms = super::shared::get_arms(ast);

        TokenStream::from(quote! {
            impl crate::routes::schema::PartialErrResponse for #name {
                fn status_code(&self) -> ::actix_web::http::StatusCode {
                    match self {
                        #(#status_code_arms)*
                    }
                }
            }
        })
    }
}

#[proc_macro_derive(MiddlewareError, attributes(status_code))]
pub fn moddleware_error_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    middleware_error::fmt(&ast)
}

#[proc_macro_derive(ResponderJson)]
pub fn responser_json_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    responder_json::fmt(&ast)
}

#[proc_macro_derive(OkResponse)]
pub fn ok_response_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    ok_response::fmt(&ast)
}

#[proc_macro_derive(ErrResponse, attributes(status_code))]
pub fn err_response_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    err_response::fmt(&ast)
}
