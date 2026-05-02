use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
};

use crate::{
    Config, ConfigFmt, ConfigGroup, ConfigParseError, ConfigParseGroupError, Key,
    ext::IterJoin,
    parse::{RawEntry, RawGroup},
};

macro_rules! impl_config_for_map {
    ($map:ident) => {
        impl<C> Config for $map<Key, C>
        where
            C: ConfigGroup<Err = ConfigParseError>,
        {
            type Err = ConfigParseError;

            fn parse_entry(&mut self, entry: RawEntry) -> Result<(), Self::Err> {
                match entry {
                    RawEntry::Group { key, body } => {
                        let key = Key::from(key);
                        self.entry(key.clone())
                            .or_insert_with(|| ConfigGroup::new(key.clone()))
                            .parse_group(key, body)?;
                    }
                    RawEntry::Operation { key, body } => {
                        return Err(ConfigParseError::UnknownOperationKey(RawEntry::Operation {
                            key,
                            body,
                        }));
                    }
                }
                Ok(())
            }

            fn replay(&mut self, other: &Self) {
                for (key, group) in other.iter() {
                    self.entry(key.clone())
                        .or_insert_with(|| ConfigGroup::new(key.clone()))
                        .replay(group);
                }
            }

            fn display(&self, fmt: ConfigFmt) -> impl Display {
                std::fmt::from_fn(move |f| {
                    write!(
                        f,
                        "{}",
                        self.values()
                            .map(|group| group.display(fmt.clone()))
                            .join('\n')
                    )
                })
            }
        }

        impl<C> ConfigGroup for $map<Key, C>
        where
            C: ConfigGroup<Err = ConfigParseGroupError>,
        {
            type Err = ConfigParseGroupError;

            fn new(_key: Key) -> Self {
                Self::default()
            }

            fn parse_group(&mut self, key: Key, body: RawGroup) -> Result<(), Self::Err> {
                self.parse_entry(
                    &key,
                    RawEntry::Group {
                        key: key.clone().into_bytes(),
                        body,
                    },
                )
            }

            fn parse_entry(&mut self, key: &Key, entry: RawEntry) -> Result<(), Self::Err> {
                let parent_group = key;

                match entry {
                    RawEntry::Group { key, body } => {
                        let key = Key::from(key);
                        self.entry(key.clone())
                            .or_insert_with(|| ConfigGroup::new(key.clone()))
                            .parse_group(key, body)?;
                    }
                    RawEntry::Operation { key, body } => {
                        return Err(ConfigParseGroupError::UnknownOperationKey {
                            group: parent_group.clone().into_bytes(),
                            entry: RawEntry::Operation { key, body },
                        });
                    }
                }
                Ok(())
            }

            fn replay(&mut self, other: &Self) {
                for (key, group) in other.iter() {
                    self.entry(key.clone())
                        .or_insert_with(|| ConfigGroup::new(key.clone()))
                        .replay(group);
                }
            }

            fn display(&self, fmt: ConfigFmt) -> impl Display {
                std::fmt::from_fn(move |f| {
                    write!(
                        f,
                        "{}",
                        self.values()
                            .map(|group| group.display(fmt.clone()))
                            .join('\n')
                    )
                })
            }
        }
    };
}

impl_config_for_map!(HashMap);
impl_config_for_map!(BTreeMap);
