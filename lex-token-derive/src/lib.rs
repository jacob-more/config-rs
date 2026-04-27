use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Visibility, spanned::Spanned};

use crate::parse::{LexEnum, LexPatterns};

mod parse;

#[proc_macro]
pub fn lex(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let lex = syn::parse_macro_input!(input as LexEnum);
    let generate_token_structs = generate_token_structs(&lex);
    let generate_token_enum = generate_token_enum(&lex);
    let generate_tokenizer = generate_tokenizer(&lex);
    let generate_token_iter = generate_token_iter(&lex);
    quote! {
        #generate_token_structs
        #generate_token_enum
        #generate_tokenizer
        #generate_token_iter
    }
    .into()
}

fn generate_token_structs(lex: &LexEnum) -> proc_macro2::TokenStream {
    let vis = &lex.vis;
    let ident = &lex.ident;
    let token_idents = lex.variants.iter().map(|var| (var.token_ident(ident), var));
    let token_ident = token_idents
        .clone()
        .map(|(ident, _)| ident)
        .collect::<Vec<_>>();
    let non_capturing_token_ident = token_idents
        .clone()
        .filter(|(_, var)| !var.patterns.is_capturing())
        .map(|(ident, _)| ident)
        .collect::<Vec<_>>();
    let capturing_token_ident = token_idents
        .clone()
        .filter(|(_, var)| var.patterns.is_capturing())
        .map(|(ident, _)| ident)
        .collect::<Vec<_>>();

    fn generate_capturing_token_struct(vis: &Visibility, token_ident: &[Ident]) -> TokenStream {
        quote! {
            #(
                #vis struct #token_ident<'a> {
                    pub(crate) buffer: &'a ::bytes::Bytes,
                    pub(crate) captures: regex::bytes::Captures<'a>,
                }

                impl<'a> #token_ident<'a> {
                    pub fn as_slice(&self) -> &'a [u8] {
                        let matched = self.captures.get_match();
                        &self.buffer[matched.start()..matched.end()]
                    }

                    pub fn as_bytes(&self) -> ::bytes::Bytes {
                        let matched = self.captures.get_match();
                        self.buffer.slice(matched.start()..matched.end())
                    }

                    pub fn span(&self) -> Span {
                        let matched = self.captures.get_match();
                        let start = get_pos(&self.buffer[..matched.start()]);
                        let mut end = get_pos(&self.buffer[matched.start()..matched.end()]);
                        end.line += start.line;
                        if start.line == end.line {
                            end.column += start.column;
                        }
                        Span { start, end }
                    }

                    pub fn start(&self) -> Pos {
                        get_pos(&self.buffer[..self.captures.get_match().start()])
                    }

                    pub fn end(&self) -> Pos {
                        get_pos(&self.buffer[..self.captures.get_match().end()])
                    }
                }
            )*
        }
    }

    fn generate_non_capturing_token_struct(vis: &Visibility, token_ident: &[Ident]) -> TokenStream {
        quote! {
            #(
                #vis struct #token_ident<'a> {
                    pub(crate) buffer: &'a ::bytes::Bytes,
                    pub(crate) start: usize,
                    pub(crate) end: usize,
                }

                impl<'a> #token_ident<'a> {
                    pub fn as_slice(&self) -> &'a [u8] {
                        &self.buffer[self.start..self.end]
                    }

                    pub fn as_bytes(&self) -> ::bytes::Bytes {
                        self.buffer.slice(self.start..self.end)
                    }

                    pub fn span(&self) -> Span {
                        let start = get_pos(&self.buffer[..self.start]);
                        let mut end = get_pos(&self.buffer[self.start..self.end]);
                        end.line += start.line;
                        if start.line == end.line {
                            end.column += start.column;
                        }
                        Span { start, end }
                    }

                    pub fn start(&self) -> Pos {
                        get_pos(&self.buffer[..self.start])
                    }

                    pub fn end(&self) -> Pos {
                        get_pos(&self.buffer[..self.end])
                    }
                }
            )*
        }
    }

    let capturing_tokens = generate_capturing_token_struct(vis, &capturing_token_ident);
    let non_capturing_tokens = generate_non_capturing_token_struct(vis, &non_capturing_token_ident);

    quote! {
        #capturing_tokens
        #non_capturing_tokens

        #(
            impl<'a> ::std::fmt::Debug for #token_ident<'a> {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!(#token_ident))
                        .field("buffer", &self.as_bytes())
                        .field("span", &self.span())
                        .finish()
                }
            }
        )*
    }
}

fn generate_token_enum(lex: &LexEnum) -> proc_macro2::TokenStream {
    let vis = &lex.vis;
    let ident = &lex.ident;
    let case_ident = lex
        .variants
        .iter()
        .map(|var| &var.ident)
        .collect::<Vec<_>>();
    let token_ident = lex.variants.iter().map(|var| var.token_ident(ident));

    quote! {
        #vis enum #ident<'a> {
            #( #case_ident(#token_ident<'a>), )*
        }

        impl<'a> ::std::fmt::Debug for #ident<'a> {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(self.ident())
                    .field("buffer", &self.as_bytes())
                    .field("span", &self.span())
                    .finish()
            }
        }

        impl<'a> #ident<'a> {
            pub(crate) fn ident(&self) -> &'static str {
                match self {
                    #( Self::#case_ident(_) => ::std::stringify!(#case_ident), )*
                }
            }

            pub fn as_slice(&self) -> &'a [u8] {
                match self {
                    #( Self::#case_ident(token) => token.as_slice(), )*
                }
            }

            pub fn as_bytes(&self) -> ::bytes::Bytes {
                match self {
                    #( Self::#case_ident(token) => token.as_bytes(), )*
                }
            }

            pub fn span(&self) -> Span {
                match self {
                    #( Self::#case_ident(token) => token.span(), )*
                }
            }

            pub fn start(&self) -> Pos {
                match self {
                    #( Self::#case_ident(token) => token.start(), )*
                }
            }

            pub fn end(&self) -> Pos {
                match self {
                    #( Self::#case_ident(token) => token.end(), )*
                }
            }
        }
    }
}

