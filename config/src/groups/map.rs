use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
};

use crate::{ConfigFmt, ConfigGroup, ConfigParseError, Key, ext::IterJoin, parse::RawEntry};

macro_rules! impl_config_group_for_map {
    ($map:ident) => {
        impl<C> ConfigGroup for $map<Key, C>
        where
            C: ConfigGroup<Err = ConfigParseError>,
        {
            type Err = ConfigParseError;

            fn parse_entry(&mut self, entry: RawEntry) -> Result<(), Self::Err> {
                match entry {
                    RawEntry::Group { key, body } => {
                        self.entry(Key::from(key))
                            .or_insert_with(|| Default::default())
                            .parse(body)?;
                    }
                    RawEntry::Collection { key, body } => {
                        return Err(ConfigParseError::UnknownCollectionKey(
                            RawEntry::Collection { key, body },
                        ));
                    }
                }
                Ok(())
            }

            fn replay(&mut self, other: &Self) {
                for (key, group) in other.iter() {
                    self.entry(key.clone())
                        .or_insert_with(|| Default::default())
                        .replay(group);
                }
            }

            fn display(&self, fmt: ConfigFmt) -> impl Display {
                std::fmt::from_fn(move |f| {
                    write!(
                        f,
                        "{}",
                        self.iter()
                            .map(|(key, group)| group.display(fmt.clone().with_key(key)))
                            .join('\n')
                    )
                })
            }
        }
    };
}

impl_config_group_for_map!(HashMap);
impl_config_group_for_map!(BTreeMap);
