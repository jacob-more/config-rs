use config::derive;
use config::{ConfigExt, ConfigValue};

#[derive(
    Debug, Clone, derive::Config, derive::ConfigGroup, derive::ConfigDefault, derive::ConfigDisplay,
)]
#[exhaustive]
pub struct Locale {
    #[key("LOCALE")]
    pub group_key: bytes::Bytes,
    pub lang: ConfigValue<Option<&'static str>>,
    pub lc_address: ConfigValue<Option<&'static str>>,
    pub lc_identification: ConfigValue<Option<&'static str>>,
    pub lc_measurement: ConfigValue<Option<&'static str>>,
    pub lc_monetary: ConfigValue<Option<&'static str>>,
    pub lc_name: ConfigValue<Option<&'static str>>,
    pub lc_numeric: ConfigValue<Option<&'static str>>,
    pub lc_paper: ConfigValue<Option<&'static str>>,
    pub lc_telephone: ConfigValue<Option<&'static str>>,
    pub lc_time: ConfigValue<Option<&'static str>>,
}

#[derive(
    Debug, Clone, derive::Config, derive::ConfigGroup, derive::ConfigDefault, derive::ConfigDisplay,
)]
pub struct OsRelease {
    #[key("OS_RELEASE")]
    pub group_key: bytes::Bytes,
    pub name: ConfigValue<Option<&'static str>>,
    pub id: ConfigValue<Option<&'static str>>,
    pub id_like: ConfigValue<Option<&'static str>>,
    pub pretty_name: ConfigValue<Option<&'static str>>,
    pub ansi_color: ConfigValue<Option<&'static str>>,
    pub home_url: ConfigValue<Option<&'static str>>,
    pub documentation_url: ConfigValue<Option<&'static str>>,
    pub support_url: ConfigValue<Option<&'static str>>,
    pub bug_report_url: ConfigValue<Option<&'static str>>,
    pub privacy_policy_url: ConfigValue<Option<&'static str>>,
}

#[derive(Debug, Clone, derive::Config, derive::ConfigDefault, derive::ConfigDisplay)]
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
