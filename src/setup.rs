use std::io::Write;

use inquire::Confirm;

use crate::Config;

pub fn setup() {
    let mut config = Config { secure: false };

    config.secure = !Confirm::new("Show * for characters when prompting?")
        .with_default(!config.secure)
        .prompt()
        .unwrap();

    let config_path = xdg::BaseDirectories::new()
        .unwrap()
        .place_config_file("sudo-askpass.yml")
        .unwrap();

    let mut config_file = std::fs::File::create(config_path).unwrap();
    write!(config_file, "{}", serde_yaml::to_string(&config).unwrap()).unwrap();
}
