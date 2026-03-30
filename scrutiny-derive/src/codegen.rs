use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields};

use crate::parse::{
    FieldInfo, FieldRule, RuleParams, StructAttrs, extract_type_info, parse_field_rules,
    parse_struct_attrs,
};

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let struct_attrs = parse_struct_attrs(&input.attrs)?;

    match &input.data {
        Data::Struct(data) => expand_struct(name, &struct_attrs, &data.fields),
        Data::Enum(data) => expand_enum(name, &struct_attrs, data),
        Data::Union(_) => Err(syn::Error::new_spanned(&input, "Validate cannot be derived for unions")),
    }
}

fn expand_struct(
    name: &syn::Ident,
    struct_attrs: &StructAttrs,
    fields: &Fields,
) -> syn::Result<TokenStream> {
    match fields {
        Fields::Named(named) => {
            let field_infos = parse_named_fields(&named.named)?;
            let field_access_impl = gen_field_access(name, &field_infos);
            let validate_impl = gen_validate(name, &field_infos, struct_attrs)?;
            Ok(quote! {
                #field_access_impl
                #validate_impl
            })
        }
        Fields::Unnamed(unnamed) => expand_tuple_struct(name, struct_attrs, unnamed),
        Fields::Unit => {
            // Unit struct: always valid
            Ok(quote! {
                impl ::scrutiny::traits::FieldAccess for #name {
                    fn get_field_value(&self, _field_name: &str) -> ::scrutiny::value::FieldValue {
                        ::scrutiny::value::FieldValue::None
                    }
                }
                impl ::scrutiny::traits::Validate for #name {
                    fn validate(&self) -> ::std::result::Result<(), ::scrutiny::error::ValidationErrors> {
                        ::std::result::Result::Ok(())
                    }
                }
            })
        }
    }
}

fn expand_tuple_struct(
    name: &syn::Ident,
    struct_attrs: &StructAttrs,
    unnamed: &syn::FieldsUnnamed,
) -> syn::Result<TokenStream> {
    let mut field_infos = Vec::new();
    for (i, field) in unnamed.unnamed.iter().enumerate() {
        let rules = parse_field_rules(&field.attrs)?;
        let (is_option, is_vec, inner_type) = extract_type_info(&field.ty);
        field_infos.push(FieldInfo {
            name: format!("{}", i),
            rules,
            is_option,
            is_vec,
            inner_type,
            ty: field.ty.clone(),
        });
    }

    // FieldAccess: index-based lookup
    let access_arms: Vec<TokenStream> = field_infos
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let idx = syn::Index::from(i);
            let name_str = &f.name;
            let inner_ty = f.inner_type.as_ref().unwrap_or(&f.ty);
            if is_convertible_type(inner_ty) {
                quote! { #name_str => ::scrutiny::value::FieldValue::from(&self.#idx), }
            } else {
                quote! { #name_str => ::scrutiny::value::FieldValue::None, }
            }
        })
        .collect();

    // Validation: bind tuple fields to named vars, then use enum-style checks
    let mut bindings = Vec::new();
    let mut checks = Vec::new();

    for (i, fi) in field_infos.iter().enumerate() {
        let idx = syn::Index::from(i);
        let var = format_ident!("__field_{}", i);
        bindings.push(quote! { let #var = &self.#idx; });

        if fi.rules.is_empty() {
            continue;
        }

        let display_name = struct_attrs
            .attributes
            .iter()
            .find(|(n, _)| *n == fi.name)
            .map(|(_, d)| d.clone())
            .unwrap_or_else(|| fi.name.clone());

        let has_bail = fi.rules.iter().any(|r| r.name == "bail");
        let real_rules: Vec<&FieldRule> = fi
            .rules
            .iter()
            .filter(|r| !matches!(r.name.as_str(), "bail" | "nullable" | "sometimes"))
            .collect();

        let field_name = &fi.name;
        for rule in &real_rules {
            let check = gen_enum_field_check(rule, fi, &display_name, has_bail, &var, field_name)?;
            checks.push(check);
        }
    }

    Ok(quote! {
        impl ::scrutiny::traits::FieldAccess for #name {
            fn get_field_value(&self, field_name: &str) -> ::scrutiny::value::FieldValue {
                match field_name {
                    #(#access_arms)*
                    _ => ::scrutiny::value::FieldValue::None,
                }
            }
        }
        impl ::scrutiny::traits::Validate for #name {
            fn validate(&self) -> ::std::result::Result<(), ::scrutiny::error::ValidationErrors> {
                let mut errors = ::scrutiny::error::ValidationErrors::new();
                #(#bindings)*
                #(#checks)*
                if errors.is_empty() {
                    ::std::result::Result::Ok(())
                } else {
                    ::std::result::Result::Err(errors)
                }
            }
        }
    })
}

fn expand_enum(
    name: &syn::Ident,
    struct_attrs: &StructAttrs,
    data: &syn::DataEnum,
) -> syn::Result<TokenStream> {
    let mut variant_arms = Vec::new();

    for variant in &data.variants {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Named(named) => {
                let field_infos = parse_named_fields(&named.named)?;
                let has_rules = field_infos.iter().any(|f| !f.rules.is_empty());

                if !has_rules {
                    // No validation needed for this variant
                    let field_names: Vec<_> = named.named.iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();
                    variant_arms.push(quote! {
                        Self::#variant_name { #(#field_names: _),* } => {}
                    });
                    continue;
                }

                let field_bindings: Vec<_> = named.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                // Generate validation for each field in this variant
                let mut checks = Vec::new();
                for fi in &field_infos {
                    if fi.rules.is_empty() {
                        continue;
                    }
                    let display_name = struct_attrs
                        .attributes
                        .iter()
                        .find(|(n, _)| *n == fi.name)
                        .map(|(_, d)| d.clone())
                        .unwrap_or_else(|| fi.name.replace('_', " "));

                    let has_bail = fi.rules.iter().any(|r| r.name == "bail");
                    let real_rules: Vec<&FieldRule> = fi
                        .rules
                        .iter()
                        .filter(|r| !matches!(r.name.as_str(), "bail" | "nullable" | "sometimes"))
                        .collect();

                    if real_rules.is_empty() {
                        continue;
                    }

                    let field_ident = format_ident!("{}", fi.name);
                    let field_name = &fi.name;

                    // For enums, we need to generate checks that use the bound variable
                    // directly, not `self.field`. We create a temporary struct-like scope.
                    for rule in &real_rules {
                        let check = gen_enum_field_check(rule, fi, &display_name, has_bail, &field_ident, field_name)?;
                        checks.push(check);
                    }
                }

                variant_arms.push(quote! {
                    Self::#variant_name { #(#field_bindings),* } => {
                        #(#checks)*
                    }
                });
            }
            Fields::Unnamed(unnamed) => {
                // Tuple variants: validate inner fields by index
                let mut has_any_rules = false;
                let mut bindings = Vec::new();
                let mut checks = Vec::new();

                for (i, field) in unnamed.unnamed.iter().enumerate() {
                    let rules = parse_field_rules(&field.attrs)?;
                    let binding = format_ident!("__field_{}", i);
                    bindings.push(binding.clone());

                    if rules.is_empty() {
                        continue;
                    }
                    has_any_rules = true;
                    let (is_option, is_vec, inner_type) = extract_type_info(&field.ty);
                    let fi = FieldInfo {
                        name: format!("{}", i),
                        rules,
                        is_option,
                        is_vec,
                        inner_type,
                        ty: field.ty.clone(),
                    };

                    let display_name = fi.name.clone();
                    let has_bail = fi.rules.iter().any(|r| r.name == "bail");
                    let real_rules: Vec<&FieldRule> = fi
                        .rules
                        .iter()
                        .filter(|r| !matches!(r.name.as_str(), "bail" | "nullable" | "sometimes"))
                        .collect();

                    let field_name = &fi.name;
                    for rule in &real_rules {
                        let check = gen_enum_field_check(rule, &fi, &display_name, has_bail, &binding, field_name)?;
                        checks.push(check);
                    }
                }

                if !has_any_rules {
                    let wildcards: Vec<_> = bindings.iter().map(|_| quote! { _ }).collect();
                    variant_arms.push(quote! {
                        Self::#variant_name(#(#wildcards),*) => {}
                    });
                } else {
                    variant_arms.push(quote! {
                        Self::#variant_name(#(#bindings),*) => {
                            #(#checks)*
                        }
                    });
                }
            }
            Fields::Unit => {
                variant_arms.push(quote! {
                    Self::#variant_name => {}
                });
            }
        }
    }

    Ok(quote! {
        impl ::scrutiny::traits::FieldAccess for #name {
            fn get_field_value(&self, _field_name: &str) -> ::scrutiny::value::FieldValue {
                ::scrutiny::value::FieldValue::None
            }
        }
        impl ::scrutiny::traits::Validate for #name {
            fn validate(&self) -> ::std::result::Result<(), ::scrutiny::error::ValidationErrors> {
                let mut errors = ::scrutiny::error::ValidationErrors::new();
                match self {
                    #(#variant_arms)*
                }
                if errors.is_empty() {
                    ::std::result::Result::Ok(())
                } else {
                    ::std::result::Result::Err(errors)
                }
            }
        }
    })
}

