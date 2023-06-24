use std::os::unix::io::AsRawFd;

use std::array::IntoIter;
use std::iter::Cycle;

use std::fs::File;

use termios::*;

use std::io::{Write, Read, stdin, stdout};

use ansi_escapes;

use colorful::Colorful;
//use colorful::Color;

use read_char::read_next_char;

fn end(orig_termios: Termios) {
    tcsetattr(stdin().as_raw_fd(), TCSAFLUSH, &orig_termios).unwrap();
}

fn termios_init() -> Termios {
    let mut termios: Termios = Termios::from_fd(stdin().as_raw_fd()).unwrap();
    tcgetattr(stdin().as_raw_fd(), &mut termios).unwrap();
    return termios;
}

fn raw(raw: &mut Termios) {
    tcgetattr(stdin().as_raw_fd(), raw).unwrap();
	raw.c_lflag &= !(ICRNL | IXON);
	raw.c_lflag &= !(ECHO | ICANON | IEXTEN);
    tcsetattr(stdin().as_raw_fd(), TCSAFLUSH, &raw).unwrap();
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

const SPINNER_CHARACTERS: [&str; 24] = [
    "", // 6
    "", // 5
    "", // 4
    "", // 3
    "", // 2
    "", // 1
    "", // 6
    "", // 5
    "", // 4
    "", // 3
    "", // 2
    "", // 1
    "", // 6
    "", // 5
    "", // 4
    "", // 3
    "", // 2
    "", // 1
    "", // 6
    "", // 5
    "", // 4
    "", // 3
    "", // 2
    "", // 1
];

struct Spinner<'a> {
     characters: Vec<&'a str>,
     empty: &'a str,
     secure: &'a str,
     offset: usize,
}

#[derive(PartialEq)]
enum SpinType {
    Empty,
    Forward,
    Backward,
    Secure,
}

fn spin<'a, T>(
    way: SpinType,
    tty: &mut File,
    iter: &mut Cycle<T>,
    output: &Vec<String>,
    spinner: &Spinner,
    ) where T: Clone, T: Iterator<Item = &'a str> {

    //let mut left_offset = (spinner.offset + output.chars().collect::<Vec<_>>().len()).try_into().unwrap();
    let mut left_offset = (spinner.offset + output.len()).try_into().unwrap();

    // due to the -1
    // should just never enter a value that == 1
    if left_offset == 0 {
        left_offset = 1;
    }

    if way == SpinType::Backward {
        for _ in 1..spinner.characters.len()-1 {
            iter.next();
        }
    }

    let mut next: Option<&str> = iter.next();

    match way {
        SpinType::Empty => {
            next = Some(spinner.empty);
        }
        SpinType::Secure => {
            next = Some(spinner.secure);
        }
        _ => {}
    }

    if next.is_some() {
        write!(tty, "{}", ansi_escapes::CursorHide).unwrap();
        write!(tty, "{}", ansi_escapes::CursorSavePosition).unwrap();
        write!(tty, "{}", ansi_escapes::CursorBackward(left_offset)).unwrap();

        write!(tty, "{}", next.unwrap().cyan()).unwrap();

        write!(tty, "{}", ansi_escapes::CursorRestorePosition).unwrap();
        write!(tty, "{}", ansi_escapes::CursorShow).unwrap();
    }
}

fn read(
    mut tty: File,
    secure: bool,
    spinner: &Spinner,
    ) -> String {
    let mut output: Vec<String> = vec!();

    let mut spinner_characters_iter = spinner.characters.clone().into_iter().cycle();
    spin(SpinType::Empty, &mut tty, &mut spinner_characters_iter, &output, spinner);

    loop {
        //match (stdin().bytes().next().unwrap().unwrap() as char).to_string().as_str() {
        match read_next_char(&mut stdin()).unwrap().to_string().as_str() {
            "\n" => break,
            "\x7F" => {
                // it returns 127 (7F) on backspace for some reason
                
                if !secure && output.pop() != None {
                    write!(tty, "{}", ansi_escapes::CursorBackward(1)).unwrap();
                    write!(tty, " ").unwrap();
                    write!(tty, "{}", ansi_escapes::CursorBackward(1)).unwrap();

                    spin(SpinType::Backward, &mut tty, &mut spinner_characters_iter, &output, spinner);
                }

                if output.last() == None {
                    spin(SpinType::Empty, &mut tty, &mut spinner_characters_iter, &output, spinner);
                }

            }
            character => {
                if !secure {
                    spin(SpinType::Forward, &mut tty, &mut spinner_characters_iter, &output, spinner);
                    write!(tty, "*").unwrap();
                } else {
                    spin(SpinType::Secure, &mut tty, &mut spinner_characters_iter, &vec!("".to_string()), spinner);
                }
                output.push(character.to_string());
            }
        }
    }

    if !secure {
        spin(SpinType::Empty, &mut tty, &mut spinner_characters_iter, &output, spinner);
    }

    write!(tty, "\n").unwrap();

    return output.join("");
}

fn main() {
    let mut tty = File::create("/dev/tty").unwrap();

    let orig_termios = termios_init();

    let mut termios: Termios = termios_init();
    raw(&mut termios);

    write!(tty, "{}", String::from("Enter password < S > ").yellow()).unwrap();

    let mut secure: bool = false;
    if std::env::args().nth(1) == Some("--secure".to_string()) {
        secure = true;
    }

    let spinner = Spinner {
        characters: SPINNER_CHARACTERS.to_vec(),
        empty: "󱃓",
        secure: "󰳌",
        offset: 4,
    };

    let string = read(tty, secure, &spinner);

    write!(stdout(), "{}\n", string).unwrap();

    end(orig_termios);
}
