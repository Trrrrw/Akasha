use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, FnArg, GenericArgument, ItemFn, Lit, LitStr, Meta,
    PatType, PathArguments, ReturnType, Token, Type, TypePath, parse_macro_input,
    punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn mcp(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    expand_mcp(func).into()
}

#[proc_macro_derive(MCPInputSchema)]
pub fn derive_mcp_input_schema(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    expand_mcp_input_schema(input).into()
}

fn expand_mcp(func: ItemFn) -> proc_macro2::TokenStream {
    let fn_name = &func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let openapi = extract_utoipa_path_info(&func.attrs);
    let docs = extract_doc_comment(&func.attrs);
    let description = tool_description(&fn_name_str, openapi.as_ref(), docs.as_ref());
    let schema_description = description.clone();
    let openapi_method = openapi
        .as_ref()
        .and_then(|info| info.method.clone())
        .unwrap_or_default();
    let openapi_path = openapi
        .as_ref()
        .and_then(|info| info.path.clone())
        .unwrap_or_default();
    let schema_fn = format_ident!("__{}_mcp_schema", fn_name);
    let call_fn = format_ident!("__{}_mcp_call", fn_name);

    let mut decode_stmts = Vec::new();
    let mut call_args = Vec::new();
    let mut schema_props = Vec::new();
    let mut schema_required = Vec::new();

    for arg in &func.sig.inputs {
        let FnArg::Typed(PatType { ty, .. }) = arg else {
            return compile_error("`#[mcp]` does not support methods with receiver parameters");
        };

        let Some((wrapper_name, inner_ty)) = extract_wrapper_inner_type(ty) else {
            return compile_error(
                "`#[mcp]` only supports Path<T>, Query<T>, and Json<T> parameters",
            );
        };

        let (arg_key, wrapper_ctor) = match wrapper_name.as_str() {
            "Path" => ("path", quote!(::axum_mcp::axum::extract::Path)),
            "Query" => ("query", quote!(::axum_mcp::axum::extract::Query)),
            "Json" => ("body", quote!(::axum_mcp::axum::Json)),
            _ => {
                return compile_error(
                    "`#[mcp]` only supports Path<T>, Query<T>, and Json<T> parameters",
                );
            }
        };

        let key_lit = LitStr::new(arg_key, Span::call_site());
        let decode_ident = format_ident!("__mcp_{}", arg_key);

        decode_stmts.push(quote! {
            let #decode_ident: #inner_ty = match __mcp_arguments.get(#key_lit) {
                Some(value) => match ::axum_mcp::serde_json::from_value::<#inner_ty>(value.clone()) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        return ::axum_mcp::serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": format!("invalid {} arguments: {}", #key_lit, err)
                            }],
                            "isError": true
                        });
                    }
                },
                None => {
                    return ::axum_mcp::serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": format!("missing {} arguments", #key_lit)
                        }],
                        "isError": true
                    });
                }
            };
        });

        call_args.push(quote!(#wrapper_ctor(#decode_ident)));

        schema_props.push(quote! {
            properties.insert(
                #key_lit.to_string(),
                <#inner_ty as ::axum_mcp::MCPInputSchema>::schema(),
            );
        });
        schema_required.push(quote! {
            required.push(#key_lit.to_string());
        });
    }

    let output_handler = match build_output_handler(&func.sig.output) {
        Ok(tokens) => tokens,
        Err(err) => return compile_error(err),
    };

    quote! {
        #func

        fn #schema_fn() -> ::axum_mcp::serde_json::Value {
            let mut properties = ::axum_mcp::serde_json::Map::new();
            let mut required = Vec::<String>::new();

            #(#schema_props)*
            #(#schema_required)*

            ::axum_mcp::serde_json::json!({
                "type": "object",
                "description": #schema_description,
                "properties": properties,
                "required": required,
                "_meta": {
                    "openapiMethod": #openapi_method,
                    "openapiPath": #openapi_path
                }
            })
        }

        fn #call_fn(__mcp_arguments: ::axum_mcp::serde_json::Value) -> ::axum_mcp::MCPCallFuture {
            Box::pin(async move {
                #(#decode_stmts)*

                let __mcp_result = #fn_name(#(#call_args),*).await;
                #output_handler
            })
        }

        ::axum_mcp::inventory::submit! {
            ::axum_mcp::MCPTool {
                name: #fn_name_str,
                description: #description,
                input_schema: #schema_fn,
                call: #call_fn,
            }
        }
    }
}

