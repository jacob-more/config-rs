use config::{ConfigExt, Cval, collections::ConfigValue};
use config::{Key, derive};

#[derive(
    Debug, Clone, derive::Config, derive::ConfigGroup, derive::ConfigDefault, derive::ConfigDisplay,
)]
#[exhaustive]
pub struct Locale {
    #[key("LOCALE")]
    pub group_key: Key,
    pub lang: ConfigValue<Option<Cval<str>>>,
    pub lc_address: ConfigValue<Option<Cval<str>>>,
    pub lc_identification: ConfigValue<Option<Cval<str>>>,
    pub lc_measurement: ConfigValue<Option<Cval<str>>>,
    pub lc_monetary: ConfigValue<Option<Cval<str>>>,
    pub lc_name: ConfigValue<Option<Cval<str>>>,
    pub lc_numeric: ConfigValue<Option<Cval<str>>>,
    pub lc_paper: ConfigValue<Option<Cval<str>>>,
    pub lc_telephone: ConfigValue<Option<Cval<str>>>,
    pub lc_time: ConfigValue<Option<Cval<str>>>,
}

#[derive(
    Debug, Clone, derive::Config, derive::ConfigGroup, derive::ConfigDefault, derive::ConfigDisplay,
)]
pub struct OsRelease {
    #[key("OS_RELEASE")]
    pub group_key: Key,
    pub name: ConfigValue<Option<Cval<str>>>,
    pub id: ConfigValue<Option<Cval<str>>>,
    pub id_like: ConfigValue<Option<Cval<str>>>,
    pub pretty_name: ConfigValue<Option<Cval<str>>>,
    pub ansi_color: ConfigValue<Option<Cval<str>>>,
    pub home_url: ConfigValue<Option<Cval<str>>>,
    pub documentation_url: ConfigValue<Option<Cval<str>>>,
    pub support_url: ConfigValue<Option<Cval<str>>>,
    pub bug_report_url: ConfigValue<Option<Cval<str>>>,
    pub privacy_policy_url: ConfigValue<Option<Cval<str>>>,
}

#[derive(
    Debug, Clone, derive::Config, derive::ConfigGroup, derive::ConfigDefault, derive::ConfigDisplay,
)]
pub struct SystemInfo {
    pub os_release: OsRelease,
    pub locale: Locale,
}

fn main() {
    const OS_RELEASE_PATH: &str = "/etc/os-release";
    const LOCALE_PATH: &str = "/etc/locale.conf";

    let os_release = OsRelease::from_file(OS_RELEASE_PATH)
        .expect(&format!("example requires {OS_RELEASE_PATH} to exist, be readable, and have valid syntax although not all keys need to be present"));
    let locale = Locale::from_file(LOCALE_PATH).expect(&format!(
        "example requires {LOCALE_PATH} to exist, be readable, and have exact syntax"
    ));

    println!("{OS_RELEASE_PATH}");
    println!("{os_release}");

    println!("{LOCALE_PATH}");
    println!("{locale}");
}
