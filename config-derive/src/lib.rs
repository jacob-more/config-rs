use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{AttrStyle, Data, DataStruct, DeriveInput, Fields, Meta, spanned::Spanned};

use crate::field::{ConfigField, FieldType};

pub(crate) mod field;

#[proc_macro_derive(Config, attributes(key, default, lazy_lock, exhaustive, parse))]
pub fn config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    derive_config(ast).into()
}

fn derive_config(ast: DeriveInput) -> proc_macro2::TokenStream {
    // TODO: generics
    match ConfigStruct::parse(&ast) {
        Ok(config) => config.generate_impl_parse_ast(),
        Err(error) => error.to_compile_error(),
    }
}

#[proc_macro_derive(ConfigGroup, attributes(key, default, lazy_lock, exhaustive, parse))]
pub fn config_group(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    derive_config_group(ast).into()
}

fn derive_config_group(ast: DeriveInput) -> proc_macro2::TokenStream {
    // TODO: generics
    match ConfigStruct::parse(&ast) {
        Ok(config) => config.generate_impl_parse_ast_group(),
        Err(error) => error.to_compile_error(),
    }
}

#[proc_macro_derive(ConfigDefault, attributes(key, default, lazy_lock, exhaustive, parse))]
pub fn config_default(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    derive_config_default(ast).into()
}

fn derive_config_default(ast: DeriveInput) -> proc_macro2::TokenStream {
    // TODO: generics
    match ConfigStruct::parse(&ast) {
        Ok(config) => config.generate_impl_default(),
        Err(error) => error.to_compile_error(),
    }
}

#[proc_macro_derive(ConfigDisplay, attributes(key, default, lazy_lock, exhaustive, parse))]
pub fn config_display(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    derive_config_display(ast).into()
}

fn derive_config_display(ast: DeriveInput) -> proc_macro2::TokenStream {
    // TODO: generics
    match ConfigStruct::parse(&ast) {
        Ok(config) => config.generate_impl_display(),
        Err(error) => error.to_compile_error(),
    }
}

struct ConfigStruct<'a> {
    attributes: ConfigStructAttributes,
    data: &'a DeriveInput,
    data_struct: &'a DataStruct,
    fields: Vec<ConfigField<'a>>,
}

impl<'a> ConfigStruct<'a> {
    fn parse(data: &'a DeriveInput) -> Result<Self, syn::Error> {
        let Data::Struct(data_struct) = &data.data else {
            return Err(syn::Error::new(
                data.span(),
                "only struct types can derive Config (currently)",
            ));
        };
        let mut fields = Vec::with_capacity(data_struct.fields.len());
        for (index, field) in data_struct.fields.iter().enumerate() {
            fields.push(ConfigField::parse(field, index)?);
        }
        Ok(Self {
            attributes: ConfigStructAttributes::parse(data)?,
            data,
            data_struct,
            fields,
        })
    }