fn extract_wrapper_inner_type(ty: &Type) -> Option<(String, Type)> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let segment = path.segments.last()?;
    let inner_ty = match &segment.arguments {
        PathArguments::AngleBracketed(args) => match args.args.first()? {
            GenericArgument::Type(inner_ty) => inner_ty.clone(),
            _ => return None,
        },
        _ => return None,
    };

    Some((segment.ident.to_string(), inner_ty))
}

fn expand_mcp_input_schema(input: DeriveInput) -> proc_macro2::TokenStream {
    let ident = input.ident;
    let Data::Struct(data) = input.data else {
        return compile_error("`MCPInputSchema` can only be derived for structs");
    };
    let Fields::Named(fields) = data.fields else {
        return compile_error("`MCPInputSchema` can only be derived for structs with named fields");
    };

    let mut property_inserts = Vec::new();
    let mut required_pushes = Vec::new();

    for field in fields.named {
        let Some(field_ident) = field.ident else {
            continue;
        };
        let field_name = field_ident.to_string();
        let field_lit = LitStr::new(&field_name, Span::call_site());
        let description = extract_field_description(&field.attrs);
        let description_tokens = match description {
            Some(description) => {
                quote!(schema["description"] = ::axum_mcp::serde_json::json!(#description);)
            }
            None => quote!(),
        };
        let (schema_type, required) = schema_type_and_required(&field.ty);
        let schema_type_lit = LitStr::new(schema_type, Span::call_site());

        property_inserts.push(quote! {
            let mut schema = ::axum_mcp::serde_json::json!({
                "type": #schema_type_lit
            });
            #description_tokens
            properties.insert(#field_lit.to_string(), schema);
        });

        if required {
            required_pushes.push(quote! {
                required.push(#field_lit.to_string());
            });
        }
    }

    quote! {
        impl ::axum_mcp::MCPInputSchema for #ident {
            fn schema() -> ::axum_mcp::serde_json::Value {
                let mut properties = ::axum_mcp::serde_json::Map::new();
                let mut required = Vec::<String>::new();

                #(#property_inserts)*
                #(#required_pushes)*

                ::axum_mcp::serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required
                })
            }
        }
    }
}

fn extract_field_description(attrs: &[Attribute]) -> Option<String> {
    extract_doc_comment(attrs)
        .and_then(|doc| join_summary_description(&doc.summary, &doc.description))
}

fn schema_type_and_required(ty: &Type) -> (&'static str, bool) {
    if let Some(inner_ty) = option_inner_type(ty) {
        return (schema_type(inner_ty), false);
    }

    (schema_type(ty), true)
}

fn option_inner_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first()? {
        GenericArgument::Type(inner_ty) => Some(inner_ty),
        _ => None,
    }
}

fn schema_type(ty: &Type) -> &'static str {
    let Type::Path(type_path) = ty else {
        return "object";
    };
    let Some(segment) = type_path.path.segments.last() else {
        return "object";
    };

    match segment.ident.to_string().as_str() {
        "String" | "str" => "string",
        "bool" => "boolean",
        "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64" | "isize" => {
            "integer"
        }
        "f32" | "f64" => "number",
        _ => "object",
    }
}

#[derive(Default)]
struct UtoipaPathInfo {
    method: Option<String>,
    path: Option<String>,
    summary: Option<String>,
    description: Option<String>,
}

struct DocComment {
    summary: Option<String>,
    description: Option<String>,
}

