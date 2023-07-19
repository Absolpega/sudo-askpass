use std::io::Write;

use inquire::{Confirm, CustomType, Text};

use crate::Config;

pub fn setup() {
    let mut config = Config::default();

    println!(
        r#"
    I recommend putting sudo-askpass into a folder that is in your PATH
        `cp target/release/sudo-askpass /usr/local/bin/sudo-askpass`
    In order to make sudo use sudo-askpass you need to
        `export SUDO_ASKPASS=/usr/local/bin/sudo-askpass`
    and launch sudo with `sudo -A`
        `alias sudo='sudo -A'`
    "#
    );

    config.secure = Confirm::new("Enable secure option?")
        .with_help_message(
            "When on will not show * for characters, spinner will only show when empty.",
        )
        .with_default(config.secure)
        .prompt()
        .unwrap();

    config.prompt.icons_ansi_color = CustomType::new("Color of spinner")
        .with_error_message("Must be a number")
        .with_help_message("https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit")
        .with_default(config.prompt.icons_ansi_color)
        .prompt()
        .unwrap();

    config.prompt.prompt_ansi_color = CustomType::new("Color of prompt")
        .with_error_message("Must be a number")
        .with_help_message("https://en.wikipedia.org/wiki/ANSI_escape_code#3-bit_and_4-bit")
        .with_default(config.prompt.prompt_ansi_color)
        .prompt()
        .unwrap();

    config.prompt.characters = Text::new("Spinner icons")
        .with_help_message("Can be separated by comma, space or not at all.")
        .with_default(
            config
                .prompt
                .characters
                .into_iter()
                .collect::<String>()
                .as_str(),
        )
        .prompt()
        .unwrap()
        .chars()
        .filter(|&c| c != ',' && c != ' ')
        .collect();

    config.prompt.empty = CustomType::new("Spinner icon for when input is empty")
        .with_error_message("Must be a single character")
        .with_help_message("Must be a single character")
        .with_default(config.prompt.empty)
        .prompt()
        .unwrap();

    config.prompt.secure = CustomType::new("Spinner icon for secure")
        .with_error_message("Must be a single character")
        .with_help_message("Must be a single character")
        .with_default(config.prompt.secure)
        .prompt()
        .unwrap();

    config.prompt.prompt_text = Text::new("The prompt itself")
        .with_help_message("Use $ to specify placement of spinner")
        .with_default(config.prompt.prompt_text.as_str())
        .prompt()
        .unwrap();

    let config_path = xdg::BaseDirectories::new()
        .unwrap()
        .place_config_file("sudo-askpass.yml")
        .unwrap();

    let mut config_file = std::fs::File::create(config_path).unwrap();
    write!(config_file, "{}", serde_yaml::to_string(&config).unwrap()).unwrap();
}