fn parse_named_fields(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> syn::Result<Vec<FieldInfo>> {
    let mut field_infos = Vec::new();
    for field in fields {
        let name = field.ident.as_ref().unwrap().to_string();
        let rules = parse_field_rules(&field.attrs)?;
        let (is_option, is_vec, inner_type) = extract_type_info(&field.ty);
        field_infos.push(FieldInfo {
            name,
            rules,
            is_option,
            is_vec,
            inner_type,
            ty: field.ty.clone(),
        });
    }
    Ok(field_infos)
}

/// Classification of a field's inner type for rule generation.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FieldKind {
    String,
    Integer,
    Float,
    Vec,
    Other,
}

fn classify_type(ty: &syn::Type) -> FieldKind {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        let name = segment.ident.to_string();
        return match name.as_str() {
            "String" => FieldKind::String,
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" => FieldKind::Integer,
            "f32" | "f64" => FieldKind::Float,
            "Vec" => FieldKind::Vec,
            _ => FieldKind::Other,
        };
    }
    FieldKind::Other
}

/// Classify a field for min/max/between/size rule generation.
/// Checks Vec first (from field.is_vec), then classifies the inner type.
fn classify_field(field: &FieldInfo) -> FieldKind {
    if field.is_vec {
        return FieldKind::Vec;
    }
    let ty = field.inner_type.as_ref().unwrap_or(&field.ty);
    classify_type(ty)
}

/// Check if a type is a known primitive/string type that can be converted to FieldValue.
fn is_convertible_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        let name = segment.ident.to_string();
        return matches!(
            name.as_str(),
            "String" | "bool"
                | "i8" | "i16" | "i32" | "i64"
                | "u8" | "u16" | "u32" | "u64"
                | "f32" | "f64"
        );
    }
    false
}

/// Generate FieldAccess impl.
fn gen_field_access(struct_name: &syn::Ident, fields: &[FieldInfo]) -> TokenStream {
    let match_arms: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name_str = &f.name;
            let field_ident = format_ident!("{}", f.name);

            // Determine the inner type to check convertibility
            let inner_ty = f.inner_type.as_ref().unwrap_or(&f.ty);
            let convertible = is_convertible_type(inner_ty);

            if convertible {
                // Known type: use From impl
                quote! {
                    #name_str => ::scrutiny::value::FieldValue::from(&self.#field_ident),
                }
            } else {
                // Unknown type (struct, etc.): return None
                quote! {
                    #name_str => ::scrutiny::value::FieldValue::None,
                }
            }
        })
        .collect();

    quote! {
        impl ::scrutiny::traits::FieldAccess for #struct_name {
            fn get_field_value(&self, field_name: &str) -> ::scrutiny::value::FieldValue {
                match field_name {
                    #(#match_arms)*
                    _ => ::scrutiny::value::FieldValue::None,
                }
            }
        }
    }
}

/// Generate Validate impl.
fn gen_validate(
    struct_name: &syn::Ident,
    fields: &[FieldInfo],
    struct_attrs: &StructAttrs,
) -> syn::Result<TokenStream> {
    let mut field_validations = Vec::new();

    for field in fields {
        if field.rules.is_empty() {
            continue;
        }
        let validation = gen_field_validation(field, struct_attrs)?;
        field_validations.push(validation);
    }

    Ok(quote! {
        impl ::scrutiny::traits::Validate for #struct_name {
            fn validate(&self) -> ::std::result::Result<(), ::scrutiny::error::ValidationErrors> {
                let mut errors = ::scrutiny::error::ValidationErrors::new();

                #(#field_validations)*

                if errors.is_empty() {
                    ::std::result::Result::Ok(())
                } else {
                    ::std::result::Result::Err(errors)
                }
            }
        }
    })
}

/// Generate validation code for a single field.
fn gen_field_validation(field: &FieldInfo, struct_attrs: &StructAttrs) -> syn::Result<TokenStream> {
    let field_ident = format_ident!("{}", field.name);
    let field_name = &field.name;

    // Resolve the display name for this field
    let display_name = struct_attrs
        .attributes
        .iter()
        .find(|(name, _)| name == field_name)
        .map(|(_, display)| display.clone())
        .unwrap_or_else(|| field.name.replace('_', " "));

    let has_bail = field.rules.iter().any(|r| r.name == "bail");
    let has_nullable = field.rules.iter().any(|r| r.name == "nullable");
    let has_sometimes = field.rules.iter().any(|r| r.name == "sometimes");

    // Filter out meta-rules
    let real_rules: Vec<&FieldRule> = field
        .rules
        .iter()
        .filter(|r| !matches!(r.name.as_str(), "bail" | "nullable" | "sometimes"))
        .collect();

    if real_rules.is_empty() {
        return Ok(TokenStream::new());
    }

    // Generate individual rule checks
    let mut rule_checks = Vec::new();
    for rule in &real_rules {
        let check = gen_rule_check(rule, field, &display_name, has_bail)?;
        rule_checks.push(check);
    }

    let body = if has_bail {
        // Wrap in a labeled block for bail
        quote! {
            'bail: {
                #(#rule_checks)*
            }
        }
    } else {
        quote! {
            #(#rule_checks)*
        }
    };

    // Wrap in Option/sometimes checks
    if field.is_option {
        if has_sometimes {
            // sometimes + Option: skip entirely if None
            Ok(quote! {
                if self.#field_ident.is_some() {
                    #body
                }
            })
        } else if has_nullable {
            // nullable: skip non-required rules if None (but required can still fire)
            // For nullable, we separate required from the rest
            let has_required = real_rules.iter().any(|r| r.name == "required");
            if has_required {
                // required still checked, but other rules only if Some
                let required_rule = real_rules.iter().find(|r| r.name == "required").unwrap();
                let required_check = gen_rule_check(required_rule, field, &display_name, has_bail)?;
                let other_checks: Vec<TokenStream> = real_rules
                    .iter()
                    .filter(|r| r.name != "required")
                    .map(|r| gen_rule_check(r, field, &display_name, has_bail))
                    .collect::<syn::Result<Vec<_>>>()?;

                Ok(quote! {
                    #required_check
                    if self.#field_ident.is_some() {
                        #(#other_checks)*
                    }
                })
            } else {
                Ok(quote! {
                    if self.#field_ident.is_some() {
                        #body
                    }
                })
            }
        } else {
            // Default for Option: run required check normally, other rules only if Some
            // But we generate them naturally — the rule checks handle Option unwrapping
            Ok(body)
        }
    } else {
        Ok(body)
    }
}

