use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    AttrStyle, Attribute, Expr, Field, GenericArgument, Ident, LitBool, LitByteStr, LitStr, Meta,
    PathArguments, PathSegment, Stmt, Type, TypePath, Visibility, punctuated::Punctuated,
    spanned::Spanned, token::PathSep,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    GroupKey,
    Config,
    Group,
    AnyGroup,
    Flatten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WellKnownType {
    Key,
    ConfigValue,
    ConfigSet,
    ConfigList,
    ConfigAcl,
    Map,
}

impl WellKnownType {
    fn is_expected_leading_path(&self, segments: &Punctuated<PathSegment, PathSep>) -> bool {
        // The last segment is skipped since this function only validates the
        // path that comes before that last segment.
        let mut rev_segments = segments.iter().rev().skip(1);
        rev_segments
            .clone()
            .all(|segment| segment.arguments.is_empty())
            && match self {
                Self::Key
                | Self::ConfigValue
                | Self::ConfigSet
                | Self::ConfigList
                | Self::ConfigAcl => {
                    rev_segments
                        .next()
                        .is_none_or(|segment| segment.ident == "config")
                        && rev_segments.next().is_none()
                }
                Self::Map => {
                    rev_segments
                        .next()
                        .is_none_or(|segment| segment.ident == "collections")
                        && rev_segments
                            .next()
                            .is_none_or(|segment| segment.ident == "std")
                        && rev_segments.next().is_none()
                }
            }
    }

    fn is_expected_arguments(&self, arguments: &PathArguments) -> bool {
        match self {
            Self::Key => {
                // Key has no lifetimes or generics, so if any are present, this
                // is not ::config::Key.
                arguments.is_none()
            }
            Self::ConfigValue | Self::ConfigSet | Self::ConfigList | Self::ConfigAcl => {
                // The base config types all take 1 generic argument, `<T>`.
                let PathArguments::AngleBracketed(args) = arguments else {
                    return false;
                };
                if 1 != args.args.len() {
                    return false;
                }
                let first_arg = args.args.first().expect("length is 1");
                let GenericArgument::Type(_) = first_arg else {
                    return false;
                };
                // The base config types are expected to be of the form
                // `ConfigType<T>`. If the colon token were present, they would
                // be of the form `ConfigType::<T>` which I don't believe is
                // permitted in the context of a struct definition.
                args.colon2_token.is_none()
            }
            Self::Map => {
                // The base config types all take 1 generic argument, `<T>`.
                let PathArguments::AngleBracketed(args) = arguments else {
                    return false;
                };
                if 2 != args.args.len() {
                    return false;
                }
                let GenericArgument::Type(first_arg) = &args.args[0] else {
                    return false;
                };
                if !matches!(Self::parse_type(first_arg), Some(Self::Key)) {
                    return false;
                }
                let GenericArgument::Type(_) = &args.args[1] else {
                    return false;
                };
                // The HashMap type is expected to be of the form
                // `HashMap<Key, T>`. If the colon token were present, then it
                // would be of the form ``HashMap::<Key, T>` which I don't
                // believe is permitted in the context of a struct definition.
                args.colon2_token.is_none()
            }
        }
    }

    fn parse_type(ty: &Type) -> Option<Self> {
        const WKT_MAP: &[(&str, WellKnownType)] = &[
            ("Key", WellKnownType::Key),
            ("ConfigValue", WellKnownType::ConfigValue),
            ("ConfigSet", WellKnownType::ConfigSet),
            ("ConfigList", WellKnownType::ConfigList),
            ("ConfigAcl", WellKnownType::ConfigAcl),
            ("HashMap", WellKnownType::Map),
        ];
        let Type::Path(TypePath { qself: None, path }) = &ty else {
            return None;
        };
        let last_segment = path.segments.last()?;
        let wkt = WKT_MAP
            .iter()
            .find(|(op, _)| last_segment.ident == op)
            .map(|(_, wkc)| *wkc)?;
        if wkt.is_expected_leading_path(&path.segments)
            && wkt.is_expected_arguments(&last_segment.arguments)
        {
            Some(wkt)
        } else {
            None
        }
    }

    pub fn parse(field: &Field) -> Option<Self> {
        Self::parse_type(&field.ty)
    }

