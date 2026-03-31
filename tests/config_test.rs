use termpp::config::Config;
use std::io::Write;

fn write_temp_config(content: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(f, "{}", content).unwrap();
    f
}

#[test]
fn loads_valid_config() {
    let f = write_temp_config(r#"
        notification_timeout = 3
        font_size = 16
        theme = "dark"
    "#);
    let config = Config::load(f.path()).unwrap();
    assert_eq!(config.notification_timeout, 3);
    assert_eq!(config.font_size, 16);
}

#[test]
fn uses_defaults_when_fields_absent() {
    let f = write_temp_config("");
    let config = Config::load(f.path()).unwrap();
    assert_eq!(config.notification_timeout, 2);
    assert_eq!(config.font_size, 14);
    assert_eq!(config.theme, "dark");
}
