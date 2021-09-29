use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, spanned::Spanned};

#[proc_macro_derive(BitfieldSpecifier, attributes(bits))]
pub fn bitfield_specifier(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse_macro_input!(input as syn::DeriveInput);

    let enum_def = if let syn::Data::Enum(ref enum_def) = input.data {
        enum_def
    } else {
        unreachable!()
    };

    let max_val = enum_def.variants.len() as u64 - 1;
    let bits = bits_to_hold_val(max_val);

    if (1 << bits) - 1 != max_val {
        let err = syn::Error::new(
            proc_macro2::Span::call_site(),
            "BitfieldSpecifier expected a number of variants which is a power of 2",
        );
        return err.to_compile_error().into();
    }

    let enum_ident = &input.ident;

    let mut get_dis = Vec::new();
    let mut from_match = Vec::new();
    let mut check_range = Vec::new();
    for (i, v) in enum_def.variants.iter().enumerate() {
        let ident = &v.ident;
        let ident_dis = quote::format_ident!("_{}", &v.ident.to_string().to_uppercase());
        get_dis.push(quote! {  const #ident_dis: u64 = #enum_ident::#ident as u64 });
        from_match.push(quote! {  #ident_dis => #enum_ident::#ident });

        let check_ident = quote::format_ident!("_CHECK_VAR_{}", i);

        let span = v.span();
        let check_stmt = quote::quote_spanned! { span=>
            const #check_ident: bool = (#enum_ident::#ident as u64) <= #max_val;
            let _: bitfield::checks::Check2<
                <bitfield::checks::Selector2<#check_ident> as bitfield::checks::TypeSelector>::Type
                >;
        };

        check_range.push(check_stmt);
    }

    let ident = &input.ident;
    let output = quote! {
        impl bitfield::Specifier for #ident {
            const BITS: usize = #bits;
            type DataType = Self;
            fn from_u64(v: u64) -> Self::DataType {
                #(#get_dis;)*
                match v {
                    #(#from_match,)*
                    _ => unreachable!(),
                }
            }
            fn to_u64(v: Self::DataType) -> u64 {
                /// t.compile_fail("tests/09-variant-out-of-range.rs");
                #(#check_range)*
                v as u64
            }
        }
    };

    // println!("{}", output.to_string());
    output.into()
}

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    // println!("{:#?}", input);
    // println!("{:#?}", args);
    let _ = args;

    let item = parse_macro_input!(input as syn::ItemStruct);
    let fields = if let syn::Fields::Named(syn::FieldsNamed { ref named, .. }) = item.fields {
        named
    } else {
        println!("fields is not named");
        unreachable!();
    };

    let fields = collect_fields_info(fields.iter()).unwrap();

    let mut bits_acc = Vec::new();
    bits_acc.push(quote! { 0 });
    let mut getters = Vec::new();
    let mut setters = Vec::new();
    let mut bits_checks = Vec::new();
    for (i, f) in fields.iter().enumerate() {
        let ty = &f.field.ty;

        let specifier_type = quote! { <#ty as bitfield::Specifier> };
        let bits = quote! { #specifier_type::BITS};
        let set_ident = quote::format_ident!("set_{}", &f.name);
        let get_ident = quote::format_ident!("get_{}", &f.name);

        getters.push(quote! {
            fn #get_ident(&self) -> #specifier_type::DataType {
                let val = bitfield::read_bits(&self.data[..], #(#bits_acc)+*, #bits);
                #specifier_type::from_u64(val)
            }
        });

        setters.push(quote! {
            fn #set_ident(&mut self, val: #specifier_type::DataType) {
                let val = #specifier_type::to_u64(val);
                bitfield::write_bits(&mut self.data[..], #(#bits_acc)+*, #bits, val)
            }
        });

        if let Some((attr_bits, lit)) = &f.bits_in_attr {
            let ident_actual = quote::format_ident!("_C_ACTUAL_{}", i);
            let ident_attr = quote::format_ident!("_C_ATTR_{}", i);
            let span = lit.span();
            let check = quote::quote_spanned! { span=>
                const #ident_actual: usize = #specifier_type::BITS;
                const #ident_attr: usize = #attr_bits;

                let _: std::marker::PhantomData::<[u8;#ident_attr]> = std::marker::PhantomData::<[u8;#ident_actual]>;
            };
            bits_checks.push(check);
        }

        bits_acc.push(bits);
    }

    let ident = &item.ident;
    let vis = &item.vis;
    let output = quote! {
        #vis struct #ident {
            data: [u8; (#(#bits_acc)+*) /8],
        }

        impl #ident {
            fn new() -> Self {
                /// t.compile_fail("tests/04-multiple-of-8bits.rs");
                let _: bitfield::checks::Check<
                    <bitfield::checks::Selector<[u8;(#(#bits_acc)+*) % 8]> as bitfield::checks::TypeSelector>::Type
                    >;

                /// t.compile_fail("tests/11-bits-attribute-wrong.rs");
                #(#bits_checks)*

                Self {
                    data: [0_u8;(#(#bits_acc)+*)/8],
                }
            }
            #(#getters)*
            #(#setters)*
        }
    };

    // println!("{}", output.to_string());
    output.into()
}

struct FieldInfo<'a> {
    field: &'a syn::Field,
    name: String,
    bits_in_attr: Option<(usize, syn::Lit)>,
}

fn get_bits_in_attr(attrs: &Vec<syn::Attribute>) -> Option<(usize, syn::Lit)> {
    for attr in attrs {
        let meta = attr.parse_meta().unwrap();

        let nv = if let syn::Meta::NameValue(ref nv) = meta {
            nv
        } else {
            unreachable!();
        };

        if nv.path.segments[0].ident != "bits" {
            continue;
        }

        if let syn::Lit::Int(ref lit) = nv.lit {
            let bits: usize = lit.base10_digits().parse().unwrap();
            return Some((bits, nv.lit.clone()));
        } else {
            continue;
        }
    }

    None
}

fn collect_fields_info<'a>(
    fields: impl Iterator<Item = &'a syn::Field>,
) -> syn::Result<Vec<FieldInfo<'a>>> {
    let mut r = Vec::new();
    for f in fields {
        if f.ident.is_none() {
            return Err(syn::Error::new_spanned(&f, "field name is empty"));
        }

        let name = f.ident.as_ref().unwrap().to_string();

        let ty_name = if let syn::Type::Path(syn::TypePath { ref path, .. }) = f.ty {
            path_name(path)
        } else {
            "".to_string()
        };

        if name.is_empty() || ty_name.is_empty() {
            return Err(syn::Error::new_spanned(&f, "invalid field"));
        }

        r.push(FieldInfo {
            field: f,
            name,
            bits_in_attr: get_bits_in_attr(&f.attrs),
        });
    }

    Ok(r)
}

fn path_name(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn bits_to_hold_val(mut v: u64) -> usize {
    let mut bits = 0;

    while v > 0 {
        bits += 1;
        v = v >> 1;
    }
    bits
}