    pub fn parser(&self) -> FieldType {
        match self {
            Self::Key => FieldType::GroupKey,
            Self::ConfigValue => FieldType::Config,
            Self::ConfigSet => FieldType::Config,
            Self::ConfigList => FieldType::Config,
            Self::ConfigAcl => FieldType::Config,
            Self::Map => FieldType::AnyGroup,
        }
    }

    pub fn is_config_const_new(&self) -> bool {
        matches!(self, Self::ConfigList | Self::ConfigAcl)
    }

    pub fn is_config_const_new_with_default(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
struct ConfigKey {
    literal: String,
    span: Span,
}

impl ConfigKey {
    fn new(literal: impl ToString, span: Span) -> Self {
        Self {
            literal: literal.to_string(),
            span,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigStrKey<'a, 'b>(&'b ConfigField<'a>);

impl<'a, 'b> ConfigStrKey<'a, 'b> {
    pub fn ident(&self) -> Ident {
        format_ident!(
            "KEY_{}_{}",
            self.0.field_index,
            self.0.ident().to_string().to_uppercase()
        )
    }

    pub fn literal(&self) -> LitStr {
        LitStr::new(self.0.key.literal.as_str(), self.0.key.span)
    }

    // May be unused, defined for consistent internal API
    #[allow(unused)]
    pub fn statement_instantiate(&self) -> Stmt {
        let ident = self.ident();
        let literal = self.literal();
        let tokens = quote! {
            const #ident: &str = #literal;
        };
        syn::parse2(tokens).expect("must generate valid statements")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigBytesKey<'a, 'b>(&'b ConfigField<'a>);

impl<'a, 'b> ConfigBytesKey<'a, 'b> {
    pub fn ident(&self) -> Ident {
        format_ident!(
            "BYTES_KEY_{}_{}",
            self.0.field_index,
            self.0.ident().to_string().to_uppercase()
        )
    }

    pub fn literal(&self) -> LitByteStr {
        LitByteStr::new(self.0.key.literal.as_bytes(), self.0.key.span)
    }

    // May be unused, defined for consistent internal API
    #[allow(unused)]
    pub fn statement_instantiate(&self) -> Stmt {
        let ident = self.ident();
        let literal = self.literal();
        let tokens = quote! {
            const #ident: &[u8] = #literal;
        };
        syn::parse2(tokens).expect("must generate valid statements")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OperationDefault<'a, 'b>(&'b ConfigField<'a>);

impl<'a, 'b> OperationDefault<'a, 'b> {
    pub fn ident(&self) -> Ident {
        format_ident!("DEFAULT_{}", self.0.key.literal.to_uppercase())
    }

    pub fn expr_instantiate(&self) -> Expr {
        let ty = &self.0.field.ty;
        let literal_key = self.0.key_bytes().literal();
        let tokens = match &self.0.attributes.default {
            Some(default_expr) => quote! {
                <#ty>::new_with_default(
                    ::config::Key::from_static(#literal_key),
                    #default_expr
                )
            },
            None => quote! {
                <#ty>::new(
                    ::config::Key::from_static(#literal_key)
                )
            },
        };
        syn::parse2(tokens).expect("must generate valid expressions")
    }

    pub fn expr_copy_from_ident(&self) -> Expr {
        let ident = self.ident();
        let tokens = match self.0.attributes.lazy_lock {
            true => quote! {
                ::std::clone::Clone::clone(
                    ::std::ops::Deref::deref(&#ident)
                )
            },
            false => quote! {
                (#ident)
            },
        };
        syn::parse2(tokens).expect("must generate valid expressions")
    }

    pub fn statement_instantiate(&self) -> Stmt {
        let ty = &self.0.field.ty;
        let ident = self.ident();
        let instantiate = self.expr_instantiate();
        let tokens = match self.0.attributes.lazy_lock {
            true => quote! {
                static #ident: ::std::sync::LazyLock<#ty> =
                    ::std::sync::LazyLock::new(|| #instantiate);
            },
            false => quote! {
                const #ident: #ty = #instantiate;
            },
        };
        syn::parse2(tokens).expect("must generate valid statements")
    }
}

#[derive(Debug, Clone)]
struct ConfigFieldAttributes {
    key: Option<LitStr>,
    default: Option<Expr>,
    lazy_lock: bool,
    parser: FieldType,
}

impl ConfigFieldAttributes {
    pub fn parse(field: &Field) -> Result<Self, syn::Error> {
        let wkt = WellKnownType::parse(field);

        let mut key = None;
        let mut default = None;
        let mut lazy_lock = None;
        let mut parser: Option<FieldType> = None;

        let mut default_lazy_lock = wkt.is_some_and(|wkc| !wkc.is_config_const_new());
        let default_parser = match wkt {
            Some(wkt) => Some(wkt.parser()),
            None => Some(FieldType::Group),
        };

        for attribute in &field.attrs {
            if matches!(attribute.style, AttrStyle::Outer)
                && let Meta::Path(path) = &attribute.meta
                && path.is_ident("group_key")
            {
                parser = Some(FieldType::GroupKey);
            } else if matches!(attribute.style, AttrStyle::Outer)
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
                default_lazy_lock = wkt.is_some_and(|wkc| !wkc.is_config_const_new_with_default());
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
                const FIELD_TYPE_MAP: &[(&str, FieldType)] = &[
                    ("config", FieldType::Config),
                    ("group", FieldType::Group),
                    ("any_group", FieldType::AnyGroup),
                    ("flatten", FieldType::Flatten),
                    ("key", FieldType::GroupKey),
                ];
                for (name, ft) in FIELD_TYPE_MAP {
                    if parsed == name {
                        parser = Some(*ft);
                        break;
                    }
                }
                if parser.is_none() {
                    return Err(syn::Error::new(
                        parsed.span(),
                        format!(
                            "invalid ast parser. Must be {}",
                            std::fmt::from_fn(|f| {
                                let mut names = FIELD_TYPE_MAP.iter().map(|(name, _)| name);
                                if let Some(first) = names.next() {
                                    write!(f, "'{first}'")?;
                                }
                                for name in names {
                                    write!(f, ", '{name}'")?;
                                }
                                Ok(())
                            })
                        ),
                    ));
                }
            } else {
                return Err(syn::Error::new(attribute.span(), "unknown attribute"));
            }
        }

        let Some(parser) = parser.or(default_parser) else {
            return Err(syn::Error::new(field.span(), "missing attribute 'parse'"));
        };

        Ok(Self {
            key,
            default,
            lazy_lock: lazy_lock.unwrap_or(default_lazy_lock),
            parser,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConfigField<'a> {
    attributes: ConfigFieldAttributes,
    key: ConfigKey,
    field: &'a Field,
    field_index: usize,
}

impl<'a> ConfigField<'a> {
    pub fn parse(field: &'a Field, field_index: usize) -> Result<Self, syn::Error> {
        let attributes = ConfigFieldAttributes::parse(field)?;
        let key = match (&field.ident, &attributes.key) {
            (_, Some(key)) => ConfigKey::new(key.value(), key.span()),
            (Some(key), None) => ConfigKey::new(key.to_string().to_uppercase(), key.span()),
            (None, None) => {
                return Err(syn::Error::new(
                    field.span(),
                    "missing attribute 'key' for unnamed field",
                ));
            }
        };
        Ok(Self {
            attributes: ConfigFieldAttributes::parse(field)?,
            field,
            field_index,
            key,
        })
    }

    pub fn field_type(&self) -> FieldType {
        self.attributes.parser
    }

    pub fn ident(&self) -> Ident {
        match &self.field.ident {
            Some(ident) => ident.clone(),
            None => format_ident!("{}", self.field_index),
        }
    }

    // May be unused, defined for consistent internal API
    #[allow(unused)]
    pub fn attributes(&self) -> &[Attribute] {
        &self.field.attrs
    }

    // May be unused, defined for consistent internal API
    #[allow(unused)]
    pub fn vis(&self) -> &Visibility {
        &self.field.vis
    }

    pub fn ty(&self) -> &Type {
        &self.field.ty
    }

    pub fn default(&self) -> OperationDefault<'a, '_> {
        OperationDefault(self)
    }

    pub fn key_str(&self) -> ConfigStrKey<'a, '_> {
        ConfigStrKey(self)
    }

    pub fn key_bytes(&self) -> ConfigBytesKey<'a, '_> {
        ConfigBytesKey(self)
    }

    pub fn span(&self) -> Span {
        self.field.span()
    }
}
