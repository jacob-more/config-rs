use config::derive;
use config::{ConfigExt, ConfigValue};

#[derive(Debug, Clone, derive::Config, derive::ConfigGroup, derive::ConfigDefault)]
#[exhaustive]
pub struct Locale {
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

#[derive(Debug, Clone, derive::Config, derive::ConfigGroup, derive::ConfigDefault)]
pub struct OsRelease {
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

#[derive(Debug, Clone, derive::Config, derive::ConfigDefault)]
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
    for config in [
        &os_release.name,
        &os_release.id,
        &os_release.id_like,
        &os_release.pretty_name,
        &os_release.ansi_color,
        &os_release.home_url,
        &os_release.documentation_url,
        &os_release.support_url,
        &os_release.bug_report_url,
        &os_release.privacy_policy_url,
    ] {
        println!("    {config}");
    }

    println!("{LOCALE_PATH}");
    for config in [
        &locale.lang,
        &locale.lc_address,
        &locale.lc_identification,
        &locale.lc_measurement,
        &locale.lc_monetary,
        &locale.lc_name,
        &locale.lc_numeric,
        &locale.lc_paper,
        &locale.lc_telephone,
        &locale.lc_time,
    ] {
        println!("    {config}");
    }
}
