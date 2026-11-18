//! Attribute and function macros for the slot surface.

use std::path::Path;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitStr, Path as SynPath, parse_macro_input};

/// Selectors every theme must implement.
const THEME_DEF: &str = include_str!("../../../assets/slots_theme_def.css");

/// Declares a tab type and the function that renders its body.
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Tab(render: TokenStream, item: TokenStream) -> TokenStream {
    let render = parse_macro_input!(render as SynPath);
    let item = parse_macro_input!(item as DeriveInput);
    let ident = &item.ident;
    let generics = item.generics.split_for_impl();
    let impl_generics = generics.0;
    let ty_generics = generics.1;
    let where_clause = generics.2;
    quote! {
        #item

        impl #impl_generics ::bevy_dioxus_render::slots::Name for #ident #ty_generics #where_clause {
            fn name() -> &'static str {
                stringify!(#ident)
            }
        }

        impl #impl_generics ::bevy_dioxus_render::slots::Tab for #ident #ty_generics #where_clause {
            fn element() -> ::bevy_dioxus_render::slots::TabFn {
                #render
            }
        }
    }
    .into()
}

/// Builds a theme from a css file validated against the theme contract.
#[proc_macro]
pub fn theme(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);
    expand_theme(path)
        .unwrap_or_else(|err| err.into_compile_error())
        .into()
}

fn expand_theme(path: LitStr) -> syn::Result<proc_macro2::TokenStream> {
    let relative = path.value();
    let rules = parse_rules(THEME_DEF, "slots_theme_def.css")?;
    let id = Path::new(&relative)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("theme");
    let css_path = if Path::new(&relative).is_absolute() {
        quote!(#path)
    } else {
        quote!(::core::concat!(
            ::core::env!("CARGO_MANIFEST_DIR"),
            "/",
            #path
        ))
    };
    let assertions = rules.iter().map(|rule| {
        let selector = LitStr::new(&rule.selector, path.span());
        let message = LitStr::new(
            &format!(
                "{relative} is missing {} (slots_theme_def.css line {})",
                rule.selector, rule.line
            ),
            path.span(),
        );
        quote! {
            const _: () = ::core::assert!(
                ::bevy_dioxus_render::slots::css_has_rule(__THEME_CSS, #selector),
                #message
            );
        }
    });
    let id = LitStr::new(id, path.span());
    Ok(quote! {
        {
            const __THEME_CSS: &str = ::core::include_str!(#css_path);
            #( #assertions )*
            ::bevy_dioxus_render::slots::Theme::new(#id, __THEME_CSS)
        }
    })
}

#[derive(Clone, Debug)]
struct Rule {
    selector: String,
    line: usize,
}

fn parse_rules(css: &str, label: &str) -> syn::Result<Vec<Rule>> {
    let css = strip_comments(css);
    let bytes = css.as_bytes();
    let mut rules = Vec::new();
    let mut index = 0;
    let mut selector_start = 0;
    let mut in_block = false;

    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' => {
                let quote = bytes[index];
                index += 1;
                while index < bytes.len() {
                    if bytes[index] == b'\\' {
                        index += 2;
                        continue;
                    }
                    if bytes[index] == quote {
                        break;
                    }
                    index += 1;
                }
            }
            b'{' => {
                if in_block {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("{label} has nested css blocks; themes must use flat selectors"),
                    ));
                }
                let raw = css[selector_start..index].trim();
                if raw.starts_with('@') {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        format!("{label} uses {raw}; the theme contract must use flat selectors"),
                    ));
                }
                let pending = &css[selector_start..index];
                let leading = pending.len() - pending.trim_start().len();
                let line = 1 + css[..selector_start + leading].matches('\n').count();
                for selector in raw.split(',') {
                    let selector = normalize_selector(selector);
                    if !selector.is_empty() {
                        rules.push(Rule { selector, line });
                    }
                }
                in_block = true;
            }
            b'}' => {
                in_block = false;
                selector_start = index + 1;
            }
            b';' if !in_block => {
                selector_start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }

    if in_block {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("{label} has an unclosed css block"),
        ));
    }
    Ok(rules)
}

/// Replaces comments with whitespace while preserving line numbers.
fn strip_comments(css: &str) -> String {
    let bytes = css.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(open) = quote {
            out.push(byte);
            if byte == b'\\' && index + 1 < bytes.len() {
                out.push(bytes[index + 1]);
                index += 2;
                continue;
            }
            if byte == open {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'"' || byte == b'\'' {
            quote = Some(byte);
            out.push(byte);
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                if bytes[index] == b'\n' {
                    out.push(b'\n');
                }
                index += 1;
            }
            index += 2;
            continue;
        }
        out.push(byte);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn normalize_selector(selector: &str) -> String {
    let mut normalized = selector.split_whitespace().collect::<Vec<_>>().join(" ");
    for (from, to) in [(" > ", ">"), (" + ", "+"), (" ~ ", "~")] {
        normalized = normalized.replace(from, to);
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEF: &str = ".a {}\n.b {}\n.b:hover {}\n";

    #[test]
    fn def_selectors_report_lines() {
        let rules = parse_rules(DEF, "def").unwrap();
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[1].selector, ".b");
        assert_eq!(rules[1].line, 2);
        assert_eq!(rules[2].selector, ".b:hover");
        assert_eq!(rules[2].line, 3);
    }

    #[test]
    fn grouped_selectors_are_split() {
        let rules = parse_rules(".a, .b:hover {}\n", "def").unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].selector, ".a");
        assert_eq!(rules[1].selector, ".b:hover");
    }

    #[test]
    fn comments_are_ignored() {
        let rules = parse_rules("/* .a {} */\n.b {}\n", "def").unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].selector, ".b");
        assert_eq!(rules[0].line, 2);
    }

    #[test]
    fn at_rules_are_rejected() {
        let err = parse_rules("@media screen {}\n", "def").unwrap_err();
        assert!(err.to_string().contains("@media"));
    }

    #[test]
    fn nested_blocks_are_rejected() {
        let err = parse_rules(".a { .b {} }\n", "def").unwrap_err();
        assert!(err.to_string().contains("nested"));
    }

    #[test]
    fn braces_in_strings_are_ignored() {
        let rules = parse_rules(".a { content: \"{\"; }\n.b {}\n", "def").unwrap();
        assert_eq!(rules.len(), 2);
    }
}
