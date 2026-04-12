use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    AttrStyle, Data, DataStruct, DeriveInput, Expr, Field, Fields, Ident, LitBool, LitByteStr,
    LitStr, Meta, Type, TypePath, spanned::Spanned,
};

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
        for field in &data_struct.fields {
            fields.push(ConfigField::parse(field)?);
        }
        Ok(Self {
            attributes: ConfigStructAttributes::parse(data)?,
            data,
            data_struct,
            fields,
        })
    }

    fn fields_with_ident(&self) -> Vec<(&ConfigField<'a>, Ident)> {
        match self.data_struct.fields {
            Fields::Named(_) => self.fields.iter().map(|f| (f, f.ident.clone())).collect(),
            Fields::Unnamed(_) => self
                .fields
                .iter()
                .enumerate()
                .map(|(index, f)| (f, format_ident!("{index}")))
                .collect(),
            Fields::Unit => Vec::new(),
        }
    }

    fn generate_new_body(&self) -> TokenStream {
        let fields_vec = self.fields_with_ident();
        let fields = fields_vec
            .iter()
            .map(|(f, i)| (f, i, &f.field.ty, f.literal_bytes()));

        let ident = fields.clone().map(|(_, ident, _, _)| ident);
        let instantiate_field =
            fields
                .clone()
                .map(|(f, _, ty, byte_literal)| match f.attributes.parser {
                    ConfigParser::GroupKey => quote! { key },
                    ConfigParser::Operation => f.expr_copy_default(),
                    ConfigParser::Group => quote! {
                        <#ty as ::config::ConfigGroup>::new(
                            ::bytes::Bytes::from(#byte_literal.as_slice())
                        )
                    },
                });
        let default_constant = fields
            .filter(|(f, _, _, _)| matches!(f.attributes.parser, ConfigParser::Operation))
            .map(|(f, _, _, _)| f.constant_statement_default());

        match &self.data_struct.fields {
            Fields::Named(_) => quote! {
                #(#default_constant)*
                Self {
                    #(#ident: #instantiate_field),*
                }
            },
            Fields::Unnamed(_) => quote! {
                #(#default_constant)*
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
        let body = self.generate_new_body();
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
        let fields = self.fields_with_ident();

        let groups = fields
            .iter()
            .filter(|(f, _)| matches!(f.attributes.parser, ConfigParser::Group));
        let operations = fields
            .iter()
            .filter(|(f, _)| matches!(f.attributes.parser, ConfigParser::Operation));

        let group_byte_constants = groups.clone().map(|(f, _)| f.constant_statement_bytes());
        let group_key = groups.clone().map(|(f, _)| f.ident_bytes());
        let err_group_key = group_key.clone();
        let group_field = groups
            .clone()
            .map(|(_, field_ident)| quote! { #field_ident });

        let operation_byte_constants = operations
            .clone()
            .map(|(f, _)| f.constant_statement_bytes());
        let operation_key = operations.clone().map(|(f, _)| f.ident_bytes());
        let err_operation_key = operation_key.clone();
        let operation_field = operations
            .clone()
            .map(|(_, field_ident)| quote! { #field_ident });

        let group_replay = groups.map(|(_, ident)| {
            quote! {
                ::config::ConfigGroup::replay(&mut self.#ident, &other.#ident);
            }
        });
        let operation_replay = operations.map(|(_, ident)| {
            quote! {
                ::config::ConfigOperationExt::replay(&mut self.#ident, &other.#ident);
            }
        });

        let err_group_keys = quote! {
            [#(#err_group_key,)*]
        };
        let err_operation_keys = quote! {
            [#(#err_operation_key,)*]
        };

        let ignore_unmatched_keys = match self.attributes.exhaustive {
            true => quote! {},
            false => quote! { _ => (), },
        };

        quote! {
            impl ::config::Config for #struct_ident {
                type Err = ::config::ConfigParseError;

                fn parse_ast(&mut self, ast: ::config::ast::AstTree) -> ::std::result::Result<(), Self::Err> {
                    #(#group_byte_constants)*
                    #(#operation_byte_constants)*

                    for entry in ::config::ast::AstTree::into_entries(ast) {
                        match entry {
                            ::config::ast::AstEntry::Group { key, group } => match ::std::ops::Deref::deref(&key) {
                                #(#group_key => if let Err(error) = ::config::ConfigGroup::parse_ast_group(&mut self.#group_field, key, group) {
                                    return Err(Self::Err::Group(error));
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#err_operation_keys, &::std::ops::Deref::deref(&key)) {
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
                                #(#operation_key => if let Err(error) = ::config::ConfigOperationExt::parse_ast_entry(&mut self.#operation_field, key, operation) {
                                    return Err(Self::Err::Operation(error));
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#err_group_keys, &::std::ops::Deref::deref(&key)) {
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
                    #(#group_replay)*
                    #(#operation_replay)*
                }
            }
        }
    }

    fn generate_impl_parse_ast_group(&self) -> TokenStream {
        let struct_ident = &self.data.ident;
        let fields = self.fields_with_ident();

        let groups = fields
            .iter()
            .filter(|(f, _)| matches!(f.attributes.parser, ConfigParser::Group));
        let operations = fields
            .iter()
            .filter(|(f, _)| matches!(f.attributes.parser, ConfigParser::Operation));

        let new_body = self.generate_new_body();

        let group_byte_constants = groups.clone().map(|(f, _)| f.constant_statement_bytes());
        let group_key = groups.clone().map(|(f, _)| f.ident_bytes());
        let err_group_key = group_key.clone();
        let group_field = groups
            .clone()
            .map(|(_, field_ident)| quote! { #field_ident });

        let operation_byte_constants = operations
            .clone()
            .map(|(f, _)| f.constant_statement_bytes());
        let operation_key = operations.clone().map(|(f, _)| f.ident_bytes());
        let err_operation_key = operation_key.clone();
        let operation_field = operations
            .clone()
            .map(|(_, field_ident)| quote! { #field_ident });

        let group_replay = groups.map(|(_, ident)| {
            quote! {
                ::config::ConfigGroup::replay(&mut self.#ident, &other.#ident);
            }
        });
        let operation_replay = operations.map(|(_, ident)| {
            quote! {
                ::config::ConfigOperationExt::replay(&mut self.#ident, &other.#ident);
            }
        });

        let err_group_keys = quote! {
            [#(#err_group_key,)*]
        };
        let err_operation_keys = quote! {
            [#(#err_operation_key,)*]
        };

        let ignore_unmatched_keys = match self.attributes.exhaustive {
            true => quote! {},
            false => quote! { _ => (), },
        };

        quote! {
            impl ::config::ConfigGroup for #struct_ident {
                type Err = ::config::ConfigParseGroupError;

                fn new(key: ::bytes::Bytes) -> Self {
                    #new_body
                }

                fn parse_ast_group(&mut self, key: bytes::Bytes, ast: ::config::ast::AstGroup) -> ::std::result::Result<(), Self::Err> {
                    #(#group_byte_constants)*
                    #(#operation_byte_constants)*

                    let parent_key = key;
                    for entry in ::config::ast::AstGroup::into_entries(ast) {
                        match entry {
                            ::config::ast::AstEntry::Group { key, group } => match ::std::ops::Deref::deref(&key) {
                                #(#group_key => if let Err(error) = ::config::ConfigGroup::parse_ast_group(&mut self.#group_field, key, group) {
                                    return Err(Self::Err::Group { group: parent_key, error });
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#err_operation_keys, &::std::ops::Deref::deref(&key)) {
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
                                #(#operation_key => if let Err(error) = ::config::ConfigOperationExt::parse_ast_entry(&mut self.#operation_field, key, operation) {
                                    return Err(Self::Err::Operation { group: parent_key, error });
                                },)*
                                #ignore_unmatched_keys
                                _ => {
                                    if <[&[u8]]>::contains(&#err_group_keys, &::std::ops::Deref::deref(&key)) {
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
                    #(#group_replay)*
                    #(#operation_replay)*
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

struct ConfigField<'a> {
    attributes: ConfigFieldAttributes,
    field: &'a Field,
    ident: Ident,
    key_span: Span,
    key: String,
}

impl<'a> ConfigField<'a> {
    fn parse(field: &'a Field) -> Result<Self, syn::Error> {
        let attributes = ConfigFieldAttributes::parse(field)?;

        let (key, key_span) = match (&field.ident, &attributes.key) {
            (_, Some(key)) => (key.suffix().to_string(), key.span()),
            (Some(key), None) => (key.to_string(), key.span()),
            (None, None) => {
                return Err(syn::Error::new(
                    field.span(),
                    "missing attribute 'key' for unnamed field",
                ));
            }
        };
        let ident = match &field.ident {
            Some(ident) => ident.clone(),
            None => Ident::new(&key, key_span),
        };

        Ok(Self {
            attributes: ConfigFieldAttributes::parse(field)?,
            field,
            ident,
            key_span,
            key,
        })
    }

    fn ident_default(&self) -> Ident {
        format_ident!("DEFAULT_{}", self.key.to_uppercase())
    }

    fn expr_default(&self) -> TokenStream {
        let ty = &self.field.ty;
        let literal_key = self.literal_key();
        match &self.attributes.default {
            Some(default_expr) => quote! {
                <#ty>::new_with_default(#literal_key, #default_expr)
            },
            None => quote! {
                <#ty>::new(#literal_key)
            },
        }
    }

    fn constant_statement_default(&self) -> TokenStream {
        let ty = &self.field.ty;
        let ident = self.ident_default();
        let expr = self.expr_default();
        match self.attributes.lazy_lock {
            true => quote! {
                static #ident: ::std::sync::LazyLock<#ty> =
                    ::std::sync::LazyLock::new(|| #expr);
            },
            false => quote! {
                const #ident: #ty = #expr;
            },
        }
    }

    fn expr_copy_default(&self) -> TokenStream {
        let ident = self.ident_default();
        match self.attributes.lazy_lock {
            true => quote! {
                ::std::clone::Clone::clone(
                    ::std::ops::Deref::deref(&#ident)
                )
            },
            false => quote! {
                (#ident)
            },
        }
    }

    fn ident_key(&self) -> Ident {
        format_ident!("KEY_{}", self.key.to_uppercase())
    }

    fn literal_key(&self) -> LitStr {
        LitStr::new(&self.key.to_uppercase(), self.key_span)
    }

    fn constant_statement_key(&self) -> TokenStream {
        let ident = self.ident_key();
        let expr = self.literal_key();
        quote! {
            const #ident: &str = #expr;
        }
    }

    fn ident_bytes(&self) -> Ident {
        format_ident!("KEY_BYTES_{}", self.key.to_uppercase())
    }

    fn literal_bytes(&self) -> LitByteStr {
        LitByteStr::new(self.key.to_uppercase().as_bytes(), self.key_span)
    }

    fn constant_statement_bytes(&self) -> TokenStream {
        let ident = self.ident_bytes();
        let expr = self.literal_bytes();
        quote! {
            const #ident: &[u8] = #expr;
        }
    }
}

struct ConfigFieldAttributes {
    key: Option<LitStr>,
    default: Option<Expr>,
    lazy_lock: bool,
    parser: ConfigParser,
}

impl ConfigFieldAttributes {
    fn parse(field: &Field) -> Result<Self, syn::Error> {
        let wkt = WellKnownType::parse(field);

        let mut key = None;
        let mut default = None;
        let mut lazy_lock = None;
        let mut parser: Option<ConfigParser> = None;

        let mut default_lazy_lock = wkt.is_some_and(|wkc| !wkc.is_operation_const_new());
        let default_parser = match wkt {
            Some(wkt) => Some(wkt.parser()),
            None => Some(ConfigParser::Group),
        };

        for attribute in &field.attrs {
            if matches!(attribute.style, AttrStyle::Outer)
                && let Meta::List(meta_list) = &attribute.meta
                && meta_list.path.is_ident("key")
            {
                let some_key = syn::parse2::<LitStr>(meta_list.tokens.clone())?;
                if some_key.value().is_empty() {
                    return Err(syn::Error::new(
                        attribute.span(),
                        "attribute key must be non-empty string",
                    ));
                }
                key = Some(some_key);
            } else if matches!(attribute.style, AttrStyle::Outer)
                && let Meta::List(meta_list) = &attribute.meta
                && meta_list.path.is_ident("default")
            {
                default = Some(syn::parse2::<Expr>(meta_list.tokens.clone())?);
                default_lazy_lock =
                    wkt.is_some_and(|wkc| !wkc.is_operation_const_new_with_default());
            } else if matches!(attribute.style, AttrStyle::Outer)
                && let Meta::List(meta_list) = &attribute.meta
                && meta_list.path.is_ident("lazy_lock")
            {
                let parsed = syn::parse2::<LitBool>(meta_list.tokens.clone())?;
                if let Some(last) = lazy_lock.as_ref() {
                    return Err(syn::Error::new(
                        attribute.span(),
                        format!("conflicts with earlier attribute lazy_lock({last})"),
                    ));
                }
                lazy_lock = Some(parsed.value);
            } else if matches!(attribute.style, AttrStyle::Outer)
                && let Meta::List(meta_list) = &attribute.meta
                && meta_list.path.is_ident("parse")
            {
                let parsed = syn::parse2::<Ident>(meta_list.tokens.clone())?;
                if parsed == "operation" {
                    parser = Some(ConfigParser::Operation)
                } else if parsed == "group" {
                    parser = Some(ConfigParser::Group)
                } else if parsed == "key" {
                    parser = Some(ConfigParser::GroupKey)
                } else {
                    return Err(syn::Error::new(
                        parsed.span(),
                        "invalid ast parser. Must be 'operation', 'group', or 'key'",
                    ));
                }
            } else {
                return Err(syn::Error::new(attribute.span(), "unknown attribute"));
            }
        }

        let Some(parser) = parser.or(default_parser) else {
            return Err(syn::Error::new(
                field.span(),
                "missing attribute 'value' or 'group'",
            ));
        };

        Ok(Self {
            key,
            default,
            lazy_lock: lazy_lock.unwrap_or(default_lazy_lock),
            parser,
        })
    }
}

enum ConfigParser {
    GroupKey,
    Operation,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WellKnownConfig {
    Value,
    Set,
    List,
    Acl,
}

impl WellKnownConfig {
    fn parse(field: &Field) -> Option<Self> {
        const WKC_MAP: &[(&str, WellKnownConfig)] = &[
            ("ConfigValue", WellKnownConfig::Value),
            ("ConfigSet", WellKnownConfig::Set),
            ("ConfigList", WellKnownConfig::List),
            ("ConfigAcl", WellKnownConfig::Acl),
        ];
        // TODO: improve accuracy to reduce false-positive rate
        if let Type::Path(TypePath { qself: None, path }) = &field.ty {
            return WKC_MAP
                .iter()
                .find(|(op, _)| {
                    path.segments
                        .last()
                        .is_some_and(|segment| segment.ident == op)
                })
                .map(|(_, wkc)| *wkc);
        }
        None
    }

    fn parser(&self) -> ConfigParser {
        ConfigParser::Operation
    }

    fn is_operation_const_new(&self) -> bool {
        matches!(self, Self::List | Self::Acl)
    }

    fn is_operation_const_new_with_default(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WellKnownType {
    Key,
    Operation(WellKnownConfig),
}

impl WellKnownType {
    fn parse(field: &Field) -> Option<Self> {
        const WKT_MAP: &[(&str, WellKnownType)] = &[("Bytes", WellKnownType::Key)];
        // TODO: improve accuracy to reduce false-positive rate
        if let Type::Path(TypePath { qself: None, path }) = &field.ty {
            return WKT_MAP
                .iter()
                .find(|(op, _)| {
                    path.segments
                        .last()
                        .is_some_and(|segment| segment.ident == op)
                })
                .map(|(_, wkc)| *wkc)
                .or_else(|| WellKnownConfig::parse(field).map(Self::Operation));
        }
        None
    }

    fn parser(&self) -> ConfigParser {
        match self {
            Self::Key => ConfigParser::GroupKey,
            Self::Operation(wkc) => wkc.parser(),
        }
    }

    fn is_operation_const_new(&self) -> bool {
        match self {
            // Although Bytes::new() is const, it is not an operation.
            Self::Key => false,
            Self::Operation(wkc) => wkc.is_operation_const_new(),
        }
    }

    fn is_operation_const_new_with_default(&self) -> bool {
        match self {
            Self::Key => false,
            Self::Operation(wkc) => wkc.is_operation_const_new_with_default(),
        }
    }
}
