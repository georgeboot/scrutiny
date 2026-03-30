use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Lit, LitStr, Meta, Token, parenthesized};

/// Parsed struct-level attributes: `#[validate(attributes(...))]`
#[derive(Default)]
pub struct StructAttrs {
    /// Friendly field names: field_name -> display name
    pub attributes: Vec<(String, String)>,
}

/// A single parsed rule on a field.
#[derive(Debug, Clone)]
pub struct FieldRule {
    pub name: String,
    pub params: RuleParams,
    pub message: Option<String>,
    pub span: Span,
}

/// Parameters for a rule.
#[derive(Debug, Clone)]
pub enum RuleParams {
    /// No parameters: `required`, `email`, `bail`
    None,
    /// Single value: `min = 3`, `regex = "pattern"`
    Value(String),
    /// Named params: `between(min = 1, max = 10)`, `required_if(field = "role", value = "admin")`
    Named(Vec<(String, String)>),
    /// Positional list: `in_list("a", "b", "c")`
    List(Vec<String>),
}

/// All parsed info for a single field.
pub struct FieldInfo {
    pub name: String,
    pub rules: Vec<FieldRule>,
    pub is_option: bool,
    pub is_vec: bool,
    pub inner_type: Option<syn::Type>,
    pub ty: syn::Type,
}

/// Parse struct-level `#[validate(...)]` attributes.
pub fn parse_struct_attrs(attrs: &[syn::Attribute]) -> syn::Result<StructAttrs> {
    let mut result = StructAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("attributes") {
                let content;
                parenthesized!(content in meta.input);
                let pairs: Punctuated<FieldAttrPair, Token![,]> =
                    content.parse_terminated(FieldAttrPair::parse, Token![,])?;
                for pair in pairs {
                    result.attributes.push((pair.field, pair.value));
                }
                return Ok(());
            }
            // Skip unknown struct-level attributes
            Ok(())
        })?;
    }

    Ok(result)
}

struct FieldAttrPair {
    field: String,
    value: String,
}

impl Parse for FieldAttrPair {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let field: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: LitStr = input.parse()?;
        Ok(FieldAttrPair {
            field: field.to_string(),
            value: value.value(),
        })
    }
}

/// Parse field-level `#[validate(...)]` attributes into rules.
pub fn parse_field_rules(attrs: &[syn::Attribute]) -> syn::Result<Vec<FieldRule>> {
    let mut rules = Vec::new();

    for attr in attrs {
        if !attr.path().is_ident("validate") {
            continue;
        }

        let meta_list = match &attr.meta {
            Meta::List(list) => list,
            _ => continue,
        };

        // Parse as a comma-separated list of rule items
        let parsed: RuleList = syn::parse2(meta_list.tokens.clone())?;
        rules.extend(parsed.rules);
    }

    Ok(rules)
}

struct RuleList {
    rules: Vec<FieldRule>,
}

impl Parse for RuleList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut rules = Vec::new();

        while !input.is_empty() {
            let rule = parse_single_rule(input)?;
            rules.push(rule);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(RuleList { rules })
    }
}

fn parse_single_rule(input: ParseStream) -> syn::Result<FieldRule> {
    let name: Ident = input.parse()?;
    let name_str = name.to_string();
    let span = name.span();

    // Case 1: `rule = value` (e.g., `min = 3`, `regex = "pattern"`, `custom = my_fn`)
    if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        let value = parse_literal_value(input)?;
        return Ok(FieldRule {
            name: name_str,
            params: RuleParams::Value(value),
            message: None,
            span,
        });
    }

    // Case 2: `rule(...)` (e.g., `between(min = 1, max = 10)`, `required(message = "...")`)
    if input.peek(syn::token::Paren) {
        let content;
        parenthesized!(content in input);

        // Check if it's a list of string literals: `in_list("a", "b", "c")`
        if matches!(
            name_str.as_str(),
            "in_list" | "not_in" | "required_with_all" | "required_without_all"
        ) {
            return parse_list_rule(name_str, &content, span);
        }

        // Otherwise parse as named params (may include `message`)
        return parse_named_params_rule(name_str, &content, span);
    }

    // Case 3: bare rule: `required`, `email`, `bail`, `nullable`, `sometimes`, `dive`
    Ok(FieldRule {
        name: name_str,
        params: RuleParams::None,
        message: None,
        span,
    })
}

