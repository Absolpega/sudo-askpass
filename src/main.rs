mod ansi;
mod setup;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    prompt: Option<String>,

    #[arg(trailing_var_arg = true, num_args(0..))]
    _ingore_rest: Option<String>,

    #[arg(long, default_value_t = false)]
    setup: bool,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    secure: bool,
    prompt: Prompt,
}

/*
impl Default for Config {
    fn default() -> Self {
        Self {
            secure: false,
            prompt: Prompt::default(),
        }
    }
}
*/

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct Prompt {
    icons_ansi_color: u8,
    prompt_ansi_color: u8,
    characters: Vec<char>,
    empty: char,
    secure: char,
    prompt_text: String,
}

impl Default for Prompt {
    fn default() -> Self {
        //offset: prompt_str.chars().count() - prompt_str.chars().position(|c| c == '$').unwrap(),
        Self {
            icons_ansi_color: 36,
            prompt_ansi_color: 33,
            characters: MOON_SPINNER_CHARACTERS.to_vec(),
            empty: '󱃓',
            secure: '󰦝',
            prompt_text: "Enter password < $ > ".to_string(),
        }
    }
}

use termios::*;

use colored::*;

use read_char::read_next_char;

use std::fs::File;
use std::io::stdin;
use std::io::Write;
use std::iter::Cycle;
use std::os::fd::AsRawFd;

struct TermiosWrapper {
    orig_termios: Termios,
    termios: Termios,
}

impl TermiosWrapper {
    fn new() -> Self {
        let mut termios: Termios = Termios::from_fd(stdin().as_raw_fd()).unwrap();
        tcgetattr(stdin().as_raw_fd(), &mut termios).unwrap();
        Self {
            orig_termios: termios,
            termios,
        }
    }
    fn raw(&mut self) {
        tcgetattr(stdin().as_raw_fd(), &mut self.termios).unwrap();
        self.termios.c_lflag &= !(ICRNL | IXON);
        self.termios.c_lflag &= !(ECHO | ICANON | IEXTEN);
        tcsetattr(stdin().as_raw_fd(), TCSAFLUSH, &self.termios).unwrap();
    }
}

impl Drop for TermiosWrapper {
    fn drop(&mut self) {
        tcsetattr(stdin().as_raw_fd(), TCSAFLUSH, &self.orig_termios).unwrap();
    }
}

#[derive(PartialEq)]
enum SpinType {
    Empty,
    Forward,
    Backward,
    Secure,
}

fn spin<T>(way: SpinType, tty: &mut File, iter: &mut Cycle<T>, password: &str, prompt: &Prompt)
where
    T: Iterator<Item = char>,
    T: Clone,
{
    let spinner_offset = ansi::strip_ansi_codes(prompt.prompt_text.as_str())
        .chars()
        .count()
        - ansi::strip_ansi_codes(prompt.prompt_text.as_str())
            .chars()
            .position(|c| c == '$')
            .unwrap();

    let mut offset = spinner_offset + password.chars().count();

    if way == SpinType::Secure {
        offset = spinner_offset;
    }

    if way == SpinType::Backward {
        for _ in 1..prompt.characters.len() - 1 {
            iter.next();
        }
    }

    let some_icon: Option<char> = match way {
        SpinType::Secure => Some(prompt.secure),
        SpinType::Empty => Some(prompt.empty),
        _ => None,
    };

    write!(tty, "{}", ansi_escapes::CursorHide).unwrap();
    write!(tty, "{}", ansi_escapes::CursorSavePosition).unwrap();
    write!(
        tty,
        "{}",
        ansi_escapes::CursorBackward(offset.try_into().unwrap())
    )
    .unwrap();

    let mut icon = some_icon
        .unwrap_or_else(|| iter.next().unwrap())
        .to_string();

    icon.insert_str(0, format!("\x1B[{}m", prompt.icons_ansi_color).as_str());
    icon.push_str("\x1B[0m");

    write!(tty, "{}", icon).unwrap();

    write!(tty, "{}", ansi_escapes::CursorRestorePosition).unwrap();
    write!(tty, "{}", ansi_escapes::CursorShow).unwrap();
}

