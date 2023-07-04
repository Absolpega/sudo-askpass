use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = false)]
    secure: bool,
}

use termios::*;

use ansi_escapes;

use colorful::Colorful;
//use colorful::Color;

use read_char::read_next_char;

use std::fs::File;
use std::io::stdin;
use std::io::stdout;
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
        return Self {
            orig_termios: termios.clone(),
            termios,
        };
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

fn spin<T>(way: SpinType, tty: &mut File, iter: &mut Cycle<T>, password: &String, spinner: &Spinner)
where
    T: Iterator<Item = char>,
    T: Clone,
{
    let mut offset = spinner.offset + password.chars().count();

    if way == SpinType::Secure {
        offset = spinner.offset;
    }

    if way == SpinType::Backward {
        for _ in 1..spinner.characters.len() - 1 {
            iter.next();
        }
    }

    let some_icon: Option<char> = match way {
        SpinType::Secure => Some(spinner.secure),
        SpinType::Empty => Some(spinner.empty),
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

    write!(
        tty,
        "{}",
        some_icon
            .unwrap_or_else(|| iter.next().unwrap())
            .to_string()
            .cyan()
    )
    .unwrap();

    write!(tty, "{}", ansi_escapes::CursorRestorePosition).unwrap();
    write!(tty, "{}", ansi_escapes::CursorShow).unwrap();
}

fn main() {
    let mut tty = File::create("/dev/tty").unwrap();

    let args = Args::parse();

    let mut termios_wrapper = TermiosWrapper::new();
    termios_wrapper.raw();

    let mut password: String = String::new();

    let spinner = Spinner {
        characters: vec!['1', '2', '3', '4'],
        empty: 'e',
        secure: 'V',
        offset: 4,
    };

    let mut spinner_iter = spinner.characters.clone().into_iter().cycle();

    write!(tty, "{}", String::from("Enter password < S > ").yellow()).unwrap();

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
                if password.pop().is_some() && !args.secure {
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

                if !args.secure {
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

    write!(tty, "\n").unwrap();

    write!(stdout(), "{}\n", password).unwrap();
}

struct Spinner {
    characters: Vec<char>,
    empty: char,
    secure: char,
    offset: usize,
}

//const SPINNER_CHARACTERS: [&str; SPINNER_CHARACTERS_SIZE] = [
//    "󰪞",
//    "󰪟",
//    "󰪠",
//    "󰪡",
//    "󰪢",
//    "󰪣",
//    "󰪤",
//    "󰪥"
//];

//const SPINNER_CHARACTERS: [&str; 24] = [
//    "", // 6
//    "", // 5
//    "", // 4
//    "", // 3
//    "", // 2
//    "", // 1
//    "", // 6
//    "", // 5
//    "", // 4
//    "", // 3
//    "", // 2
//    "", // 1
//    "", // 6
//    "", // 5
//    "", // 4
//    "", // 3
//    "", // 2
//    "", // 1
//    "", // 6
//    "", // 5
//    "", // 4
//    "", // 3
//    "", // 2
//    "", // 1
//];
