use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    AttrStyle, Attribute, Expr, Field, Ident, LitBool, LitByteStr, LitStr, Meta, Stmt, Type,
    TypePath, Visibility, spanned::Spanned,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    GroupKey,
    Config,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WellKnownType {
    Key,
    ConfigValue,
    ConfigSet,
    ConfigList,
    ConfigAcl,
}

impl WellKnownType {
    pub fn parse(field: &Field) -> Option<Self> {
        const WKT_MAP: &[(&str, WellKnownType)] = &[
            ("Bytes", WellKnownType::Key),
            ("ConfigValue", WellKnownType::ConfigValue),
            ("ConfigSet", WellKnownType::ConfigSet),
            ("ConfigList", WellKnownType::ConfigList),
            ("ConfigAcl", WellKnownType::ConfigAcl),
        ];
        // TODO: to improve accuracy and reduce false-positive rate, checks all
        //       segments in the path that are present and count the number of
        //       generic arguments.
        if let Type::Path(TypePath { qself: None, path }) = &field.ty {
            return WKT_MAP
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

    pub fn parser(&self) -> FieldType {
        match self {
            Self::Key => FieldType::GroupKey,
            Self::ConfigValue => FieldType::Config,
            Self::ConfigSet => FieldType::Config,
            Self::ConfigList => FieldType::Config,
            Self::ConfigAcl => FieldType::Config,
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
    // May be unused, defined for consistent internal API
    #[allow(unused)]
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
        let literal_key = self.0.key_str().literal();
        let tokens = match &self.0.attributes.default {
            Some(default_expr) => quote! {
                <#ty>::new_with_default(#literal_key, #default_expr)
            },
            None => quote! {
                <#ty>::new(#literal_key)
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
}
