use proc_macro::TokenStream;

use syn::parse::{Parse, ParseStream};

struct SeqInput {
    ident: syn::Ident,
    from: usize,
    to: usize,
    ts: proc_macro2::TokenStream,
}

impl Parse for SeqInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = syn::Ident::parse(input)?;
        let _in = <syn::Token![in]>::parse(input)?;

        // range
        let mut plus = 0;
        let from = syn::LitInt::parse(input)?;
        let _dot_eq = <syn::Token![..=]>::parse(input);
        if _dot_eq.is_err() {
            let _dot = <syn::Token![..]>::parse(input);
        } else {
            plus = 1;
        }
        let to = syn::LitInt::parse(input)?;

        let content;
        let _brace_token = syn::braced!(content in input);

        let ts = proc_macro2::TokenStream::parse(&content)?;
        // println!("{:?}", ts);
        Ok(SeqInput {
            ident,
            from: from.base10_parse::<usize>().unwrap(),
            to: to.base10_parse::<usize>().unwrap() + plus,
            ts,
        })
    }
}

fn push_to_stream<T: Into<proc_macro2::TokenTree>>(ts: &mut proc_macro2::TokenStream, token: T) {
    let token = token.into();
    let token_tree: proc_macro2::TokenTree = token;
    ts.extend(std::iter::once(token_tree));
}

fn get_repeat_section(ts: &BufferredTokenStream, pos: usize) -> Option<proc_macro2::Group> {
    match (ts.peek(pos, 1), ts.peek(pos, 2)) {
        (Some(proc_macro2::TokenTree::Group(group)), Some(proc_macro2::TokenTree::Punct(star)))
            if group.delimiter() == proc_macro2::Delimiter::Parenthesis
                && star.as_char() == '*' =>
        {
            Some(group)
        }
        _ => None,
    }
}

impl SeqInput {
    fn expand(&self, ts: proc_macro2::TokenStream, i: usize) -> proc_macro2::TokenStream {
        use proc_macro2::TokenTree;
        let ts = BufferredTokenStream::new(ts);
        let mut output = proc_macro2::TokenStream::new();
        let mut skip = 0;
        for (pos, item) in ts.tokens.iter().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }

            match item {
                TokenTree::Group(group) => {
                    let delimiter = group.delimiter();
                    let inner_stream = group.stream();
                    let expaned = self.expand(inner_stream, i);

                    let mut new_group = proc_macro2::Group::new(delimiter, expaned);
                    new_group.set_span(group.span());
                    push_to_stream(&mut output, new_group);
                }
                TokenTree::Ident(ident) => {
                    let ident = ident.clone();
                    if ident == self.ident {
                        let mut lit = proc_macro2::Literal::usize_unsuffixed(i);
                        lit.set_span(ident.span());
                        push_to_stream(&mut output, lit);
                    } else {
                        // IDENT #
                        match ts.peek(pos, 1) {
                            Some(TokenTree::Punct(v)) if v.as_char() == '#' => {}
                            _ => {
                                push_to_stream(&mut output, ident);
                                continue;
                            }
                        };

                        // IDENT # N
                        match ts.peek(pos, 2) {
                            Some(TokenTree::Ident(v)) if v == self.ident => {}
                            _ => {
                                push_to_stream(&mut output, ident);
                                continue;
                            }
                        };

                        //    IDENT # N #
                        // or IDENT # N x
                        match ts.peek(pos, 3) {
                            Some(TokenTree::Punct(v)) if v.as_char() == '#' => {}
                            _ => {
                                let mut merge_ident = quote::format_ident!("{}{}", ident, i);
                                merge_ident.set_span(ident.span());
                                push_to_stream(&mut output, merge_ident);
                                skip = 2;
                                continue;
                            }
                        };

                        //    IDENT # N # IDENT
                        // or IDENT # N # ?
                        match ts.peek(pos, 4) {
                            Some(TokenTree::Ident(ident2)) => {
                                let mut merge_ident =
                                    quote::format_ident!("{}{}{}", ident, i, ident2);
                                merge_ident.set_span(ident.span());
                                push_to_stream(&mut output, merge_ident);
                                skip = 4;
                                continue
                            }
                            _ => {
                            }
                        };
                        push_to_stream(&mut output, ident);
                    }
                }
                TokenTree::Punct(punc) => {
                    push_to_stream(&mut output, punc.clone());
                }
                TokenTree::Literal(lit) => {
                    push_to_stream(&mut output, lit.clone());
                }
            }
        }

        output
    }

    fn check_repeat(&self, ts: proc_macro2::TokenStream) -> bool {
        use proc_macro2::TokenTree;

        let ts = BufferredTokenStream::new(ts);
        for (pos, item) in ts.tokens.iter().enumerate() {
            match item {
                TokenTree::Group(group) => {
                    if self.check_repeat(group.stream()) {
                        return true;
                    }
                }
                TokenTree::Punct(punct) => {
                    if punct.as_char() != '#' {
                        continue;
                    }

                    if get_repeat_section(&ts, pos).is_some() {
                        return true;
                    }
                }
                _ => {}
            };
        }
        return false;
    }

    fn expand_repeat(&self, ts: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        use proc_macro2::TokenTree;

        let mut output = proc_macro2::TokenStream::new();
        let ts = BufferredTokenStream::new(ts);
        let mut skip = 0;
        for (pos, item) in ts.tokens.iter().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }

            match item {
                TokenTree::Group(group) => {
                    let expanded = self.expand_repeat(group.stream());
                    let mut new_group = proc_macro2::Group::new(group.delimiter(), expanded);
                    new_group.set_span(group.span());
                    push_to_stream(&mut output, new_group);
                }
                TokenTree::Punct(punct) => {
                    if punct.as_char() != '#' {
                        push_to_stream(&mut output, punct.clone());
                        continue;
                    }

                    if let Some(group) = get_repeat_section(&ts, pos) {
                        let inner_stream = group.stream();
                        for i in self.from..self.to {
                            let expanded = self.expand(inner_stream.clone(), i);
                            // println!("== repeat group: {}\n", expanded.to_string());
                            output.extend(expanded);
                        }
                        skip = 2;
                    } else {
                        push_to_stream(&mut output, punct.clone());
                    }
                }
                TokenTree::Literal(lit) => {
                    push_to_stream(&mut output, lit.clone());
                }
                TokenTree::Ident(ident) => {
                    push_to_stream(&mut output, ident.clone());
                }
            };
        }
        output
    }
}

struct BufferredTokenStream {
    tokens: Vec<proc_macro2::TokenTree>,
}

impl BufferredTokenStream {
    fn new(ts: proc_macro2::TokenStream) -> Self {
        let mut tokens = Vec::new();
        tokens.extend(ts.into_iter());
        Self { tokens }
    }

    fn peek(&self, cur_pos: usize, next_n: usize) -> Option<proc_macro2::TokenTree> {
        let pos = cur_pos + next_n;
        if pos >= self.tokens.len() {
            None
        } else {
            Some(self.tokens[pos].clone())
        }
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let data = syn::parse_macro_input!(input as SeqInput);
    let mut ts = proc_macro2::TokenStream::new();
    if data.check_repeat(data.ts.clone()) {
        ts = data.expand_repeat(data.ts.clone());
    } else {
        for i in data.from..data.to {
            ts.extend(data.expand(data.ts.clone(), i));
        }
    }
    ts.into()
}