fn get_config() -> Option<Config> {
    let some_config_path = xdg::BaseDirectories::new()
        .unwrap()
        .find_config_file("sudo-askpass.yml")?;

    let some_config_string = std::fs::read_to_string(some_config_path);

    if some_config_string.is_err() {
        return None;
    }

    serde_yaml::from_str(some_config_string.unwrap().as_str()).ok()
}

fn main() {
    let args = Args::parse();

    if args.setup {
        setup::setup();
        return;
    }

    let some_config = get_config();

    let config = some_config.clone().unwrap_or_default();

    let secure = config.secure;

    colored::control::set_override(true);

    let mut tty = File::create("/dev/tty").unwrap();

    let mut termios_wrapper = TermiosWrapper::new();
    termios_wrapper.raw();

    let mut password: String = String::new();

    let spinner = config.prompt;

    let mut spinner_iter = spinner.characters.clone().into_iter().cycle();

    if some_config.is_none() {
        writeln!(
            tty,
            "sudo-askpass: {}",
            "Please create a configuration file with `sudo-askpass --setup`".red()
        )
        .unwrap();
    }

    let mut text = spinner.prompt_text.clone();

    if args.prompt.is_some() && !args.prompt.clone().unwrap_or_default().contains("sudo") {
        text = args.prompt.unwrap() + "< $ > ";
    }

    text.insert_str(0, format!("\x1B[{}m", spinner.prompt_ansi_color).as_str());
    text.push_str("\x1B[0m");

    write!(tty, "{}", text).unwrap();

    spin(
        SpinType::Empty,
        &mut tty,
        &mut spinner_iter,
        &password,
        &spinner,
    );

    loop {
        match read_next_char(&mut stdin()).unwrap() {
            '\n' => break,
            '\x7F' => {
                // backspace
                if password.pop().is_some() && !secure {
                    write!(tty, "{}", ansi_escapes::CursorBackward(1)).unwrap();
                    write!(tty, " ").unwrap();
                    write!(tty, "{}", ansi_escapes::CursorBackward(1)).unwrap();

                    spin(
                        SpinType::Backward,
                        &mut tty,
                        &mut spinner_iter,
                        &password,
                        &spinner,
                    );
                }

                if password.chars().last().is_none() {
                    spin(
                        SpinType::Empty,
                        &mut tty,
                        &mut spinner_iter,
                        &password,
                        &spinner,
                    );
                }
            }
            character => {
                password.push(character);

                if !secure {
                    write!(tty, "*").unwrap();

                    spin(
                        SpinType::Forward,
                        &mut tty,
                        &mut spinner_iter,
                        &password,
                        &spinner,
                    );
                } else {
                    spin(
                        SpinType::Secure,
                        &mut tty,
                        &mut spinner_iter,
                        &password,
                        &spinner,
                    );
                }
            }
        }
    }

    writeln!(tty).unwrap();

    println!("{}", password);
}

//const CLOCK_SPINNER_CHARACTERS: [char; 8] = ['󰪞', '󰪟', '󰪠', '󰪡', '󰪢', '󰪣', '󰪤', '󰪥'];

const MOON_SPINNER_CHARACTERS: [char; 24] = [
    '', // 6
    '', // 5
    '', // 4
    '', // 3
    '', // 2
    '', // 1
    '', // 6
    '', // 5
    '', // 4
    '', // 3
    '', // 2
    '', // 1
    '', // 6
    '', // 5
    '', // 4
    '', // 3
    '', // 2
    '', // 1
    '', // 6
    '', // 5
    '', // 4
    '', // 3
    '', // 2
    '', // 1
];
