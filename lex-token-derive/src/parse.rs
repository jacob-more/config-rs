use std::num::NonZero;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Token, Visibility, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub struct LexEnum {
    pub vis: Visibility,
    pub enum_keyword: Token![enum],
    /// Name of the enum.
    pub ident: Ident,
    pub variants: Punctuated<LexVariant, Token![,]>,
}

impl Parse for LexEnum {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let vis = input.parse::<Visibility>()?;
        let enum_keyword = input.parse::<Token![enum]>()?;
        let ident = input.parse::<Ident>()?;

        let content;
        let _ = braced!(content in input);
        let variants = content.parse_terminated(LexVariant::parse, Token![,])?;

        Ok(Self {
            vis,
            enum_keyword,
            ident,
            variants,
        })
    }
}

impl LexEnum {
    pub fn tokenizer_regex(&self) -> TokenStream {
        let mut patterns = self
            .variants
            .iter()
            .filter(|var| matches!(var.patterns, LexPatterns::Patterns(_)))
            .map(|var| var.patterns.regex_pattern());
        let first_pattern = patterns
            .next()
            .map(|pat| {
                quote! {
                    ::std::write!(f, "({})", #pat)?;
                }
            })
            .expect("length is at least 1");
        let tail_patterns = patterns.map(|pat| {
            quote! {
                ::std::write!(f, "|({})", #pat)?;
            }
        });
        let pattern = quote! {
            ::std::fmt::from_fn(|f| {
                #first_pattern
                #(#tail_patterns)*
                ::std::result::Result::Ok(())
            })
        };
        quote! {
            ::regex::bytes::Regex::new(
                &::std::string::ToString::to_string(&#pattern)
            ).expect("tokenizer regex must be valid")
        }
    }

    pub fn iter_ident(&self) -> Ident {
        Ident::new(&format!("{}Iter", self.ident), self.ident.span())
    }
}

pub struct LexVariant {
    /// Name of the variant.
    pub ident: Ident,
    /// Content stored in the variant.
    pub patterns: LexPatterns,
}

impl Parse for LexVariant {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let ident = input.parse()?;
        let _ = input.parse::<Token![=]>()?;
        let patterns = input.parse()?;
        Ok(Self { ident, patterns })
    }
}

impl LexVariant {
    pub fn token_ident(&self, prefix: &Ident) -> Ident {
        let ident = &self.ident;
        Ident::new(&format!("{prefix}{ident}"), ident.span())
    }
}

pub enum LexPatterns {
    Any,
    Patterns(Punctuated<LexExpr, Token![|]>),
}

impl Parse for LexPatterns {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![_]) {
            let _ = input.parse::<Token![_]>()?;
            Ok(Self::Any)
        } else {
            Ok(Self::Patterns(Punctuated::parse_separated_nonempty(input)?))
        }
    }
}

impl LexPatterns {
    fn regex_pattern(&self) -> TokenStream {
        match self {
            Self::Any => quote! {},
            Self::Patterns(patterns) => {
                if patterns.len() == 1 {
                    patterns.first().unwrap().regex_pattern()
                } else {
                    let mut patterns = patterns.iter().map(LexExpr::regex_pattern);
                    let first_pattern = patterns
                        .next()
                        .map(|pat| {
                            quote! {
                                ::std::write!(f, "{}", #pat)?;
                            }
                        })
                        .expect("length is at least 2");
                    let tail_patterns = patterns.map(|pat| {
                        quote! {
                            ::std::write!(f, "|{}", #pat)?;
                        }
                    });
                    quote! {
                        ::std::fmt::from_fn(|f| {
                            #first_pattern
                            #(#tail_patterns)*
                            ::std::result::Result::Ok(())
                        })
                    }
                }
            }
        }
    }

    pub fn extra_capture_count(&self) -> usize {
        match self {
            Self::Any => 0,
            Self::Patterns(patterns) => patterns
                .iter()
                .map(|pat| match pat {
                    LexExpr::Captures(_, captures) => captures.get(),
                    LexExpr::Matches(_) => 0,
                    LexExpr::Constant(_) => 0,
                })
                .sum(),
        }
    }

    pub fn is_capturing(&self) -> bool {
        self.extra_capture_count() > 0
    }
}

pub enum LexExpr {
    Captures(LitStr, NonZero<usize>),
    Matches(LitStr),
    Constant(Ident),
}

impl Parse for LexExpr {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            let pattern = input.parse::<LitStr>()?;
            let pattern_string = pattern.value();
            let Ok(pattern_regex) = regex::bytes::Regex::new(&pattern_string) else {
                return Err(syn::Error::new(
                    pattern.span(),
                    format!("string {pattern_string} must be a valid regular expression"),
                ));
            };
            let capture_count = pattern_regex.captures_len();
            if capture_count > 1 {
                Ok(Self::Captures(
                    pattern,
                    NonZero::new(capture_count - 1).unwrap(),
                ))
            } else {
                Ok(Self::Matches(pattern))
            }
        } else {
            Ok(Self::Constant(input.parse()?))
        }
    }
}

impl LexExpr {
    fn regex_pattern(&self) -> TokenStream {
        match self {
            Self::Captures(lit_str, _) | Self::Matches(lit_str) => {
                quote! { #lit_str }
            }
            Self::Constant(ident) => {
                quote! { ::regex::escape(#ident) }
            }
        }
    }
}