/// Generate a single rule check.
fn gen_rule_check(
    rule: &FieldRule,
    field: &FieldInfo,
    display_name: &str,
    has_bail: bool,
) -> syn::Result<TokenStream> {
    let field_ident = format_ident!("{}", field.name);
    let field_name = &field.name;

    let bail_break = if has_bail {
        quote! { break 'bail; }
    } else {
        quote! {}
    };

    match rule.name.as_str() {
        "required" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is required.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    if !::scrutiny::rules::presence::is_present_option(&self.#field_ident) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("required", #msg));
                        #bail_break
                    }
                })
            } else {
                // Non-option required: check if string is empty, etc.
                Ok(quote! {
                    if !::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("required", #msg));
                        #bail_break
                    }
                })
            }
        }

        "email" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid email address.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "email", has_bail,
                quote! { ::scrutiny::rules::format::is_email(val) })
        }

        "url" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid URL.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "url", has_bail,
                quote! { ::scrutiny::rules::format::is_url(val) })
        }

        "alpha" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must only contain letters.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "alpha", has_bail,
                quote! { ::scrutiny::rules::string::is_alpha(val) })
        }

        "alpha_num" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must only contain letters and numbers.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "alpha_num", has_bail,
                quote! { ::scrutiny::rules::string::is_alpha_num(val) })
        }

        "alpha_dash" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must only contain letters, numbers, dashes, and underscores.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "alpha_dash", has_bail,
                quote! { ::scrutiny::rules::string::is_alpha_dash(val) })
        }

        "numeric" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a number.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "numeric", has_bail,
                quote! { ::scrutiny::rules::string::is_numeric(val) })
        }

        "ascii" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must only contain ASCII characters.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "ascii", has_bail,
                quote! { ::scrutiny::rules::format::is_ascii(val) })
        }

        "uuid" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid UUID.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "uuid", has_bail,
                quote! { ::scrutiny::rules::format::is_uuid(val) })
        }

        "ulid" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid ULID.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "ulid", has_bail,
                quote! { ::scrutiny::rules::format::is_ulid(val) })
        }

        "ip" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid IP address.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "ip", has_bail,
                quote! { ::scrutiny::rules::format::is_ip(val) })
        }

        "ipv4" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid IPv4 address.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "ipv4", has_bail,
                quote! { ::scrutiny::rules::format::is_ipv4(val) })
        }

        "ipv6" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid IPv6 address.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "ipv6", has_bail,
                quote! { ::scrutiny::rules::format::is_ipv6(val) })
        }

        "json" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be valid JSON.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "json", has_bail,
                quote! { ::scrutiny::rules::format::is_json(val) })
        }

        "mac_address" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid MAC address.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "mac_address", has_bail,
                quote! { ::scrutiny::rules::format::is_mac_address(val) })
        }

        "min" => {
            let value_str = extract_single_value(rule, "min")?;
            let kind = classify_field(field);
            match kind {
                FieldKind::Integer | FieldKind::Float => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must be at least {}.", display_name, value_str)
                    });
                    gen_numeric_rule_check(field, &field_ident, field_name, &msg, "min", has_bail, &value_str,
                        |v| quote! { *val >= #v })
                }
                FieldKind::Vec => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must have at least {} items.", display_name, value_str)
                    });
                    let min_val: usize = value_str.parse().map_err(|_| syn::Error::new(rule.span, "min must be an integer for Vec"))?;
                    gen_vec_rule_check(field, &field_ident, field_name, &msg, "min", has_bail,
                        quote! { val.len() >= #min_val })
                }
                _ => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must be at least {} characters.", display_name, value_str)
                    });
                    let min_val: usize = value_str.parse().map_err(|_| syn::Error::new(rule.span, "min must be a number"))?;
                    gen_string_rule_check(field, &field_ident, field_name, &msg, "min", has_bail,
                        quote! { ::scrutiny::rules::string::check_min_length(val, #min_val) })
                }
            }
        }

        "max" => {
            let value_str = extract_single_value(rule, "max")?;
            let kind = classify_field(field);
            match kind {
                FieldKind::Integer | FieldKind::Float => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must not exceed {}.", display_name, value_str)
                    });
                    gen_numeric_rule_check(field, &field_ident, field_name, &msg, "max", has_bail, &value_str,
                        |v| quote! { *val <= #v })
                }
                FieldKind::Vec => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must not have more than {} items.", display_name, value_str)
                    });
                    let max_val: usize = value_str.parse().map_err(|_| syn::Error::new(rule.span, "max must be an integer for Vec"))?;
                    gen_vec_rule_check(field, &field_ident, field_name, &msg, "max", has_bail,
                        quote! { val.len() <= #max_val })
                }
                _ => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must not exceed {} characters.", display_name, value_str)
                    });
                    let max_val: usize = value_str.parse().map_err(|_| syn::Error::new(rule.span, "max must be a number"))?;
                    gen_string_rule_check(field, &field_ident, field_name, &msg, "max", has_bail,
                        quote! { ::scrutiny::rules::string::check_max_length(val, #max_val) })
                }
            }
        }

        "between" => {
            let (min_str, max_str) = match &rule.params {
                RuleParams::Named(params) => {
                    let min = params.iter().find(|(k, _)| k == "min").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "between requires min"))?;
                    let max = params.iter().find(|(k, _)| k == "max").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "between requires max"))?;
                    (min, max)
                }
                _ => return Err(syn::Error::new(rule.span, "between requires min and max")),
            };
            let kind = classify_field(field);
            match kind {
                FieldKind::Integer | FieldKind::Float => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must be between {} and {}.", display_name, min_str, max_str)
                    });
                    let min_lit: proc_macro2::TokenStream = min_str.parse().map_err(|_| syn::Error::new(rule.span, "between min must be a number"))?;
                    let max_lit: proc_macro2::TokenStream = max_str.parse().map_err(|_| syn::Error::new(rule.span, "between max must be a number"))?;
                    let inner_ty = field.inner_type.as_ref().unwrap_or(&field.ty);
                    gen_numeric_rule_check_raw(field, &field_ident, field_name, &msg, "between", has_bail,
                        quote! { *val >= (#min_lit as #inner_ty) && *val <= (#max_lit as #inner_ty) })
                }
                FieldKind::Vec => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must have between {} and {} items.", display_name, min_str, max_str)
                    });
                    let min_val: usize = min_str.parse().map_err(|_| syn::Error::new(rule.span, "between min must be integer for Vec"))?;
                    let max_val: usize = max_str.parse().map_err(|_| syn::Error::new(rule.span, "between max must be integer for Vec"))?;
                    gen_vec_rule_check(field, &field_ident, field_name, &msg, "between", has_bail,
                        quote! { val.len() >= #min_val && val.len() <= #max_val })
                }
                _ => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must be between {} and {} characters.", display_name, min_str, max_str)
                    });
                    let min_val: usize = min_str.parse().map_err(|_| syn::Error::new(rule.span, "between min must be integer"))?;
                    let max_val: usize = max_str.parse().map_err(|_| syn::Error::new(rule.span, "between max must be integer"))?;
                    gen_string_rule_check(field, &field_ident, field_name, &msg, "between", has_bail,
                        quote! { ::scrutiny::rules::string::check_between_length(val, #min_val, #max_val) })
                }
            }
        }

        "regex" => {
            let pattern = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                RuleParams::Named(params) => params
                    .iter()
                    .find(|(k, _)| k == "pattern")
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| syn::Error::new(rule.span, "regex requires a pattern"))?,
                _ => return Err(syn::Error::new(rule.span, "regex requires a pattern")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} format is invalid.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "regex", has_bail,
                quote! { ::scrutiny::rules::format::matches_regex(val, #pattern) })
        }

        "in_list" => {
            let items = match &rule.params {
                RuleParams::List(items) => items.clone(),
                _ => return Err(syn::Error::new(rule.span, "in_list requires a list of values")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The selected {} is invalid.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "in_list", has_bail,
                quote! { ::scrutiny::rules::comparison::is_in(val, &[#(#items),*]) })
        }

        "not_in" => {
            let items = match &rule.params {
                RuleParams::List(items) => items.clone(),
                _ => return Err(syn::Error::new(rule.span, "not_in requires a list of values")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The selected {} is invalid.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "not_in", has_bail,
                quote! { ::scrutiny::rules::comparison::is_not_in(val, &[#(#items),*]) })
        }

        "confirmed" => {
            let confirmation_field = format!("{}_confirmation", field.name);
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} confirmation does not match.", display_name)
            });
            Ok(quote! {
                {
                    let a = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let b = ::scrutiny::traits::FieldAccess::get_field_value(self, #confirmation_field);
                    if !::scrutiny::rules::comparison::is_same(&a, &b) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("confirmed", #msg));
                        #bail_break
                    }
                }
            })
        }

        "same" => {
            let other = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                _ => return Err(syn::Error::new(rule.span, "same requires a field name")),
            };
            let other_display = other.replace('_', " ");
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must match {}.", display_name, other_display)
            });
            Ok(quote! {
                {
                    let a = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let b = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                    if !::scrutiny::rules::comparison::is_same(&a, &b) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("same", #msg));
                        #bail_break
                    }
                }
            })
        }

        "different" => {
            let other = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                _ => return Err(syn::Error::new(rule.span, "different requires a field name")),
            };
            let other_display = other.replace('_', " ");
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be different from {}.", display_name, other_display)
            });
            Ok(quote! {
                {
                    let a = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let b = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                    if !::scrutiny::rules::comparison::is_different(&a, &b) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("different", #msg));
                        #bail_break
                    }
                }
            })
        }

        "required_if" => {
            let (cond_field, cond_value) = match &rule.params {
                RuleParams::Named(params) => {
                    let f = params.iter().find(|(k, _)| k == "field").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "required_if requires field"))?;
                    let v = params.iter().find(|(k, _)| k == "value").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "required_if requires value"))?;
                    (f, v)
                }
                _ => return Err(syn::Error::new(rule.span, "required_if requires field and value")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is required.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other == ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if !::scrutiny::rules::presence::is_present_option(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_if", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other == ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if !::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_if", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            }
        }

        "required_unless" => {
            let (cond_field, cond_value) = match &rule.params {
                RuleParams::Named(params) => {
                    let f = params.iter().find(|(k, _)| k == "field").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "required_unless requires field"))?;
                    let v = params.iter().find(|(k, _)| k == "value").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "required_unless requires value"))?;
                    (f, v)
                }
                _ => return Err(syn::Error::new(rule.span, "required_unless requires field and value")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is required.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other != ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if !::scrutiny::rules::presence::is_present_option(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_unless", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other != ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if !::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_unless", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            }
        }

        "required_with" => {
            let other = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                _ => return Err(syn::Error::new(rule.span, "required_with requires a field name")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is required.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                        if !other.is_none() && !other.is_empty() {
                            if !::scrutiny::rules::presence::is_present_option(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_with", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                        if !other.is_none() && !other.is_empty() {
                            if !::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_with", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            }
        }

        "required_without" => {
            let other = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                _ => return Err(syn::Error::new(rule.span, "required_without requires a field name")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is required.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    {
                        let other_val = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                        if other_val.is_none() || other_val.is_empty() {
                            if !::scrutiny::rules::presence::is_present_option(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_without", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let other_val = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                        if other_val.is_none() || other_val.is_empty() {
                            if !::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("required_without", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            }
        }

        "starts_with" => {
            let prefix = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                RuleParams::Named(params) => params
                    .iter().find(|(k, _)| k == "value").map(|(_, v)| v.clone())
                    .ok_or_else(|| syn::Error::new(rule.span, "starts_with requires a value"))?,
                _ => return Err(syn::Error::new(rule.span, "starts_with requires a value")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must start with {}.", display_name, prefix)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "starts_with", has_bail,
                quote! { ::scrutiny::rules::string::starts_with(val, #prefix) })
        }

        "ends_with" => {
            let suffix = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                RuleParams::Named(params) => params
                    .iter().find(|(k, _)| k == "value").map(|(_, v)| v.clone())
                    .ok_or_else(|| syn::Error::new(rule.span, "ends_with requires a value"))?,
                _ => return Err(syn::Error::new(rule.span, "ends_with requires a value")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must end with {}.", display_name, suffix)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "ends_with", has_bail,
                quote! { ::scrutiny::rules::string::ends_with(val, #suffix) })
        }

        "nested" | "dive" => {
            gen_dive_check(field, &field_ident, field_name, has_bail)
        }

        "custom" => {
            let fn_name = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                _ => return Err(syn::Error::new(rule.span, "custom requires a function name")),
            };
            let fn_ident = format_ident!("{}", fn_name);
            Ok(quote! {
                if let ::std::result::Result::Err(msg) = #fn_ident(&self.#field_ident, self) {
                    errors.add(#field_name, ::scrutiny::error::ValidationError::new("custom", msg));
                    #bail_break
                }
            })
        }

        "string" => {
            // Type assertion — for String/Option<String> this is always true at compile time
            Ok(TokenStream::new())
        }

        "integer" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be an integer.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "integer", has_bail,
                quote! { ::scrutiny::rules::string::is_integer(val) })
        }

        "boolean" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be true or false.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "boolean", has_bail,
                quote! { matches!(val, "true" | "false" | "1" | "0") })
        }

        "accepted" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be accepted.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "accepted", has_bail,
                quote! { ::scrutiny::rules::presence::is_accepted(val) })
        }

        "accepted_if" => {
            let (cond_field, cond_value) = extract_field_value_params(rule)?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be accepted.", display_name)
            });
            let check = gen_string_rule_check(field, &field_ident, field_name, &msg, "accepted_if", has_bail,
                quote! { ::scrutiny::rules::presence::is_accepted(val) })?;
            Ok(quote! {
                {
                    let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                    if other == ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                        #check
                    }
                }
            })
        }

        "declined" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be declined.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "declined", has_bail,
                quote! { ::scrutiny::rules::presence::is_declined(val) })
        }

        "declined_if" => {
            let (cond_field, cond_value) = extract_field_value_params(rule)?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be declined.", display_name)
            });
            let check = gen_string_rule_check(field, &field_ident, field_name, &msg, "declined_if", has_bail,
                quote! { ::scrutiny::rules::presence::is_declined(val) })?;
            Ok(quote! {
                {
                    let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                    if other == ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                        #check
                    }
                }
            })
        }

        "filled" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must not be empty when present.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "filled", has_bail,
                quote! { ::scrutiny::rules::presence::is_filled(val) })
        }

        "prohibited" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is prohibited.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    if self.#field_ident.is_some() {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("prohibited", #msg));
                        #bail_break
                    }
                })
            } else {
                Ok(quote! {
                    if ::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("prohibited", #msg));
                        #bail_break
                    }
                })
            }
        }

        "prohibited_if" => {
            let (cond_field, cond_value) = extract_field_value_params(rule)?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is prohibited.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other == ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if self.#field_ident.is_some() {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("prohibited_if", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other == ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if ::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("prohibited_if", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            }
        }

        "prohibited_unless" => {
            let (cond_field, cond_value) = extract_field_value_params(rule)?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is prohibited.", display_name)
            });
            if field.is_option {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other != ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if self.#field_ident.is_some() {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("prohibited_unless", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let other = ::scrutiny::traits::FieldAccess::get_field_value(self, #cond_field);
                        if other != ::scrutiny::value::FieldValue::String(#cond_value.to_string()) {
                            if ::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new("prohibited_unless", #msg));
                                #bail_break
                            }
                        }
                    }
                })
            }
        }

        "required_with_all" => {
            let fields_list = match &rule.params {
                RuleParams::List(items) => items.clone(),
                _ => return Err(syn::Error::new(rule.span, "required_with_all requires a list of field names")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is required.", display_name)
            });
            let checks: Vec<TokenStream> = fields_list.iter().map(|f| {
                quote! {
                    {
                        let fv = ::scrutiny::traits::FieldAccess::get_field_value(self, #f);
                        !fv.is_none() && !fv.is_empty()
                    }
                }
            }).collect();
            let presence_check = gen_presence_check(field, &field_ident, field_name, &msg, "required_with_all", &bail_break);
            Ok(quote! {
                if #(#checks)&&* {
                    #presence_check
                }
            })
        }

        "required_without_all" => {
            let fields_list = match &rule.params {
                RuleParams::List(items) => items.clone(),
                _ => return Err(syn::Error::new(rule.span, "required_without_all requires a list of field names")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field is required.", display_name)
            });
            let checks: Vec<TokenStream> = fields_list.iter().map(|f| {
                quote! {
                    {
                        let fv = ::scrutiny::traits::FieldAccess::get_field_value(self, #f);
                        fv.is_none() || fv.is_empty()
                    }
                }
            }).collect();
            let presence_check = gen_presence_check(field, &field_ident, field_name, &msg, "required_without_all", &bail_break);
            Ok(quote! {
                if #(#checks)&&* {
                    #presence_check
                }
            })
        }

        "not_regex" => {
            let pattern = match &rule.params {
                RuleParams::Value(v) => v.clone(),
                RuleParams::Named(params) => params
                    .iter()
                    .find(|(k, _)| k == "pattern")
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| syn::Error::new(rule.span, "not_regex requires a pattern"))?,
                _ => return Err(syn::Error::new(rule.span, "not_regex requires a pattern")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} format is invalid.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "not_regex", has_bail,
                quote! { ::scrutiny::rules::format::not_matches_regex(val, #pattern) })
        }

        "contains" => {
            let needle = extract_single_value(rule, "contains")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must contain {}.", display_name, needle)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "contains", has_bail,
                quote! { ::scrutiny::rules::string::contains(val, #needle) })
        }

        "doesnt_contain" => {
            let needle = extract_single_value(rule, "doesnt_contain")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must not contain {}.", display_name, needle)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "doesnt_contain", has_bail,
                quote! { ::scrutiny::rules::string::doesnt_contain(val, #needle) })
        }

        "doesnt_start_with" => {
            let prefix = extract_single_value(rule, "doesnt_start_with")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must not start with {}.", display_name, prefix)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "doesnt_start_with", has_bail,
                quote! { ::scrutiny::rules::string::doesnt_start_with(val, #prefix) })
        }

        "doesnt_end_with" => {
            let suffix = extract_single_value(rule, "doesnt_end_with")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must not end with {}.", display_name, suffix)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "doesnt_end_with", has_bail,
                quote! { ::scrutiny::rules::string::doesnt_end_with(val, #suffix) })
        }

        "uppercase" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be uppercase.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "uppercase", has_bail,
                quote! { ::scrutiny::rules::string::is_uppercase(val) })
        }

        "lowercase" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be lowercase.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "lowercase", has_bail,
                quote! { ::scrutiny::rules::string::is_lowercase(val) })
        }

        "gt" => {
            let other = extract_single_value(rule, "gt")?;
            let other_display = other.replace('_', " ");
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be greater than {}.", display_name, other_display)
            });
            Ok(quote! {
                {
                    let a = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let b = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                    if !::scrutiny::rules::comparison::is_gt(&a, &b) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("gt", #msg));
                        #bail_break
                    }
                }
            })
        }

        "gte" => {
            let other = extract_single_value(rule, "gte")?;
            let other_display = other.replace('_', " ");
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be greater than or equal to {}.", display_name, other_display)
            });
            Ok(quote! {
                {
                    let a = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let b = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                    if !::scrutiny::rules::comparison::is_gte(&a, &b) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("gte", #msg));
                        #bail_break
                    }
                }
            })
        }

        "lt" => {
            let other = extract_single_value(rule, "lt")?;
            let other_display = other.replace('_', " ");
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be less than {}.", display_name, other_display)
            });
            Ok(quote! {
                {
                    let a = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let b = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                    if !::scrutiny::rules::comparison::is_lt(&a, &b) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("lt", #msg));
                        #bail_break
                    }
                }
            })
        }

        "lte" => {
            let other = extract_single_value(rule, "lte")?;
            let other_display = other.replace('_', " ");
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be less than or equal to {}.", display_name, other_display)
            });
            Ok(quote! {
                {
                    let a = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let b = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                    if !::scrutiny::rules::comparison::is_lte(&a, &b) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("lte", #msg));
                        #bail_break
                    }
                }
            })
        }

        "size" => {
            let value_str = extract_single_value(rule, "size")?;
            let kind = classify_field(field);
            match kind {
                FieldKind::Integer | FieldKind::Float => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must be exactly {}.", display_name, value_str)
                    });
                    gen_numeric_rule_check(field, &field_ident, field_name, &msg, "size", has_bail, &value_str,
                        |v| quote! { *val == #v })
                }
                FieldKind::Vec => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must have exactly {} items.", display_name, value_str)
                    });
                    let size_val: usize = value_str.parse().map_err(|_| syn::Error::new(rule.span, "size must be integer for Vec"))?;
                    gen_vec_rule_check(field, &field_ident, field_name, &msg, "size", has_bail,
                        quote! { val.len() == #size_val })
                }
                _ => {
                    let msg = rule.message.clone().unwrap_or_else(|| {
                        format!("The {} field must be exactly {} characters.", display_name, value_str)
                    });
                    let size_val: usize = value_str.parse().map_err(|_| syn::Error::new(rule.span, "size must be a number"))?;
                    gen_string_rule_check(field, &field_ident, field_name, &msg, "size", has_bail,
                        quote! { ::scrutiny::rules::string::check_size(val, #size_val) })
                }
            }
        }

        "digits" => {
            let value_str = extract_single_value(rule, "digits")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be {} digits.", display_name, value_str)
            });
            let count: usize = value_str.parse().map_err(|_| {
                syn::Error::new(rule.span, "digits value must be a positive integer")
            })?;
            gen_string_rule_check(field, &field_ident, field_name, &msg, "digits", has_bail,
                quote! { ::scrutiny::rules::numeric::check_digits(val, #count) })
        }

        "digits_between" => {
            let (min_str, max_str) = match &rule.params {
                RuleParams::Named(params) => {
                    let min = params.iter().find(|(k, _)| k == "min").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "digits_between requires min"))?;
                    let max = params.iter().find(|(k, _)| k == "max").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "digits_between requires max"))?;
                    (min, max)
                }
                _ => return Err(syn::Error::new(rule.span, "digits_between requires min and max")),
            };
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be between {} and {} digits.", display_name, min_str, max_str)
            });
            let min_val: usize = min_str.parse().map_err(|_| syn::Error::new(rule.span, "digits_between min must be integer"))?;
            let max_val: usize = max_str.parse().map_err(|_| syn::Error::new(rule.span, "digits_between max must be integer"))?;
            gen_string_rule_check(field, &field_ident, field_name, &msg, "digits_between", has_bail,
                quote! { ::scrutiny::rules::numeric::check_digits_between(val, #min_val, #max_val) })
        }

        "multiple_of" => {
            let value_str = extract_single_value(rule, "multiple_of")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a multiple of {}.", display_name, value_str)
            });
            let n: f64 = value_str.parse().map_err(|_| {
                syn::Error::new(rule.span, "multiple_of value must be a number")
            })?;
            gen_string_rule_check(field, &field_ident, field_name, &msg, "multiple_of", has_bail,
                quote! {
                    {
                        let parsed: ::std::result::Result<f64, _> = val.parse();
                        parsed.is_ok_and(|v| ::scrutiny::rules::numeric::is_multiple_of(v, #n))
                    }
                })
        }

        "decimal" => {
            let (min_str, max_str) = match &rule.params {
                RuleParams::Value(v) => (v.clone(), None),
                RuleParams::Named(params) => {
                    let min = params.iter().find(|(k, _)| k == "min").map(|(_, v)| v.clone())
                        .ok_or_else(|| syn::Error::new(rule.span, "decimal requires min"))?;
                    let max = params.iter().find(|(k, _)| k == "max").map(|(_, v)| v.clone());
                    (min, max)
                }
                _ => return Err(syn::Error::new(rule.span, "decimal requires parameters")),
            };
            let min_places: usize = min_str.parse().map_err(|_| syn::Error::new(rule.span, "decimal min must be integer"))?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                match &max_str {
                    Some(max) => format!("The {} field must have between {} and {} decimal places.", display_name, min_str, max),
                    None => format!("The {} field must have {} decimal places.", display_name, min_str),
                }
            });
            match max_str {
                Some(max) => {
                    let max_places: usize = max.parse().map_err(|_| syn::Error::new(rule.span, "decimal max must be integer"))?;
                    gen_string_rule_check(field, &field_ident, field_name, &msg, "decimal", has_bail,
                        quote! { ::scrutiny::rules::numeric::check_decimal(val, #min_places, Some(#max_places)) })
                }
                None => {
                    gen_string_rule_check(field, &field_ident, field_name, &msg, "decimal", has_bail,
                        quote! { ::scrutiny::rules::numeric::check_decimal(val, #min_places, None) })
                }
            }
        }

        "date" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid ISO 8601 date (YYYY-MM-DD).", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "date", has_bail,
                quote! { ::scrutiny::rules::format::is_iso_date(val) })
        }

        "datetime" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid ISO 8601 datetime.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "datetime", has_bail,
                quote! { ::scrutiny::rules::format::is_iso_datetime(val) })
        }

        "date_equals" => {
            let other = extract_single_value(rule, "date_equals")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be equal to {}.", display_name, other)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "date_equals", has_bail,
                quote! { ::scrutiny::rules::format::is_date_equals(val, #other) })
        }

        "before" => {
            let other = extract_single_value(rule, "before")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a date before {}.", display_name, other)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "before", has_bail,
                quote! { ::scrutiny::rules::format::is_before(val, #other) })
        }

        "after" => {
            let other = extract_single_value(rule, "after")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a date after {}.", display_name, other)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "after", has_bail,
                quote! { ::scrutiny::rules::format::is_after(val, #other) })
        }

        "before_or_equal" => {
            let other = extract_single_value(rule, "before_or_equal")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a date before or equal to {}.", display_name, other)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "before_or_equal", has_bail,
                quote! { ::scrutiny::rules::format::is_before_or_equal(val, #other) })
        }

        "after_or_equal" => {
            let other = extract_single_value(rule, "after_or_equal")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a date after or equal to {}.", display_name, other)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "after_or_equal", has_bail,
                quote! { ::scrutiny::rules::format::is_after_or_equal(val, #other) })
        }

        "hex_color" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid hex color.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "hex_color", has_bail,
                quote! { ::scrutiny::rules::format::is_hex_color(val) })
        }

        "timezone" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must be a valid timezone.", display_name)
            });
            gen_string_rule_check(field, &field_ident, field_name, &msg, "timezone", has_bail,
                quote! { ::scrutiny::rules::format::is_timezone(val) })
        }

        "in_array" => {
            let other = extract_single_value(rule, "in_array")?;
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must exist in {}.", display_name, other.replace('_', " "))
            });
            Ok(quote! {
                {
                    let val = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    let arr = ::scrutiny::traits::FieldAccess::get_field_value(self, #other);
                    if !::scrutiny::rules::comparison::is_in_array(&val, &arr) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("in_array", #msg));
                        #bail_break
                    }
                }
            })
        }

        "distinct" => {
            let msg = rule.message.clone().unwrap_or_else(|| {
                format!("The {} field must not have duplicate values.", display_name)
            });
            Ok(quote! {
                {
                    let val = ::scrutiny::traits::FieldAccess::get_field_value(self, #field_name);
                    if !::scrutiny::rules::comparison::is_distinct(&val) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("distinct", #msg));
                        #bail_break
                    }
                }
            })
        }

        _ => Err(syn::Error::new(rule.span, format!("unknown validation rule: {}", rule.name))),
    }
}

/// Helper: extract (field, value) from named params.
fn extract_field_value_params(rule: &FieldRule) -> syn::Result<(String, String)> {
    match &rule.params {
        RuleParams::Named(params) => {
            let f = params.iter().find(|(k, _)| k == "field").map(|(_, v)| v.clone())
                .ok_or_else(|| syn::Error::new(rule.span, format!("{} requires field", rule.name)))?;
            let v = params.iter().find(|(k, _)| k == "value").map(|(_, v)| v.clone())
                .ok_or_else(|| syn::Error::new(rule.span, format!("{} requires value", rule.name)))?;
            Ok((f, v))
        }
        _ => Err(syn::Error::new(rule.span, format!("{} requires field and value", rule.name))),
    }
}

/// Generate a rule check for an enum variant field.
/// Uses the bound variable directly instead of `self.field`.
fn gen_enum_field_check(
    rule: &FieldRule,
    field: &FieldInfo,
    display_name: &str,
    has_bail: bool,
    var_ident: &syn::Ident,
    field_name: &str,
) -> syn::Result<TokenStream> {
    let bail_break = if has_bail {
        quote! { break 'bail; }
    } else {
        quote! {}
    };

    let msg = rule.message.clone().unwrap_or_else(|| {
        default_message_for_rule(&rule.name, display_name, &rule.params)
    });

    // For string-based rules on Option<String>, generate if-let check
    // For required, generate presence check
    // For other rules, delegate to appropriate check

    match rule.name.as_str() {
        "required" => {
            if field.is_option {
                Ok(quote! {
                    if !::scrutiny::rules::presence::is_present_option(#var_ident) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("required", #msg));
                        #bail_break
                    }
                })
            } else {
                Ok(quote! {
                    if !::scrutiny::rules::presence::Presentable::is_present(#var_ident) {
                        errors.add(#field_name, ::scrutiny::error::ValidationError::new("required", #msg));
                        #bail_break
                    }
                })
            }
        }

        "nested" | "dive" => {
            if field.is_option {
                Ok(quote! {
                    if let Some(ref inner) = #var_ident {
                        if let ::std::result::Result::Err(nested_errors) = ::scrutiny::traits::Validate::validate(inner) {
                            errors.merge_with_prefix(#field_name, nested_errors);
                        }
                    }
                })
            } else {
                Ok(quote! {
                    if let ::std::result::Result::Err(nested_errors) = ::scrutiny::traits::Validate::validate(#var_ident) {
                        errors.merge_with_prefix(#field_name, nested_errors);
                    }
                })
            }
        }

        // Numeric min/max for enum/tuple fields
        "min" | "max" if matches!(classify_field(field), FieldKind::Integer | FieldKind::Float) => {
            let rule_name = &rule.name;
            let value_str = extract_single_value(rule, &rule.name)?;
            let raw_lit: proc_macro2::TokenStream = value_str.parse().map_err(|_| {
                syn::Error::new(rule.span, format!("{} value must be a number", rule.name))
            })?;
            let inner_ty = field.inner_type.as_ref().unwrap_or(&field.ty);
            let lit = quote! { (#raw_lit as #inner_ty) };
            let check = if rule.name == "min" {
                quote! { *val >= #lit }
            } else {
                quote! { *val <= #lit }
            };

            if field.is_option {
                Ok(quote! {
                    if let Some(val) = #var_ident {
                        if !(#check) {
                            errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                            #bail_break
                        }
                    }
                })
            } else {
                Ok(quote! {
                    {
                        let val = #var_ident;
                        if !(#check) {
                            errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                            #bail_break
                        }
                    }
                })
            }
        }

        // For all other rules, wrap with Option check and delegate to string check
        _ => {
            let rule_name = &rule.name;
            let check_expr = gen_string_check_expr(rule, field, display_name)?;

            if let Some(check) = check_expr {
                if field.is_option {
                    Ok(quote! {
                        if let Some(val) = #var_ident {
                            if !#check {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                                #bail_break
                            }
                        }
                    })
                } else {
                    Ok(quote! {
                        {
                            let val = #var_ident;
                            let val: &str = val.as_ref();
                            if !#check {
                                errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                                #bail_break
                            }
                        }
                    })
                }
            } else {
                Ok(TokenStream::new())
            }
        }
    }
}

/// Get the string check expression for a rule (without the error handling wrapper).
/// Returns None for rules that need cross-field access (same, different, gt, etc.)
fn gen_string_check_expr(
    rule: &FieldRule,
    field: &FieldInfo,
    _display_name: &str,
) -> syn::Result<Option<TokenStream>> {
    Ok(match rule.name.as_str() {
        "email" => Some(quote! { ::scrutiny::rules::format::is_email(val) }),
        "url" => Some(quote! { ::scrutiny::rules::format::is_url(val) }),
        "alpha" => Some(quote! { ::scrutiny::rules::string::is_alpha(val) }),
        "alpha_num" => Some(quote! { ::scrutiny::rules::string::is_alpha_num(val) }),
        "alpha_dash" => Some(quote! { ::scrutiny::rules::string::is_alpha_dash(val) }),
        "numeric" => Some(quote! { ::scrutiny::rules::string::is_numeric(val) }),
        "integer" => Some(quote! { ::scrutiny::rules::string::is_integer(val) }),
        "ascii" => Some(quote! { ::scrutiny::rules::format::is_ascii(val) }),
        "uuid" => Some(quote! { ::scrutiny::rules::format::is_uuid(val) }),
        "ulid" => Some(quote! { ::scrutiny::rules::format::is_ulid(val) }),
        "ip" => Some(quote! { ::scrutiny::rules::format::is_ip(val) }),
        "ipv4" => Some(quote! { ::scrutiny::rules::format::is_ipv4(val) }),
        "ipv6" => Some(quote! { ::scrutiny::rules::format::is_ipv6(val) }),
        "json" => Some(quote! { ::scrutiny::rules::format::is_json(val) }),
        "mac_address" => Some(quote! { ::scrutiny::rules::format::is_mac_address(val) }),
        "hex_color" => Some(quote! { ::scrutiny::rules::format::is_hex_color(val) }),
        "uppercase" => Some(quote! { ::scrutiny::rules::string::is_uppercase(val) }),
        "lowercase" => Some(quote! { ::scrutiny::rules::string::is_lowercase(val) }),
        "date" => Some(quote! { ::scrutiny::rules::format::is_iso_date(val) }),
        "datetime" => Some(quote! { ::scrutiny::rules::format::is_iso_datetime(val) }),
        "min" => {
            let v = extract_single_value(rule, "min")?;
            let kind = classify_field(field);
            match kind {
                FieldKind::Integer | FieldKind::Float => {
                    let inner_ty = field.inner_type.as_ref().unwrap_or(&field.ty);
                    let raw: proc_macro2::TokenStream = v.parse().unwrap();
                    return Ok(Some(quote! { { let val = #raw as #inner_ty; *val >= val } }));
                }
                _ => {
                    let n: usize = v.parse().unwrap_or(0);
                    Some(quote! { ::scrutiny::rules::string::check_min_length(val, #n) })
                }
            }
        }
        "max" => {
            let v = extract_single_value(rule, "max")?;
            let kind = classify_field(field);
            match kind {
                FieldKind::Integer | FieldKind::Float => {
                    let inner_ty = field.inner_type.as_ref().unwrap_or(&field.ty);
                    let raw: proc_macro2::TokenStream = v.parse().unwrap();
                    return Ok(Some(quote! { { let val = #raw as #inner_ty; *val <= val } }));
                }
                _ => {
                    let n: usize = v.parse().unwrap_or(0);
                    Some(quote! { ::scrutiny::rules::string::check_max_length(val, #n) })
                }
            }
        }
        "regex" => {
            let pattern = extract_single_value(rule, "regex")?;
            Some(quote! { ::scrutiny::rules::format::matches_regex(val, #pattern) })
        }
        "not_regex" => {
            let pattern = extract_single_value(rule, "not_regex")?;
            Some(quote! { ::scrutiny::rules::format::not_matches_regex(val, #pattern) })
        }
        "contains" => {
            let needle = extract_single_value(rule, "contains")?;
            Some(quote! { ::scrutiny::rules::string::contains(val, #needle) })
        }
        "starts_with" => {
            let prefix = extract_single_value(rule, "starts_with")?;
            Some(quote! { ::scrutiny::rules::string::starts_with(val, #prefix) })
        }
        "ends_with" => {
            let suffix = extract_single_value(rule, "ends_with")?;
            Some(quote! { ::scrutiny::rules::string::ends_with(val, #suffix) })
        }
        "in_list" => {
            let items = match &rule.params {
                RuleParams::List(items) => items.clone(),
                _ => return Ok(None),
            };
            Some(quote! { ::scrutiny::rules::comparison::is_in(val, &[#(#items),*]) })
        }
        "string" | "boolean" | "filled" | "accepted" | "declined" => {
            // These work the same way in enums
            match rule.name.as_str() {
                "filled" => Some(quote! { ::scrutiny::rules::presence::is_filled(val) }),
                "accepted" => Some(quote! { ::scrutiny::rules::presence::is_accepted(val) }),
                "declined" => Some(quote! { ::scrutiny::rules::presence::is_declined(val) }),
                "boolean" => Some(quote! { matches!(val, "true" | "false" | "1" | "0") }),
                _ => Some(quote! { true }),
            }
        }
        // Cross-field rules don't work in enum context (no FieldAccess)
        _ => None,
    })
}

/// Default message for a rule (used by enum variant checks).
fn default_message_for_rule(rule_name: &str, display_name: &str, _params: &RuleParams) -> String {
    match rule_name {
        "required" => format!("The {} field is required.", display_name),
        "email" => format!("The {} field must be a valid email address.", display_name),
        "url" => format!("The {} field must be a valid URL.", display_name),
        "min" => format!("The {} field is too small.", display_name),
        "max" => format!("The {} field is too large.", display_name),
        "alpha" => format!("The {} field must only contain letters.", display_name),
        "uuid" => format!("The {} field must be a valid UUID.", display_name),
        "date" => format!("The {} field must be a valid ISO 8601 date (YYYY-MM-DD).", display_name),
        _ => format!("The {} field is invalid.", display_name),
    }
}

/// Helper: extract a single value from Value or Named(value=...) params.
fn extract_single_value(rule: &FieldRule, name: &str) -> syn::Result<String> {
    match &rule.params {
        RuleParams::Value(v) => Ok(v.clone()),
        RuleParams::Named(params) => params
            .iter()
            .find(|(k, _)| k == "value")
            .map(|(_, v)| v.clone())
            .ok_or_else(|| syn::Error::new(rule.span, format!("{} requires a value", name))),
        _ => Err(syn::Error::new(rule.span, format!("{} requires a value", name))),
    }
}

/// Helper: generate a presence check (required-like) for conditional required rules.
fn gen_presence_check(
    field: &FieldInfo,
    field_ident: &syn::Ident,
    field_name: &str,
    msg: &str,
    rule_name: &str,
    bail_break: &TokenStream,
) -> TokenStream {
    if field.is_option {
        quote! {
            if !::scrutiny::rules::presence::is_present_option(&self.#field_ident) {
                errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                #bail_break
            }
        }
    } else {
        quote! {
            if !::scrutiny::rules::presence::Presentable::is_present(&self.#field_ident) {
                errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                #bail_break
            }
        }
    }
}

/// Helper to generate a string-based rule check, handling Option<String> unwrapping.
fn gen_string_rule_check(
    field: &FieldInfo,
    field_ident: &syn::Ident,
    field_name: &str,
    msg: &str,
    rule_name: &str,
    has_bail: bool,
    check_expr: TokenStream,
) -> syn::Result<TokenStream> {
    let bail_break = if has_bail {
        quote! { break 'bail; }
    } else {
        quote! {}
    };

    if field.is_option {
        Ok(quote! {
            if let Some(ref val) = self.#field_ident {
                if !#check_expr {
                    errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                    #bail_break
                }
            }
        })
    } else {
        Ok(quote! {
            {
                let val = &self.#field_ident;
                let val: &str = val.as_ref();
                if !#check_expr {
                    errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                    #bail_break
                }
            }
        })
    }
}

/// Helper to generate a numeric rule check, handling Option<T> unwrapping.
/// `make_check` receives the parsed literal as a TokenStream and returns the condition.
#[allow(clippy::too_many_arguments)]
fn gen_numeric_rule_check(
    field: &FieldInfo,
    field_ident: &syn::Ident,
    field_name: &str,
    msg: &str,
    rule_name: &str,
    has_bail: bool,
    value_str: &str,
    make_check: impl FnOnce(proc_macro2::TokenStream) -> TokenStream,
) -> syn::Result<TokenStream> {
    let raw_lit: proc_macro2::TokenStream = value_str.parse().map_err(|_| {
        syn::Error::new_spanned(field_ident, format!("{} value must be a number", rule_name))
    })?;
    let inner_ty = field.inner_type.as_ref().unwrap_or(&field.ty);
    let lit = quote! { (#raw_lit as #inner_ty) };
    let check_expr = make_check(lit);
    gen_numeric_rule_check_raw(field, field_ident, field_name, msg, rule_name, has_bail, check_expr)
}

/// Helper to generate a numeric rule check with a pre-built check expression.
fn gen_numeric_rule_check_raw(
    field: &FieldInfo,
    field_ident: &syn::Ident,
    field_name: &str,
    msg: &str,
    rule_name: &str,
    has_bail: bool,
    check_expr: TokenStream,
) -> syn::Result<TokenStream> {
    let bail_break = if has_bail {
        quote! { break 'bail; }
    } else {
        quote! {}
    };

    if field.is_option {
        Ok(quote! {
            if let Some(ref val) = self.#field_ident {
                if !(#check_expr) {
                    errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                    #bail_break
                }
            }
        })
    } else {
        Ok(quote! {
            {
                let val = &self.#field_ident;
                if !(#check_expr) {
                    errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                    #bail_break
                }
            }
        })
    }
}

/// Helper to generate a Vec rule check, handling Option<Vec<T>> unwrapping.
fn gen_vec_rule_check(
    field: &FieldInfo,
    field_ident: &syn::Ident,
    field_name: &str,
    msg: &str,
    rule_name: &str,
    has_bail: bool,
    check_expr: TokenStream,
) -> syn::Result<TokenStream> {
    let bail_break = if has_bail {
        quote! { break 'bail; }
    } else {
        quote! {}
    };

    if field.is_option {
        Ok(quote! {
            if let Some(ref val) = self.#field_ident {
                if !(#check_expr) {
                    errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                    #bail_break
                }
            }
        })
    } else {
        Ok(quote! {
            {
                let val = &self.#field_ident;
                if !(#check_expr) {
                    errors.add(#field_name, ::scrutiny::error::ValidationError::new(#rule_name, #msg));
                    #bail_break
                }
            }
        })
    }
}

/// Generate dive check for nested structs and Vec items.
fn gen_dive_check(
    field: &FieldInfo,
    field_ident: &syn::Ident,
    field_name: &str,
    has_bail: bool,
) -> syn::Result<TokenStream> {
    let bail_break = if has_bail {
        quote! { break 'bail; }
    } else {
        quote! {}
    };

    if field.is_option && field.is_vec {
        // Option<Vec<T>> — validate each element if Some
        Ok(quote! {
            if let Some(ref items) = self.#field_ident {
                for (i, item) in items.iter().enumerate() {
                    if let ::std::result::Result::Err(nested_errors) = ::scrutiny::traits::Validate::validate(item) {
                        errors.merge_with_prefix(&format!("{}.{}", #field_name, i), nested_errors);
                    }
                }
            }
        })
    } else if field.is_option {
        // Option<T> — validate inner if Some
        Ok(quote! {
            if let Some(ref inner) = self.#field_ident {
                if let ::std::result::Result::Err(nested_errors) = ::scrutiny::traits::Validate::validate(inner) {
                    errors.merge_with_prefix(#field_name, nested_errors);
                    #bail_break
                }
            }
        })
    } else if field.is_vec {
        // Vec<T> — validate each element
        Ok(quote! {
            for (i, item) in self.#field_ident.iter().enumerate() {
                if let ::std::result::Result::Err(nested_errors) = ::scrutiny::traits::Validate::validate(item) {
                    errors.merge_with_prefix(&format!("{}.{}", #field_name, i), nested_errors);
                }
            }
        })
    } else {
        // T — validate directly
        Ok(quote! {
            if let ::std::result::Result::Err(nested_errors) = ::scrutiny::traits::Validate::validate(&self.#field_ident) {
                errors.merge_with_prefix(#field_name, nested_errors);
                #bail_break
            }
        })
    }
}
