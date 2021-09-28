use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, Result};

#[proc_macro_attribute]
pub fn sorted(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let original = input.clone();

    let mut original = TokenStream::from(original);
    let input = parse_macro_input!(input as syn::Item);

    let x = check_type(&input);
    if x.is_err() {
        original.extend(x.unwrap_err().to_compile_error());
        return original.into();
    }

    let e = x.unwrap();
    if let Err(err) = check_enum_order(e) {
        original.extend(err.to_compile_error());
        return original.into();
    }

    original.into()
}

fn check_enum_order(item: &syn::ItemEnum) -> Result<()> {
    let mut names = Vec::new();

    for v in item.variants.iter() {
        let name = v.ident.to_string();
        if names.last().map(|v| &name < v).unwrap_or(false) {
            let pos = names.binary_search(&name).unwrap_err();
            let err = syn::Error::new_spanned(
                &v.ident,
                format!("{} should sort before {}", name, names[pos]),
            );
            return Err(err);
        }
        names.push(name);
    }

    Ok(())
}

fn check_type(item: &syn::Item) -> Result<&syn::ItemEnum> {
    match item {
        syn::Item::Enum(e) => Ok(e),
        _ => {
            let err = syn::Error::new(
                proc_macro2::Span::call_site(),
                "expected enum or match expression",
            );
            Err(err)
        }
    }
}

struct ExprMatchReplace {
    errs: Vec<syn::Error>,
}

impl VisitMut for ExprMatchReplace {
    fn visit_expr_match_mut(&mut self, expr: &mut syn::ExprMatch) {
        if expr.attrs.is_empty() {
            return;
        }

        let mut vec = Vec::new();
        std::mem::swap(&mut vec, &mut expr.attrs);

        let mut has_sorted = false;
        for attr in vec.into_iter() {
            if !has_sorted_attr(&attr) {
                expr.attrs.push(attr);
            } else {
                has_sorted = true
            }
        }

        if !has_sorted {
            return;
        }

        // check sorted
        let mut names = Vec::new();
        let mut no_more = false;
        for arm in expr.arms.iter() {
            let name = extart_arm_ident(&arm.pat);
            if name.is_err() {
                let err = syn::Error::new(name.unwrap_err(), "unsupported by #[sorted]");
                self.errs.push(err);
                break;
            }

            if no_more {
                let err = syn::Error::new(name.unwrap_err(), "_ should be the last one");
                self.errs.push(err);
                break;
            }

            let (name, span) = name.unwrap();
            if name == "_" {
                no_more = true;
                continue
            }
            if names.last().map(|v| &name < v).unwrap_or(false) {
                let pos = names.binary_search(&name).unwrap_err();
                let err =
                    syn::Error::new(span, format!("{} should sort before {}", name, names[pos]));
                self.errs.push(err);
                break;
            }
            names.push(name);
        }
    }
}

fn join_path_names(path: &syn::Path) -> String {
    let names: Vec<_> = path
        .segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect();

    return names.join("::");
}

fn extart_arm_ident(
    pat: &syn::Pat,
) -> std::result::Result<(String, proc_macro2::Span), proc_macro2::Span> {
    match pat {
        syn::Pat::Ident(ident) => Ok((ident.ident.to_string(), ident.span())),
        syn::Pat::TupleStruct(syn::PatTupleStruct { ref path, .. }) => {
            Ok((join_path_names(path), path.span()))
        }
        syn::Pat::Path(syn::PatPath { ref path, .. }) => Ok((join_path_names(path), path.span())),
        syn::Pat::Wild(v) => Ok(("_".to_string(), v.span())),
        _ => Err(pat.span()),
    }
}

fn has_sorted_attr(attr: &syn::Attribute) -> bool {
    let meta = attr.parse_meta().unwrap();
    if let syn::Meta::Path(ref path) = meta {
        return path.segments.len() == 1 && path.segments[0].ident == "sorted";
    }
    false
}

#[proc_macro_attribute]
pub fn check(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut f = parse_macro_input!(input as syn::ItemFn);
    let mut v = ExprMatchReplace { errs: Vec::new() };
    v.visit_item_fn_mut(&mut f);

    let mut output = quote::quote! { #f };
    output.extend(v.errs.iter().map(|err| err.to_compile_error()));
    output.into()
}
