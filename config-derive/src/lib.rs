use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{AttrStyle, Data, DataStruct, DeriveInput, Fields, Meta, spanned::Spanned};

use crate::field::{ConfigField, FieldType};

pub(crate) mod field;

#[proc_macro_derive(ConfigGroup, attributes(key, default, lazy_lock, exhaustive, parse))]
pub fn config_group(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as DeriveInput);
    derive_config_group(ast).into()
}

fn derive_config_group(ast: DeriveInput) -> proc_macro2::TokenStream {
    // TODO: generics
    match ConfigStruct::parse(&ast) {
        Ok(config) => config.generate_impl_parse_group(),
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
        // Ensure all keys are unique within namespaces
        for namespace in [FieldType::Collection, FieldType::Group] {
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

    fn generate_impl_default(&self) -> TokenStream {
        let struct_ident = &self.data.ident;
        let ident = self.fields.iter().map(|f| f.ident());
        let default_instantiate_statement = self
            .fields
            .iter()
            .filter(|f| matches!(f.field_type(), FieldType::Collection))
            .map(|f| f.default().statement_instantiate());
        let instantiate_field = self.fields.iter().map(|f| match f.field_type() {
            FieldType::Collection => f.default().expr_copy_from_ident().to_token_stream(),
            FieldType::Group | FieldType::AnyGroup | FieldType::Flatten => {
                let ty = f.ty();
                quote! {
                    <#ty as ::core::default::Default>::default()
                }
            }
        });

        let body = match &self.data_struct.fields {
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
        };
        quote! {
            impl ::core::default::Default for #struct_ident {
                fn default() -> Self {
                    #body
                }
            }
        }
    }

    fn generate_impl_parse_group(&self) -> TokenStream {
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
            .filter(|f| matches!(f.field_type(), FieldType::Collection));

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

        let replay_field = self.fields.iter().map(|f| match f.field_type() {
            FieldType::Collection => {
                let ident = f.ident();
                quote! {
                    ::config::ConfigCollectionExt::replay(&mut self.#ident, &other.#ident);
                }
            }
            FieldType::Group | FieldType::AnyGroup | FieldType::Flatten => {
                let ident = f.ident();
                quote! {
                    ::config::ConfigGroup::replay(&mut self.#ident, &other.#ident);
                }
            }
        });

        let display_body = self.generate_impl_display_body(quote! { &fmt });

        let ignore_unmatched_keys = match self.attributes.exhaustive {
            true => quote! {},
            false => quote! { _ => (), },
        };

        quote! {
            impl ::config::ConfigGroup for #struct_ident {
                type Err = ::config::ConfigParseError;

                #[allow(unreachable_code)]
                fn parse_entry(
                    &mut self,
                    entry: ::config::parse::RawEntry
                ) -> ::std::result::Result<(), Self::Err> {
                    #(
                        let entry = match ::config::ConfigGroup::parse_entry(&mut self.#flat_ident, entry) {
                            ::std::result::Result::Ok(()) => return ::std::result::Result::Ok(()),
                            ::std::result::Result::Err(::config::ConfigParseError::UnknownKey(rejected_entry)) => rejected_entry,
                            ::std::result::Result::Err(::config::ConfigParseError::UnknownGroupKey(rejected_entry)) => rejected_entry,
                            ::std::result::Result::Err(::config::ConfigParseError::UnknownCollectionKey(rejected_entry)) => rejected_entry,
                            ::std::result::Result::Err(error) => return ::std::result::Result::Err(error),
                        };
                    )*

                    match entry {
                        ::config::parse::RawEntry::Group { key, body } => match ::std::ops::Deref::deref(&key) {
                            #(#group_key_pattern => if let ::std::result::Result::Err(error) =
                                ::config::ConfigGroup::parse(
                                    &mut self.#group_ident,
                                    body
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseError::Group(
                                        ::std::boxed::Box::new(error)
                                    )
                                );
                            },)*
                            #(_ => if let ::std::result::Result::Err(error) =
                                ::config::ConfigGroup::parse_entry(
                                    &mut self.#any_group_ident,
                                    ::config::parse::RawEntry::Group { key, body }
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseError::Group(
                                        ::std::boxed::Box::new(error)
                                    )
                                );
                            },)*
                            #ignore_unmatched_keys
                            _ => {
                                if <[&[u8]]>::contains(&#config_key_array, &::std::ops::Deref::deref(&key)) {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownGroupKey(
                                            ::config::parse::RawEntry::Group { key, body }
                                        )
                                    );
                                } else {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownKey(
                                            ::config::parse::RawEntry::Group { key, body }
                                        )
                                    );
                                }
                            },
                        },
                        ::config::parse::RawEntry::Collection { key, body } => match ::std::ops::Deref::deref(&key) {
                            #(#config_key_pattern => if let ::std::result::Result::Err(error) =
                                ::config::ConfigCollectionExt::parse_entry(
                                    &mut self.#config_ident,
                                    body
                                )
                            {
                                return ::std::result::Result::Err(
                                    ::config::ConfigParseError::Entry(error)
                                );
                            },)*
                            #ignore_unmatched_keys
                            _ => {
                                if <[&[u8]]>::contains(&#group_key_array, &::std::ops::Deref::deref(&key)) {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownCollectionKey(
                                            ::config::parse::RawEntry::Collection { key, body }
                                        )
                                    );
                                } else {
                                    return ::std::result::Result::Err(
                                        ::config::ConfigParseError::UnknownKey(
                                            ::config::parse::RawEntry::Collection { key, body }
                                        )
                                    );
                                }
                            },
                        },
                    }
                    ::std::result::Result::Ok(())
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

    fn generate_impl_display_body(&self, ref_config_fmt: TokenStream) -> TokenStream {
        let fields = self
            .fields
            .iter()
            .map(|f| {
                let ident = f.ident();
                let key = f.key_bytes().literal();
                match f.field_type() {
                    FieldType::Collection => quote! {
                        ::config::ConfigCollection::display(
                            &self.#ident,
                            ::config::ConfigFmt::with_key(
                                ::config::ConfigFmt::next(fmt),
                                &::config::Key::from_static(#key),
                            ),
                        )
                    },
                    FieldType::Group | FieldType::AnyGroup => quote! {
                        ::config::ConfigGroup::display(
                            &self.#ident,
                            ::config::ConfigFmt::with_key(
                                ::config::ConfigFmt::next(fmt),
                                &::config::Key::from_static(#key),
                            ),
                        )
                    },
                    FieldType::Flatten => quote! {
                        ::config::ConfigGroup::display(
                            &self.#ident,
                            ::config::ConfigFmt::with_flatten(
                                ::config::ConfigFmt::with_key(
                                    ::config::ConfigFmt::next(fmt),
                                    &::config::Key::from_static(#key),
                                )
                            ),
                        )
                    },
                }
            })
            .collect::<Vec<_>>();

        match fields.split_last() {
            Some((last_field, fields)) => {
                quote! {
                    let fmt = #ref_config_fmt;
                    match fmt.key() {
                        Some(key) => {
                            let indent = ::config::ConfigFmt::indent(fmt);

                            if !::config::ConfigFmt::flatten(fmt) {
                                ::core::writeln!(f, "{indent}{key}: {{")?;
                            }
                            #(::core::writeln!(f, "{}", #fields)?;)*
                            ::core::write!(f, "{}", #last_field)?;
                            if !::config::ConfigFmt::flatten(fmt) {
                                ::core::write!(f, "\n{indent}}}")?;
                            }
                            ::std::result::Result::Ok(())
                        },
                        None => {
                            // let fmt = ::config::ConfigFmt::with_flatten(fmt);
                            #(::core::writeln!(f, "{}", #fields)?;)*
                            ::core::write!(f, "{}", #last_field)
                        }
                    }
                }
            }
            None => {
                quote! {
                    let fmt = #ref_config_fmt;
                    if let Some(key) = fmt.key() {
                        let indent = ::config::ConfigFmt::indent(fmt);

                        if !::config::ConfigFmt::flatten(fmt) {
                            ::core::write!(f, "{indent}{key}: {{ }}")?;
                        }
                    }
                    ::std::result::Result::Ok(())
                }
            }
        }
    }

    fn generate_impl_display(&self) -> TokenStream {
        let struct_ident = &self.data.ident;
        let body = self.generate_impl_display_body(quote! {
            &::config::ConfigFmt::with_flatten(
                ::config::ConfigFmt::new()
            )
        });

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