fn generate_tokenizer(lex: &LexEnum) -> proc_macro2::TokenStream {
    let vis = &lex.vis;
    let tokenizer_regex = lex.tokenizer_regex();
    let iter_ident = lex.iter_ident();

    quote! {
        #[derive(Debug, Clone)]
        #vis struct Tokenizer {
            pattern: ::regex::bytes::Regex
        }

        impl Tokenizer {
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl ::core::default::Default for Tokenizer {
            fn default() -> Self {
                static PATTERN: std::sync::LazyLock<::regex::bytes::Regex> =
                    std::sync::LazyLock::new(|| {
                        #tokenizer_regex
                    });
                Self {
                    pattern: ::std::clone::Clone::clone(&*PATTERN)
                }
            }
        }

        impl Tokenizer {
            pub fn tokenize<'r, 'h>(&'r self, buffer: &'h ::bytes::Bytes) -> TokenIter<'r, 'h> {
                #iter_ident::new(self, buffer)
            }
        }
    }
}

fn generate_token_iter(lex: &LexEnum) -> proc_macro2::TokenStream {
    let vis = &lex.vis;
    let ident = &lex.ident;
    let iter_ident = lex.iter_ident();

    let variant_catchall = match *lex
        .variants
        .iter()
        .filter(|var| matches!(var.patterns, LexPatterns::Any))
        .collect::<Vec<_>>()
        .as_slice()
    {
        [] => {
            return syn::Error::new(
                lex.enum_keyword.span(),
                "exhaustive pattern requires one catch-all case (`Ident = _`)",
            )
            .into_compile_error();
        }
        [token_unknown] => token_unknown,
        [first, second, ..] => return syn::Error::new(
            second.ident.span(),
            format!(
                "can have at most one catch-all case (`Ident = _`) but two were found: {} and {}",
                first.ident, second.ident
            ),
        )
        .into_compile_error(),
    };
    let case_ident_catchall = &variant_catchall.ident;
    let token_ident_catchall = variant_catchall.token_ident(ident);

    let mut index: usize = 1;
    let capture_variants = lex
        .variants
        .iter()
        .filter(|var| !matches!(var.patterns, LexPatterns::Any))
        .map(|var| {
            let case_ident = &var.ident;
            let token_ident = var.token_ident(ident);

            let tokens = if var.patterns.is_capturing() {
                quote! {
                    if captures.get(#index).is_some() {
                        return Some(#ident::#case_ident(#token_ident {
                            buffer: self.buffer,
                            captures,
                        }))
                    }
                }
            } else {
                quote! {
                    if captures.get(#index).is_some() {
                        return Some(#ident::#case_ident(#token_ident {
                            buffer: self.buffer,
                            start: matched.start(),
                            end: matched.end(),
                        }))
                    }
                }
            };
            index += 1 + var.patterns.extra_capture_count();
            tokens
        })
        .collect::<Vec<_>>();

    quote! {
        #[derive(Debug)]
        #vis struct #iter_ident<'r, 'h> {
            buffer: &'h ::bytes::Bytes,
            tokenizer: &'r Tokenizer,
            last_end: usize,
            captures: ::std::iter::Peekable<::regex::bytes::CaptureMatches<'r, 'h>>,
        }

        impl<'r, 'h> #iter_ident<'r, 'h> {
            pub fn new(tokenizer: &'r Tokenizer, buffer: &'h ::bytes::Bytes) -> Self {
                Self {
                    buffer,
                    tokenizer,
                    last_end: 0,
                    captures: tokenizer.pattern
                        .captures_iter(&**buffer)
                        .peekable(),
                }
            }
        }

        impl<'r, 'h> Iterator for #iter_ident<'r, 'h> {
            type Item = #ident<'h>;

            fn next(&mut self) -> Option<Self::Item> {
                let Some(captures) = self.captures.peek() else {
                    if self.last_end < self.buffer.len() {
                        std::hint::cold_path();
                        let result = Some(
                            #ident::#case_ident_catchall(
                                #token_ident_catchall {
                                    buffer: self.buffer,
                                    start: self.last_end,
                                    end: self.buffer.len(),
                                }
                            )
                        );
                        self.last_end = self.buffer.len();
                        return result;
                    }
                    return None;
                };

                let matched = captures.get_match();
                if self.last_end < matched.start() {
                    std::hint::cold_path();
                    let result = Some(
                        #ident::#case_ident_catchall(
                            #token_ident_catchall {
                                buffer: self.buffer,
                                start: self.last_end,
                                end: matched.start(),
                            }
                        )
                    );
                    self.last_end = matched.start();
                    return result;
                }

                let captures = self.captures
                    .next()
                    .expect("peek operation succeeded. At least one value remained");
                let matched = captures.get_match();
                self.last_end = matched.end();
                #(#capture_variants)*
                panic!("must match at least one capture group");
            }
        }
    }
}
