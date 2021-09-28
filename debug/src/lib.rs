use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

fn get_custom_fmt(f: &syn::Field) -> Option<syn::Lit> {
    for attr in f.attrs.iter() {
        let meta = attr.parse_meta().unwrap();
        if let syn::Meta::NameValue(ref nv) = meta {
            return Some(nv.lit.clone());
        }
    }

    None
}

fn parse_bound_from_attrs(attrs: &Vec<syn::Attribute>) -> Option<syn::LitStr> {
    for attr in attrs {
        let meta = attr.parse_meta().unwrap();

        if let syn::Meta::List(syn::MetaList { ref nested, .. }) = meta {
            for nv in nested.iter() {
                if let syn::NestedMeta::Meta(syn::Meta::NameValue(ref nv)) = nv {
                    let name = nv.path.to_token_stream().to_string();
                    if name != "bound" {
                        continue;
                    }
                    if let syn::Lit::Str(ref lit) = nv.lit {
                        return Some(lit.clone());
                    }
                }
            }
        }
    }
    None
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let ident = &input.ident;
    let data = &input.data;

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = data
    {
        named
    } else {
        unreachable!();
    };

    let fmt_fields: Vec<_> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            if let Some(custom_fmt) = get_custom_fmt(f) {
                quote! { field(stringify!(#ident), &format_args!(#custom_fmt, &self.#ident)) }
            } else {
                quote! { field(stringify!(#ident), &self.#ident) }
            }
        })
        .collect();

    let output = if !input.generics.params.is_empty() {
        let debug_constrains = if let Some(bound) = parse_bound_from_attrs(&input.attrs) {
            let x: TokenStream = syn::parse_str(bound.value().as_str()).unwrap();
            x
        } else {
            gen_type_constrains2(&input.generics, fields.iter())
        };

        let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
        let where_clause = if let Some(c) = where_clause {
            quote! { #c #debug_constrains }
        } else {
            quote! { where #debug_constrains }
        };
        quote! {
            impl #impl_generics std::fmt::Debug for #ident #type_generics #where_clause {
                fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                    fmt.debug_struct(stringify!(#ident))
                        #(.#fmt_fields)*
                        .finish()
                }
            }
        }
    } else {
        quote! {
            impl std::fmt::Debug for #ident {
                fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                    fmt.debug_struct(stringify!(#ident))
                        #(.#fmt_fields)*
                        .finish()
                }
            }
        }
    };

    // println!("{}\n", output.to_string());
    output.into()
}

fn unwrap_type<'a>(ty: &'a syn::Type, wrapper: &'_ str) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath { ref path, .. }) = ty {
        if path.segments.is_empty() {
            return None;
        }
        let ty = path.segments.last().unwrap();
        if ty.ident != wrapper {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            ref args,
            ..
        }) = ty.arguments
        {
            if args.len() != 1 {
                return None;
            }

            if let syn::GenericArgument::Type(ref ty) = args[0] {
                return Some(ty);
            }
        }
    };
    None
}

fn gen_type_constrains2<'a>(
    generics: &syn::Generics,
    fields: impl Iterator<Item = &'a syn::Field>,
) -> TokenStream {
    let mut generic_idents = Vec::new();
    for generic in generics.params.iter() {
        if let syn::GenericParam::Type(t) = generic {
            let ident = &t.ident;
            generic_idents.push(ident);
        }
    }

    let mut pathes = Vec::new();
    let mut added = Vec::new();
    for f in fields {
        // println!("==== process field: {:?}", f.ident);
        // skip field: PhantomData<ANY>
        if let Some(_) = unwrap_type(&f.ty, "PhantomData") {
            continue;
        }

        if let syn::Type::Path(syn::TypePath { ref path, .. }) = f.ty {
            // println!("==========process path: {}", quote! { #path }.to_string());
            search_path(path, &generic_idents, &mut pathes, &mut added);
        }
    }

    /*
    for p in pathes.iter() {
    println!("===bound trait: {:?}", p.to_string())
    }
    */

    let constrains = quote! {
        #(#pathes: std::fmt::Debug)*
    };

    constrains.into()
}

fn search_path(
    path: &syn::Path,
    generics: &Vec<&syn::Ident>,
    bounds: &mut Vec<TokenStream>,
    added: &mut Vec<syn::Ident>,
) {
    if path.segments.is_empty() {
        return;
    }

    let ident = &path.segments[0].ident;
    if generics.binary_search(&ident).is_ok() {
        if added.binary_search(ident).is_err() {
            bounds.push(quote! { #path });
            added.push(ident.clone());
        }
        return;
    }

    for segment in path.segments.iter() {
        if segment.arguments.is_empty() {
            continue;
        }

        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            ref args,
            ..
        }) = segment.arguments
        {
            for arg in args.iter() {
                if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath { path, .. })) = arg
                {
                    search_path(path, generics, bounds, added);
                }
            }
        }
    }
}
