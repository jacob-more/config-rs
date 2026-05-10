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
    let (capturing_token_ident, capturing_names): (Vec<_>, Vec<_>) = token_idents
        .clone()
        .filter(|(_, var)| var.patterns.is_capturing())
        .map(|(ident, var)| (ident, var.patterns.extra_capture_names()))
        .unzip();

    fn generate_capturing_token_struct(
        vis: &Visibility,
        token_ident: &[Ident],
        capture_name: &[Vec<Option<&str>>],
    ) -> TokenStream {
        let captures_count = capture_name.iter().map(|names| names.len());
        let (capture_index, capture_name): (Vec<_>, Vec<_>) = capture_name
            .iter()
            .map(|names| {
                let (indices, names): (Vec<_>, Vec<_>) = names
                    .iter()
                    .enumerate()
                    .filter_map(|(index, name)| Some((index, (*name)?)))
                    .unzip();
                (indices, names)
            })
            .unzip();
        quote! {
            #(
                #vis struct #token_ident<'a> {
                    pub(crate) buffer: &'a ::bytes::Bytes,
                    pub(crate) start: usize,
                    pub(crate) end: usize,
                    pub(crate) captures: [Option<(usize, usize)>; #captures_count],
                }

                impl<'a> #token_ident<'a> {
                    pub fn get_slice(&self, i: usize) -> Option<&'a [u8]> {
                        if 0 == i {
                            Some(&self.buffer[self.start..self.end])
                        } else {
                            self.captures
                                .get(i - 1)
                                .and_then(|pair| pair.as_ref())
                                .map(|(start, end)| &self.buffer[*start..*end])
                        }
                    }
                    pub fn get_bytes(&self, i: usize) -> Option<::bytes::Bytes> {
                        if 0 == i {
                            Some(self.buffer.slice(self.start..self.end))
                        } else {
                            self.captures
                                .get(i - 1)
                                .and_then(|pair| pair.as_ref())
                                .map(|(start, end)| self.buffer.slice(start..end))
                        }
                    }
                    pub fn name_slice(&self, name: &str) -> Option<&'a [u8]> {
                        match name {
                            #(
                                #capture_name => self.captures[#capture_index].map(|(start, end)| {
                                    &self.buffer[start..end]
                                }),
                            )*
                            _ => None,
                        }
                    }
                    pub fn name_bytes(&self, name: &str) -> Option<::bytes::Bytes> {
                        match name {
                            #(
                                #capture_name => self.captures[#capture_index].map(|(start, end)| {
                                    self.buffer.slice(start..end)
                                }),
                            )*
                            _ => None,
                        }
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
            )*
        }
    }

    let capturing_tokens =
        generate_capturing_token_struct(vis, &capturing_token_ident, &capturing_names);
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

            impl<'a> ::std::cmp::PartialEq for #token_ident<'a> {
                fn eq(&self, other: &Self) -> bool {
                    self.as_slice() == other.as_slice()
                }
            }
            impl<'a> ::std::cmp::Eq for #token_ident<'a> {}

            impl<'a> ::std::hash::Hash for #token_ident<'a> {
                fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                    self.as_slice().hash(state);
                }
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
        #[derive(PartialEq, Eq, Hash)]
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
            pattern: ::regex_automata::meta::Regex
        }

        impl Tokenizer {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn compile() -> Self {
                Self {
                    pattern: #tokenizer_regex
                }
            }
        }

        impl ::core::default::Default for Tokenizer {
            fn default() -> Self {
                static DEFAULT: std::sync::LazyLock<Tokenizer> =
                    std::sync::LazyLock::new(Tokenizer::compile);
                ::std::clone::Clone::clone(&*DEFAULT)
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

    let primary_capture_count = lex
        .variants
        .iter()
        .filter(|var| !matches!(var.patterns, LexPatterns::Any))
        .count();
    let secondary_capture_count = lex
        .variants
        .iter()
        .filter(|var| !matches!(var.patterns, LexPatterns::Any))
        .map(|var| var.patterns.extra_capture_count())
        .sum::<usize>();
    let mut secondary_capture_index = 2 * primary_capture_count;
    let capture_variants = lex
        .variants
        .iter()
        .filter(|var| !matches!(var.patterns, LexPatterns::Any))
        .enumerate()
        .map(|(pattern_id, var)| (u32::try_from(pattern_id).unwrap(), var))
        .map(|(pattern_id, var)| {
            let case_ident = &var.ident;
            let token_ident = var.token_ident(ident);

            if var.patterns.is_capturing() {
                let captures = (0..var.patterns.extra_capture_count()).map(|_| {
                    let start_index = secondary_capture_index;
                    let end_index = start_index + 1;
                    secondary_capture_index += 2;
                    quote! {
                        captures[#start_index].and_then(|start| {
                            Some((
                                start.get() + offset,
                                captures[#end_index]?.get() + offset
                            ))
                        }),
                    }
                });
                quote! {
                    #pattern_id => Some(#ident::#case_ident(#token_ident {
                        buffer: self.buffer,
                        start: offset + match_start,
                        end: offset + match_end,
                        captures: [ #(#captures)* ],
                    })),
                }
            } else {
                quote! {
                    #pattern_id => Some(#ident::#case_ident(#token_ident {
                        buffer: self.buffer,
                        start: offset + match_start,
                        end: offset + match_end,
                    })),
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #[derive(Debug)]
        #vis struct #iter_ident<'r, 'h> {
            buffer: &'h ::bytes::Bytes,
            tokenizer: &'r Tokenizer,
            last_end: usize,
        }

        impl<'r, 'h> #iter_ident<'r, 'h> {
            pub fn new(tokenizer: &'r Tokenizer, buffer: &'h ::bytes::Bytes) -> Self {
                Self {
                    buffer,
                    tokenizer,
                    last_end: 0,
                }
            }
        }

        impl<'r, 'h> Iterator for #iter_ident<'r, 'h> {
            type Item = #ident<'h>;

            fn next(&mut self) -> Option<Self::Item> {
                let mut captures = [None; 2 * (#primary_capture_count + #secondary_capture_count)];
                let Some(matched_pattern) = self
                    .tokenizer
                    .pattern
                    .search_slots(&self.buffer[self.last_end..].into(), &mut captures)
                else {
                    if self.last_end < self.buffer.len() {
                        ::std::hint::cold_path();
                        let result = #ident::#case_ident_catchall(
                            #token_ident_catchall {
                                buffer: self.buffer,
                                start: self.last_end,
                                end: self.buffer.len(),
                            }
                        );
                        self.last_end = self.buffer.len();
                        return Some(result);
                    }
                    return None;
                };

                let start_index = matched_pattern.as_usize() * 2;
                let end_index = start_index + 1;
                let match_start = captures[start_index]
                    .expect("first capture must be present with matching pattern id")
                    .get();
                let match_end = captures[end_index]
                    .expect("first capture must be present with matching pattern id")
                    .get();
                let offset = self.last_end;

                if self.last_end < (offset + match_start) {
                    ::std::hint::cold_path();
                    let result = #ident::#case_ident_catchall(
                        #token_ident_catchall {
                            buffer: self.buffer,
                            start: self.last_end,
                            end: offset + match_start,
                        }
                    );
                    self.last_end += match_start;
                    return Some(result);
                }

                self.last_end += match_end;
                match matched_pattern.as_u32() {
                    #(#capture_variants)*
                    _ => {
                        ::std::hint::cold_path();
                        panic!("must match at least one capture group");
                    }
                }
            }
        }
    }
}
