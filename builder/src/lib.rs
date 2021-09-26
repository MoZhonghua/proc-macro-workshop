use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

fn get_named_values_from_attr(f: &syn::Field) -> Vec<(syn::MetaList, syn::Ident, syn::Lit)> {
    let mut named_vaules = Vec::new();

    for attr in &f.attrs {
        if attr.path.segments.len() != 1 {
            continue;
        }

        let attr_ident = &attr.path.segments[0].ident;
        if attr_ident != "builder" {
            continue;
        }

        // let meta = syn::Meta::parse(&attr.tokens).unwrap();
        let meta = attr.parse_meta().unwrap();

        if let syn::Meta::List(ref meta_list) = meta {
            for n in &meta_list.nested {
                if let syn::NestedMeta::Meta(syn::Meta::NameValue(ref nv)) = n {
                    let name = nv.path.segments[0].ident.clone();
                    let val = nv.lit.clone();
                    println!("attr: {:?} {:?}={:?}", attr_ident, name, val);
                    named_vaules.push((meta_list.clone(), name, val));
                }
            }
        }
    }
    named_vaules
}

fn get_value_of_each(f: &syn::Field) -> Option<String> {
    for (_, name, value) in get_named_values_from_attr(f) {
        if name == "each" {
            if let syn::Lit::Str(ref s) = value {
                return Some(s.value());
            }
        }
    }
    None
}

fn check_attribute(f: &syn::Field) -> Option<syn::Error> {
    for (meta_list, name, _) in get_named_values_from_attr(f) {
        if name != "each" {
            /*
            let mut tokens = (&attr.path).into_token_stream();
            let tokens2 = (&attr.tokens).into_token_stream();
            tokens.extend(tokens2.into_iter());
            */
            return Some(syn::Error::new_spanned(
                &meta_list,
                "expected `builder(each = \"...\")`",
            ));
        }
    }
    None
}

struct FieldInfo<'a> {
    field: &'a syn::Field,
    option_inner_type: Option<&'a syn::Type>,

    each_func: Option<syn::Ident>,
    vec_inner_type: Option<&'a syn::Type>,
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = data
    {
        named
    } else {
        unimplemented!()
    };

    for f in fields {
        if let Some(err) = check_attribute(f) {
            return err.into_compile_error().into();
        }
    }

    let fields_info = fields
        .iter()
        .map(|f| {
            let mut info = FieldInfo {
                field: f,
                option_inner_type: None,
                each_func: None,
                vec_inner_type: None,
            };

            if let Some(inner_type) = unwrap_ty(&f.ty, "Option") {
                info.option_inner_type = Some(inner_type);
            } else {
                if let Some(each_func) = get_value_of_each(f) {
                    if let Some(inner_type) = unwrap_ty(&f.ty, "Vec") {
                        info.each_func = Some(quote::format_ident!("{}", each_func));
                        info.vec_inner_type = Some(inner_type);
                    }
                }
            }

            info
        })
        .collect::<Vec<_>>();

    let builder_ident = quote::format_ident!("{}Builder", ident);

    let builder_fields = fields_info.iter().map(|f| {
        let ty = &f.field.ty;
        let ident = &f.field.ident;

        if f.option_inner_type.is_some() || f.vec_inner_type.is_some() {
            quote! {
                #ident: #ty
            }
        } else {
            quote! {
                #ident: ::std::option::Option::<#ty>
            }
        }
    });

    let builder_methods = fields_info.iter().map(|f| {
        let ty = &f.field.ty;
        let ident = &f.field.ident;
        if let Some(inner_type) = f.option_inner_type {
            quote! {
                fn #ident(&mut self, #ident: #inner_type) -> &mut Self {
                    self.#ident = ::std::option::Option::Some(#ident);
                    self
                }
            }
        } else {
            if let Some(inner_type) = f.vec_inner_type {
                let name = f.each_func.as_ref().unwrap();
                quote! {
                    fn #name(&mut self, #ident: #inner_type) -> &mut Self {
                        self.#ident.push(#ident);
                        self
                    }
                }
            } else {
                quote! {
                    fn #ident(&mut self, #ident: #ty) -> &mut Self {
                        self.#ident = ::std::option::Option::Some(#ident);
                        self
                    }
                }
            }
        }
    });

    let builder_none = fields_info.iter().map(|f| {
        let ident = &f.field.ident;
        if f.each_func.is_some() {
            quote! {
                #ident: ::std::vec::Vec::new()
            }
        } else {
            quote! {
                #ident: ::std::option::Option::None
            }
        }
    });

    let return_fields = fields_info.iter().map(|f| {
        let ident = &f.field.ident;
        if f.option_inner_type.is_some() || f.each_func.is_some() {
            quote! {
                #ident: self.#ident.clone()
            }
        } else {
            quote! {
                #ident: self.#ident.clone().ok_or(concat!(stringify!(#ident), " is not set"))?
            }
        }
    });

    let output = quote! {
        pub struct #builder_ident {
            #(#builder_fields,)*
        }

        impl #builder_ident {
            #(#builder_methods)*

            fn build(&mut self) -> std::result::Result<Command, ::std::boxed::Box<dyn std::error::Error>> {
                ::std::result::Result::Ok(#ident {
                    #(#return_fields),*
                })

            }
        }
        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#builder_none,)*
                }
            }
        }
    };

    let x: TokenStream = output.into();
    // println!("{}", x.to_string());
    x
}

fn unwrap_ty<'a>(ty: &'a syn::Type, name: &'_ str) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { ref segments, .. },
        ..
    }) = ty
    {
        if segments.len() != 1 || segments[0].ident != name {
            return None;
        }

        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            ref args,
            ..
        }) = segments[0].arguments
        {
            if args.len() != 1 {
                panic!("not valid Option type");
            }

            if let syn::GenericArgument::Type(ref ty) = args[0] {
                return Some(ty);
            }
        }
    }
    None
}