fn parse_list_rule(name: String, content: ParseStream, span: Span) -> syn::Result<FieldRule> {
    let mut items = Vec::new();
    let mut message = None;

    while !content.is_empty() {
        // Check for `message = "..."` at the end
        if content.peek(Ident) {
            let ident: Ident = content.parse()?;
            if ident == "message" {
                content.parse::<Token![=]>()?;
                let msg: LitStr = content.parse()?;
                message = Some(msg.value());
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
                continue;
            }
            return Err(syn::Error::new(
                ident.span(),
                "expected string literal or `message`",
            ));
        }

        let lit: LitStr = content.parse()?;
        items.push(lit.value());

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    Ok(FieldRule {
        name,
        params: RuleParams::List(items),
        message,
        span,
    })
}

fn parse_named_params_rule(
    name: String,
    content: ParseStream,
    span: Span,
) -> syn::Result<FieldRule> {
    let mut params = Vec::new();
    let mut message = None;

    while !content.is_empty() {
        let key: Ident = content.parse()?;
        content.parse::<Token![=]>()?;

        if key == "message" {
            let msg: LitStr = content.parse()?;
            message = Some(msg.value());
        } else {
            let value = parse_literal_value(content)?;
            params.push((key.to_string(), value));
        }

        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        }
    }

    // If only a message and no other params, this is a bare rule with custom message
    let rule_params = if params.is_empty() {
        RuleParams::None
    } else {
        RuleParams::Named(params)
    };

    Ok(FieldRule {
        name,
        params: rule_params,
        message,
        span,
    })
}

fn parse_literal_value(input: ParseStream) -> syn::Result<String> {
    // Try string literal
    if input.peek(LitStr) {
        let lit: LitStr = input.parse()?;
        return Ok(lit.value());
    }

    // Try other literals (integers, floats, bools)
    if input.peek(Lit) {
        let lit: Lit = input.parse()?;
        return Ok(match lit {
            Lit::Int(i) => i.base10_digits().to_string(),
            Lit::Float(f) => f.base10_digits().to_string(),
            Lit::Bool(b) => b.value.to_string(),
            Lit::Str(s) => s.value(),
            _ => return Err(syn::Error::new(input.span(), "unsupported literal type")),
        });
    }

    // Try an ident (for `custom = my_function_name`)
    if input.peek(Ident) {
        let ident: Ident = input.parse()?;
        return Ok(ident.to_string());
    }

    // Try a negative number
    if input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        let lit: Lit = input.parse()?;
        return Ok(format!(
            "-{}",
            match lit {
                Lit::Int(i) => i.base10_digits().to_string(),
                Lit::Float(f) => f.base10_digits().to_string(),
                _ => return Err(syn::Error::new(input.span(), "expected number after -")),
            }
        ));
    }

    Err(syn::Error::new(input.span(), "expected a value"))
}

/// Extract type info from a field type.
pub fn extract_type_info(ty: &syn::Type) -> (bool, bool, Option<syn::Type>) {
    let is_option;
    let is_vec;
    let inner;

    // Check for Option<T>
    if let Some(inner_ty) = extract_generic_arg(ty, "Option") {
        is_option = true;
        // Check for Option<Vec<T>>
        if let Some(vec_inner) = extract_generic_arg(&inner_ty, "Vec") {
            is_vec = true;
            inner = Some(vec_inner);
        } else {
            is_vec = false;
            inner = Some(inner_ty);
        }
    } else if let Some(vec_inner) = extract_generic_arg(ty, "Vec") {
        is_option = false;
        is_vec = true;
        inner = Some(vec_inner);
    } else {
        is_option = false;
        is_vec = false;
        inner = None;
    }

    (is_option, is_vec, inner)
}

fn extract_generic_arg(ty: &syn::Type, wrapper: &str) -> Option<syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        let segment = type_path.path.segments.last()?;
        if segment.ident != wrapper {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
        {
            return Some(inner.clone());
        }
    }
    None
}
