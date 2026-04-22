use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{AttrStyle, Data, DataStruct, DeriveInput, Fields, Meta, spanned::Spanned};

use crate::field::{ConfigField, FieldType};

pub(crate) mod field;

#[proc_macro_derive(
    Config,
    attributes(key, group_key, default, lazy_lock, exhaustive, parse)
)]
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

#[proc_macro_derive(
    ConfigGroup,
    attributes(key, group_key, default, lazy_lock, exhaustive, parse)
)]
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

#[proc_macro_derive(
    ConfigDefault,
    attributes(key, group_key, default, lazy_lock, exhaustive, parse)
)]
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

#[proc_macro_derive(
    ConfigDisplay,
    attributes(key, group_key, default, lazy_lock, exhaustive, parse)
)]
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
        // Ensure all keys are unique within namespaces
        for namespace in [FieldType::GroupKey, FieldType::Config, FieldType::Group] {
            let mut taken_keys = HashMap::with_capacity(fields.len());
            for field in fields.iter().filter(|f| f.field_type() == namespace) {
                let key = field.key_str().literal().value();
                if let Some(previous) = taken_keys.get(&key) {
                    return Err(syn::Error::new(
                        field.span(),
                        format!(
                            "field {} has the same key as {previous} ({key})",
                            field.ident()
                        ),
                    ));
                }
                taken_keys.insert(key, field.ident());
            }
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
                        ::config::Key::from(#byte_literal.as_slice())
                    }
                }
            },
            FieldType::Config => f.default().expr_copy_from_ident().to_token_stream(),
            FieldType::Group | FieldType::AnyGroup | FieldType::Flatten => {
                let ty = f.ty();
                let byte_literal = f.key_bytes().literal();
                quote! {
                    <#ty as ::config::ConfigGroup>::new(
                        ::config::Key::from(#byte_literal.as_slice())
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

        let flat_groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Flatten));
        let groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Group));
        let any_groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::AnyGroup));
        let configs = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Config));

        let flat_ident = flat_groups.map(|f| f.ident());

        let group_key_pattern = groups.clone().map(|f| f.key_bytes().literal());
        let group_ident = groups.clone().map(|f| f.ident());
        let group_key_array = {
            let literal = groups.clone().map(|f| f.key_bytes().literal());
            quote! {
                [#(#literal,)*]
            }
        };

        let any_group_ident = any_groups.map(|f| f.ident());

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
            FieldType::Group | FieldType::AnyGroup | FieldType::Flatten => {
                let ident = f.ident();
                let tokens = quote! {
                    ::config::ConfigGroup::replay(&mut self.#ident, &other.#ident);
                };
                Some(tokens)
            }
        });

        let display_body = self.generate_impl_display_body(Some(quote! { &fmt }));

        let ignore_unmatched_keys = match self.attributes.exhaustive {
            true => quote! {},
            false => quote! { _ => (), },
        };

        quote! {
            impl ::config::Config for #struct_ident {
                type Err = ::config::ConfigParseError;

                #[allow(unreachable_code)]
                fn parse_ast_entry(
                    &mut self,
                    entry: ::config::ast::AstEntry
                ) -> ::std::result::Result<(), Self::Err> {
                    #(
                        let entry = match ::config::Config::parse_ast_entry(&mut self.#flat_ident, entry) {
                            ::std::result::Result::Ok(()) => return ::std::result::Result::Ok(()),
                            ::std::result::Result::Err(::config::ConfigParseError::UnknownKey(rejected_entry)) => rejected_entry,
                            ::std::result::Result::Err(::config::ConfigParseError::UnknownGroupKey(rejected_entry)) => rejected_entry,
                            ::std::result::Result::Err(::config::ConfigParseError::UnknownOperationKey(rejected_entry)) => rejected_entry,
                            ::std::result::Result::Err(error) => return ::std::result::Result::Err(error),
                        };
                    )*

                    match entry {
                        ::config::ast::AstEntry::Group { key, group } => match ::std::ops::Deref::deref(&key) {
                            #(#group_key_pattern => if let ::std::result::Result::Err(error) =
                                ::config::ConfigGroup::parse_ast_group(
                                    &mut self.#group_ident,
                                    ::core::convert::From::from(key),
                                    group
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseError::Group(error)
                                );
                            },)*
                            #(_ => if let ::std::result::Result::Err(error) =
                                ::config::ConfigGroup::parse_ast_group(
                                    &mut self.#any_group_ident,
                                    ::core::convert::From::from(key),
                                    group
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseError::Group(error)
                                );
                            },)*
                            #ignore_unmatched_keys
                            _ => {
                                if <[&[u8]]>::contains(&#config_key_array, &::std::ops::Deref::deref(&key)) {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownGroupKey(
                                            ::config::ast::AstEntry::Group { key, group }
                                        )
                                    );
                                } else {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownKey(
                                            ::config::ast::AstEntry::Group { key, group }
                                        )
                                    );
                                }
                            },
                        },
                        ::config::ast::AstEntry::Operation { key, operation } => match ::std::ops::Deref::deref(&key) {
                            #(#config_key_pattern => if let ::std::result::Result::Err(error) =
                                ::config::ConfigOperationExt::parse_ast_entry(
                                    &mut self.#config_ident,
                                    ::core::convert::From::from(key),
                                    operation
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseError::Operation(error)
                                );
                            },)*
                            #ignore_unmatched_keys
                            _ => {
                                if <[&[u8]]>::contains(&#group_key_array, &::std::ops::Deref::deref(&key)) {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownOperationKey(
                                            ::config::ast::AstEntry::Operation { key, operation }
                                        )
                                    );
                                } else {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownKey(
                                            ::config::ast::AstEntry::Operation { key, operation }
                                        )
                                    );
                                }
                            },
                        },
                    }
                    Ok(())
                }

                fn replay(&mut self, other: &Self) {
                    #(#replay_field)*
                }

                fn display(&self, fmt: ::config::ConfigFmt) -> impl ::std::fmt::Display {
                    ::std::fmt::from_fn(move |f| {
                        #display_body
                    })
                }
            }
        }
    }

    fn generate_impl_parse_ast_group(&self) -> TokenStream {
        let struct_ident = &self.data.ident;

        // The function `new()` takes an argument `key` which we want to use
        // instead of the pre-defined literal.
        let new_body = self.generate_new_body(Some(quote! { key }));

        let group_keys = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::GroupKey));
        let flat_groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Flatten));
        let groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Group));
        let any_groups = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::AnyGroup));
        let configs = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Config));

        let group_key = group_keys.map(|f| {
            let ident = f.ident();
            quote! {
                self.#ident = ::std::clone::Clone::clone(&key);
            }
        });

        let flat_ident = flat_groups.map(|f| f.ident());

        let group_key_pattern = groups.clone().map(|f| f.key_bytes().literal());
        let group_ident = groups.clone().map(|f| f.ident());
        let group_key_array = {
            let literal = groups.clone().map(|f| f.key_bytes().literal());
            quote! {
                [#(#literal,)*]
            }
        };

        let any_group_ident = any_groups.map(|f| f.ident());

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
            FieldType::Group | FieldType::AnyGroup | FieldType::Flatten => {
                let ident = f.ident();
                let tokens = quote! {
                    ::config::ConfigGroup::replay(&mut self.#ident, &other.#ident);
                };
                Some(tokens)
            }
        });

        let display_body = self.generate_impl_display_body(Some(quote! { &fmt }));

        let ignore_unmatched_keys = match self.attributes.exhaustive {
            true => quote! {},
            false => quote! { _ => (), },
        };

        quote! {
            impl ::config::ConfigGroup for #struct_ident {
                type Err = ::config::ConfigParseGroupError;

                fn new(key: ::config::Key) -> Self {
                    #new_body
                }

                #[allow(unreachable_code)]
                fn parse_ast_entry(
                    &mut self,
                    key: &::config::Key,
                    entry: ::config::ast::AstEntry
                ) -> ::std::result::Result<(), Self::Err> {
                    #(#group_key)*
                    let parent_key = key;

                    #(
                        let entry = match ::config::ConfigGroup::parse_ast_entry(&mut self.#flat_ident, parent_key, entry) {
                            ::std::result::Result::Ok(()) => return ::std::result::Result::Ok(()),
                            ::std::result::Result::Err(::config::ConfigParseGroupError::UnknownKey { entry: rejected_entry, .. }) => rejected_entry,
                            ::std::result::Result::Err(::config::ConfigParseGroupError::UnknownGroupKey { entry: rejected_entry, .. }) => rejected_entry,
                            ::std::result::Result::Err(::config::ConfigParseGroupError::UnknownOperationKey { entry: rejected_entry, .. }) => rejected_entry,
                            ::std::result::Result::Err(error) => return ::std::result::Result::Err(error),
                        };
                    )*

                    match entry {
                        ::config::ast::AstEntry::Group { key, group } => match ::std::ops::Deref::deref(&key) {
                            #(#group_key_pattern => if let ::std::result::Result::Err(error) =
                                ::config::ConfigGroup::parse_ast_group(
                                    &mut self.#group_ident,
                                    ::core::convert::From::from(key),
                                    group
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseGroupError::Group {
                                        group: ::config::derive::Bytes::from(
                                            ::std::clone::Clone::clone(parent_key)
                                        ),
                                        error: ::std::boxed::Box::new(error),
                                    }
                                );
                            },)*
                            #(_ => if let ::std::result::Result::Err(error) =
                                ::config::ConfigGroup::parse_ast_group(
                                    &mut self.#any_group_ident,
                                    ::core::convert::From::from(key),
                                    group
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseGroupError::Group {
                                        group: ::config::derive::Bytes::from(
                                            ::std::clone::Clone::clone(parent_key)
                                        ),
                                        error: ::std::boxed::Box::new(error),
                                    }
                                );
                            },)*
                            #ignore_unmatched_keys
                            _ => {
                                if <[&[u8]]>::contains(&#config_key_array, &::std::ops::Deref::deref(&key)) {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseGroupError::UnknownGroupKey {
                                            group: ::config::derive::Bytes::from(
                                                ::std::clone::Clone::clone(parent_key)
                                            ),
                                            entry: ::config::ast::AstEntry::Group { key, group },
                                        }
                                    );
                                } else {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseGroupError::UnknownKey {
                                            group: ::config::derive::Bytes::from(
                                                ::std::clone::Clone::clone(parent_key)
                                            ),
                                            entry: ::config::ast::AstEntry::Group { key, group },
                                        }
                                    );
                                }
                            },
                        },
                        ::config::ast::AstEntry::Operation { key, operation } => match ::std::ops::Deref::deref(&key) {
                            #(#config_key_pattern => if let ::std::result::Result::Err(error) =
                                ::config::ConfigOperationExt::parse_ast_entry(
                                    &mut self.#config_ident,
                                    ::core::convert::From::from(key),
                                    operation
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseGroupError::Operation {
                                        group: ::config::derive::Bytes::from(
                                            ::std::clone::Clone::clone(parent_key)
                                        ),
                                        error,
                                    }
                                );
                            },)*
                            #ignore_unmatched_keys
                            _ => {
                                if <[&[u8]]>::contains(&#group_key_array, &::std::ops::Deref::deref(&key)) {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseGroupError::UnknownOperationKey {
                                            group: ::config::derive::Bytes::from(
                                                ::std::clone::Clone::clone(parent_key)
                                            ),
                                            entry: ::config::ast::AstEntry::Operation { key, operation },
                                        }
                                    );
                                } else {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseGroupError::UnknownKey {
                                            group: ::config::derive::Bytes::from(
                                                ::std::clone::Clone::clone(parent_key)
                                            ),
                                            entry: ::config::ast::AstEntry::Operation { key, operation },
                                        }
                                    );
                                }
                            },
                        },
                    }
                    Ok(())
                }

                fn replay(&mut self, other: &Self) {
                    #(#replay_field)*
                }

                fn display(&self, fmt: ::config::ConfigFmt) -> impl ::std::fmt::Display {
                    ::std::fmt::from_fn(move |f| {
                        #display_body
                    })
                }
            }
        }
    }

    fn generate_impl_display_body(&self, fmt: Option<TokenStream>) -> TokenStream {
        let group_key = self
            .fields
            .iter()
            .find(|f| matches!(f.field_type(), FieldType::GroupKey))
            .map(|f| {
                let key_ident = f.ident();
                quote! {
                    &self.#key_ident
                }
            });
        let fields = self
            .fields
            .iter()
            .filter_map(|f| {
                let ident = f.ident();
                match f.field_type() {
                    FieldType::GroupKey => None,
                    FieldType::Config => Some(quote! {
                        ::config::ConfigOperation::display(
                            &self.#ident,
                            ::config::ConfigFmt::next(&fmt),
                        )
                    }),
                    FieldType::Group | FieldType::AnyGroup => Some(quote! {
                        ::config::ConfigGroup::display(
                            &self.#ident,
                            ::config::ConfigFmt::next(&fmt),
                        )
                    }),
                    FieldType::Flatten => Some(quote! {
                        ::config::ConfigGroup::display(
                            &self.#ident,
                            ::config::ConfigFmt::with_flatten(
                                ::config::ConfigFmt::next(&fmt),
                            )
                        )
                    }),
                }
            })
            .collect::<Vec<_>>();

        let fmt = fmt.unwrap_or_else(|| {
            if group_key.is_some() {
                quote! {
                    ::config::ConfigFmt::new()
                }
            } else {
                quote! {
                    ::config::ConfigFmt::with_flatten(
                        ::config::ConfigFmt::new()
                    )
                }
            }
        });

        match (group_key, fields.split_last()) {
            (Some(group_key), Some((last_field, fields))) => {
                quote! {
                    let fmt = #fmt;
                    let indent = ::config::ConfigFmt::indent(&fmt);
                    let inner_fmt = ::config::ConfigFmt::next(&fmt);

                    if !::config::ConfigFmt::flatten(&fmt) {
                        ::core::writeln!(f, "{indent}{}: {{", #group_key)?;
                    }
                    #(::core::writeln!(f, "{}", #fields)?;)*
                    ::core::write!(f, "{}", #last_field)?;
                    if !::config::ConfigFmt::flatten(&fmt) {
                        ::core::write!(f, "\n{indent}}}")?;
                    }
                    ::std::result::Result::Ok(())
                }
            }
            (Some(group_key), None) => {
                quote! {
                    let fmt = #fmt;
                    let indent = ::config::ConfigFmt::indent(&fmt);

                    if !::config::ConfigFmt::flatten(&fmt) {
                        ::core::write!(f, "{indent}{}: {{ }}", #group_key)?;
                    }
                    ::std::result::Result::Ok(())
                }
            }
            (None, Some((last_field, fields))) => {
                quote! {
                    let fmt = #fmt;

                    #(::core::writeln!(f, "{}", #fields)?;)*
                    ::core::write!(f, "{}", #last_field)
                }
            }
            (None, None) => {
                quote! {
                    ::std::result::Result::Ok(())
                }
            }
        }
    }

    fn generate_impl_display(&self) -> TokenStream {
        let struct_ident = &self.data.ident;
        let body = self.generate_impl_display_body(None);

        quote! {
            impl ::std::fmt::Display for #struct_ident {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    #body
                }
            }
        }
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