fn extract_utoipa_path_info(attrs: &[Attribute]) -> Option<UtoipaPathInfo> {
    let attr = attrs.iter().find(|attr| is_utoipa_path_attr(attr))?;
    let mut info = UtoipaPathInfo::default();

    let Ok(items) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) else {
        return Some(info);
    };

    for item in items {
        match item {
            Meta::Path(path) => {
                if let Some(method) =
                    path.get_ident()
                        .map(|ident| ident.to_string())
                        .filter(|name| {
                            matches!(name.as_str(), "get" | "post" | "put" | "delete" | "patch")
                        })
                {
                    info.method = Some(method.to_uppercase());
                }
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("path") => {
                info.path = lit_str_expr_value(&name_value.value);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("summary") => {
                info.summary = lit_str_expr_value(&name_value.value);
            }
            Meta::NameValue(name_value) if name_value.path.is_ident("description") => {
                info.description = lit_str_expr_value(&name_value.value);
            }
            _ => {}
        }
    }

    Some(info)
}

fn lit_str_expr_value(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            Lit::Str(lit) => Some(lit.value()),
            _ => None,
        },
        _ => None,
    }
}

fn is_utoipa_path_attr(attr: &Attribute) -> bool {
    let mut segments = attr
        .path()
        .segments
        .iter()
        .map(|segment| segment.ident.to_string());

    matches!(
        (segments.next().as_deref(), segments.next().as_deref()),
        (Some("utoipa"), Some("path"))
    )
}

fn extract_doc_comment(attrs: &[Attribute]) -> Option<DocComment> {
    let mut lines = Vec::new();

    for attr in attrs {
        if let Meta::NameValue(meta) = &attr.meta {
            if !meta.path.is_ident("doc") {
                continue;
            }

            if let syn::Expr::Lit(expr_lit) = &meta.value
                && let syn::Lit::Str(lit) = &expr_lit.lit
            {
                let line = lit.value().trim().to_string();
                if !line.is_empty() {
                    lines.push(line);
                }
            }
        }
    }

    if lines.is_empty() {
        return None;
    }

    let summary = lines.first().cloned();
    let description_lines = lines.iter().skip(1).cloned().collect::<Vec<_>>().join("\n");
    let description = (!description_lines.is_empty()).then_some(description_lines);

    Some(DocComment {
        summary,
        description,
    })
}

fn tool_description(
    fn_name: &str,
    openapi: Option<&UtoipaPathInfo>,
    docs: Option<&DocComment>,
) -> String {
    if let Some(info) = openapi
        && let Some(description) = join_summary_description(&info.summary, &info.description)
    {
        return description;
    }

    if let Some(doc) = docs
        && let Some(description) = join_summary_description(&doc.summary, &doc.description)
    {
        return description;
    }

    if let Some(info) = openapi
        && let (Some(method), Some(path)) = (&info.method, &info.path)
    {
        return format!("{method} {path}");
    }

    fn_name.to_string()
}

fn join_summary_description(
    summary: &Option<String>,
    description: &Option<String>,
) -> Option<String> {
    match (summary, description) {
        (Some(summary), Some(description)) => Some(format!("{summary}\n\n{description}")),
        (Some(summary), None) => Some(summary.clone()),
        (None, Some(description)) => Some(description.clone()),
        (None, None) => None,
    }
}