    fn generate_new_body(&self, group_key: Option<TokenStream>) -> TokenStream {
        let ident = self.fields.iter().map(|f| f.ident());
        let default_instantiate_statement = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Config))
            .map(|f| f.default().statement_instantiate());
        let instantiate_field = self.fields.iter().map(|f| match f.field_type() {
            FieldType::GroupKey => match group_key.clone() {
                Some(group_key) => group_key,
                None => {
                    let byte_literal = f.key_bytes().literal();
                    quote! {
                        ::config::derive::Bytes::from(#byte_literal.as_slice())
                    }
                }
            },
            FieldType::Config => f.default().expr_copy_from_ident().to_token_stream(),
            FieldType::Group => {
                let ty = f.ty();
                let byte_literal = f.key_bytes().literal();
                quote! {
                    <#ty as ::config::ConfigGroup>::new(
                        ::config::derive::Bytes::from(#byte_literal.as_slice())
                    )
                }
            }
        });

        match &self.data_struct.fields {
            Fields::Named(_) => quote! {
                #(#default_instantiate_statement)*
                Self {
                    #(#ident: #instantiate_field),*
                }
            },
            Fields::Unnamed(_) => quote! {
                #(#default_instantiate_statement)*
                Self(
                    #(#instantiate_field),*
                )
            },
            Fields::Unit => quote! {
                Self
            },
        }
    }

    fn generate_impl_default(&self) -> TokenStream {
        let struct_ident = &self.data.ident;
        let body = self.generate_new_body(None);
        quote! {
            impl ::core::default::Default for #struct_ident {
                fn default() -> Self {
                    #body
                }
            }
        }
    }

    fn generate_impl_parse_ast(&self) -> TokenStream {
        let struct_ident = &self.data.ident;

        let key_bytes_instantiate_statement = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Group | FieldType::Config))
            .map(|f| f.key_bytes().statement_instantiate());

        let groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Group));
        let configs = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Config));

        let group_key_pattern = groups.clone().map(|f| f.key_bytes().literal());
        let group_ident = groups.clone().map(|f| f.ident());
        let group_key_array = {
            let literal = groups.clone().map(|f| f.key_bytes().literal());
            quote! {
                [#(#literal,)*]
            }
        };

        let config_key_pattern = configs.clone().map(|f| f.key_bytes().literal());
        let config_ident = configs.clone().map(|f| f.ident());
        let config_key_array = {
            let literal = configs.clone().map(|f| f.key_bytes().literal());
            quote! {
                [#(#literal,)*]
            }
        };

        let replay_field = self.fields.iter().filter_map(|f| match f.field_type() {
            FieldType::GroupKey => None,
            FieldType::Config => {
                let ident = f.ident();
                let tokens = quote! {
                    ::config::ConfigOperationExt::replay(&mut self.#ident, &other.#ident);
                };
                Some(tokens)
            }
            FieldType::Group => {
                let ident = f.ident();
                let tokens = quote! {
                    ::config::ConfigGroup::replay(&mut self.#ident, &other.#ident);
                };
                Some(tokens)
            }
        });

        let ignore_unmatched_keys = match self.attributes.exhaustive {
            true => quote! {},
            false => quote! { _ => (), },
        };

        quote! {
            impl ::config::Config for #struct_ident {
                type Err = ::config::ConfigParseError;

                fn parse_ast(&mut self, ast: ::config::ast::AstTree) -> ::std::result::Result<(), Self::Err> {
                    #(#key_bytes_instantiate_statement)*

                    for entry in ::config::ast::AstTree::into_entries(ast) {
                        match entry {
                            ::config::ast::AstEntry::Group { key, group } => match ::std::ops::Deref::deref(&key) {
                                #(#group_key_pattern => if let Err(error) = ::config::ConfigGroup::parse_ast_group(&mut self.#group_ident, key, group) {
                                    return Err(Self::Err::Group(error));
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#config_key_array, &::std::ops::Deref::deref(&key)) {
                                        return Err(Self::Err::UnknownGroupKey(
                                            ::config::ast::AstEntry::Group { key, group }
                                        ));
                                    } else {
                                        return Err(Self::Err::UnknownKey(
                                            ::config::ast::AstEntry::Group { key, group }
                                        ));
                                    }
                                },
                            },
                            ::config::ast::AstEntry::Operation { key, operation } => match ::std::ops::Deref::deref(&key) {
                                #(#config_key_pattern => if let Err(error) = ::config::ConfigOperationExt::parse_ast_entry(&mut self.#config_ident, key, operation) {
                                    return Err(Self::Err::Operation(error));
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#group_key_array, &::std::ops::Deref::deref(&key)) {
                                        return Err(Self::Err::UnknownOperationKey(
                                            ::config::ast::AstEntry::Operation { key, operation }
                                        ));
                                    } else {
                                        return Err(Self::Err::UnknownKey(
                                            ::config::ast::AstEntry::Operation { key, operation }
                                        ));
                                    }
                                },
                            },
                        }
                    }
                    Ok(())
                }

                fn replay(&mut self, other: &Self) {
                    #(#replay_field)*
                }
            }
        }
    }

    fn generate_impl_parse_ast_group(&self) -> TokenStream {
        let struct_ident = &self.data.ident;

        // The function `new()` takes an argument `key` which we want to use
        // instead of the pre-defined literal.
        let new_body = self.generate_new_body(Some(quote! { key }));

        let key_bytes_instantiate_statement = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Group | FieldType::Config))
            .map(|f| f.key_bytes().statement_instantiate());

        let groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Group));
        let configs = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Config));

        let group_key_pattern = groups.clone().map(|f| f.key_bytes().literal());
        let group_ident = groups.clone().map(|f| f.ident());
        let group_key_array = {
            let literal = groups.clone().map(|f| f.key_bytes().literal());
            quote! {
                [#(#literal,)*]
            }
        };

        let config_key_pattern = configs.clone().map(|f| f.key_bytes().literal());
        let config_ident = configs.clone().map(|f| f.ident());
        let config_key_array = {
            let literal = configs.clone().map(|f| f.key_bytes().literal());
            quote! {
                [#(#literal,)*]
            }
        };

        let replay_field = self.fields.iter().filter_map(|f| match f.field_type() {
            FieldType::GroupKey => None,
            FieldType::Config => {
                let ident = f.ident();
                let tokens = quote! {
                    ::config::ConfigOperationExt::replay(&mut self.#ident, &other.#ident);
                };
                Some(tokens)
            }
            FieldType::Group => {
                let ident = f.ident();
                let tokens = quote! {
                    ::config::ConfigGroup::replay(&mut self.#ident, &other.#ident);
                };
                Some(tokens)
            }
        });

        let ignore_unmatched_keys = match self.attributes.exhaustive {
            true => quote! {},
            false => quote! { _ => (), },
        };

        quote! {
            impl ::config::ConfigGroup for #struct_ident {
                type Err = ::config::ConfigParseGroupError;

                fn new(key: ::config::derive::Bytes) -> Self {
                    #new_body
                }

                fn parse_ast_group(&mut self, key: ::config::derive::Bytes, ast: ::config::ast::AstGroup) -> ::std::result::Result<(), Self::Err> {
                    #(#key_bytes_instantiate_statement)*

                    let parent_key = key;
                    for entry in ::config::ast::AstGroup::into_entries(ast) {
                        match entry {
                            ::config::ast::AstEntry::Group { key, group } => match ::std::ops::Deref::deref(&key) {
                                #(#group_key_pattern => if let Err(error) = ::config::ConfigGroup::parse_ast_group(&mut self.#group_ident, key, group) {
                                    return Err(Self::Err::Group { group: parent_key, error });
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#config_key_array, &::std::ops::Deref::deref(&key)) {
                                        return Err(Self::Err::UnknownGroupKey {
                                            group: parent_key,
                                            entry: ::config::ast::AstEntry::Group { key, group }
                                        });
                                    } else {
                                        return Err(Self::Err::UnknownKey {
                                            group: parent_key,
                                            entry: ::config::ast::AstEntry::Group { key, group },
                                        });
                                    }
                                },
                            },
                            ::config::ast::AstEntry::Operation { key, operation } => match ::std::ops::Deref::deref(&key) {
                                #(#config_key_pattern => if let Err(error) = ::config::ConfigOperationExt::parse_ast_entry(&mut self.#config_ident, key, operation) {
                                    return Err(Self::Err::Operation { group: parent_key, error });
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#group_key_array, &::std::ops::Deref::deref(&key)) {
                                        return Err(Self::Err::UnknownOperationKey {
                                            group: parent_key,
                                            entry: ::config::ast::AstEntry::Operation { key, operation },
                                        });
                                    } else {
                                        return Err(Self::Err::UnknownKey {
                                            group: parent_key,
                                            entry: ::config::ast::AstEntry::Operation { key, operation },
                                        });
                                    }
                                },
                            },
                        }
                    }
                    Ok(())
                }

                fn replay(&mut self, other: &Self) {
                    #(#replay_field)*
                }
            }
        }
    }

    fn generate_impl_display(&self) -> TokenStream {
        todo!()
    }
}

struct ConfigStructAttributes {
    exhaustive: bool,
}

impl ConfigStructAttributes {
    fn parse(data: &DeriveInput) -> Result<Self, syn::Error> {
        let mut this = Self { exhaustive: false };
        for attribute in &data.attrs {
            if matches!(attribute.style, AttrStyle::Outer)
                && let Meta::Path(path) = &attribute.meta
                && path.is_ident("exhaustive")
            {
                this.exhaustive = true;
            } else {
                return Err(syn::Error::new(attribute.span(), "unknown attribute"));
            }
        }
        Ok(this)
    }
}