fn build_output_handler(output: &ReturnType) -> Result<proc_macro2::TokenStream, &'static str> {
    match output {
        ReturnType::Default => Err("`#[mcp]` requires a return type"),
        ReturnType::Type(_, ty) => {
            if is_result_json_response(ty) {
                Ok(quote! {
                    match __mcp_result {
                        Ok(::axum_mcp::axum::Json(ok)) => {
                            let structured = ::axum_mcp::serde_json::to_value(&ok)
                                .unwrap_or(::axum_mcp::serde_json::Value::Null);
                            let text = match &structured {
                                ::axum_mcp::serde_json::Value::String(value) => value.clone(),
                                _ => ::axum_mcp::serde_json::to_string(&structured)
                                    .unwrap_or_else(|_| "serialization error".to_string()),
                            };
                            let structured_content = match structured {
                                ::axum_mcp::serde_json::Value::Object(_) => structured,
                                value => ::axum_mcp::serde_json::json!({ "result": value }),
                            };

                            ::axum_mcp::serde_json::json!({
                                "content": [{
                                    "type": "text",
                                    "text": text
                                }],
                                "structuredContent": structured_content,
                                "isError": false
                            })
                        }
                        Err((status, ::axum_mcp::axum::Json(err))) => {
                            let structured = ::axum_mcp::serde_json::to_value(&err)
                                .unwrap_or(::axum_mcp::serde_json::Value::Null);
                            let text = match &structured {
                                ::axum_mcp::serde_json::Value::String(value) => value.clone(),
                                _ => ::axum_mcp::serde_json::to_string(&structured)
                                    .unwrap_or_else(|_| "serialization error".to_string()),
                            };
                            let structured_content = match structured {
                                ::axum_mcp::serde_json::Value::Object(_) => structured,
                                value => ::axum_mcp::serde_json::json!({ "result": value }),
                            };

                            ::axum_mcp::serde_json::json!({
                                "content": [{
                                    "type": "text",
                                    "text": text
                                }],
                                "structuredContent": structured_content,
                                "isError": true,
                                "_meta": {
                                    "httpStatus": status.as_u16()
                                }
                            })
                        }
                    }
                })
            } else if is_named_generic(ty, "Json") {
                Ok(quote! {
                    let ::axum_mcp::axum::Json(ok) = __mcp_result;

                    let structured = ::axum_mcp::serde_json::to_value(&ok)
                        .unwrap_or(::axum_mcp::serde_json::Value::Null);
                    let text = match &structured {
                        ::axum_mcp::serde_json::Value::String(value) => value.clone(),
                        _ => ::axum_mcp::serde_json::to_string(&structured)
                            .unwrap_or_else(|_| "serialization error".to_string()),
                    };
                    let structured_content = match structured {
                        ::axum_mcp::serde_json::Value::Object(_) => structured,
                        value => ::axum_mcp::serde_json::json!({ "result": value }),
                    };

                    ::axum_mcp::serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": text
                        }],
                        "structuredContent": structured_content,
                        "isError": false
                    })
                })
            } else {
                Ok(quote! {
                    let structured = ::axum_mcp::serde_json::to_value(&__mcp_result)
                        .unwrap_or(::axum_mcp::serde_json::Value::Null);
                    let text = match &structured {
                        ::axum_mcp::serde_json::Value::String(value) => value.clone(),
                        _ => ::axum_mcp::serde_json::to_string(&structured)
                            .unwrap_or_else(|_| "serialization error".to_string()),
                    };
                    let structured_content = match structured {
                        ::axum_mcp::serde_json::Value::Object(_) => structured,
                        value => ::axum_mcp::serde_json::json!({ "result": value }),
                    };

                    ::axum_mcp::serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": text
                        }],
                        "structuredContent": structured_content,
                        "isError": false
                    })
                })
            }
        }
    }
}

fn is_result_json_response(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    let Some(result_segment) = type_path.path.segments.last() else {
        return false;
    };

    if result_segment.ident != "Result" {
        return false;
    }

    let PathArguments::AngleBracketed(result_args) = &result_segment.arguments else {
        return false;
    };

    if result_args.args.len() != 2 {
        return false;
    }

    let ok_is_json = matches!(
        &result_args.args[0],
        GenericArgument::Type(ok_ty) if is_named_generic(ok_ty, "Json")
    );

    let err_is_tuple = matches!(
        &result_args.args[1],
        GenericArgument::Type(Type::Tuple(tuple)) if tuple.elems.len() == 2
    );

    ok_is_json && err_is_tuple
}

fn is_named_generic(ty: &Type, expected_name: &str) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == expected_name)
}

fn compile_error(message: &'static str) -> proc_macro2::TokenStream {
    quote!(compile_error!(#message);)
}
