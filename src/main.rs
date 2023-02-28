use std::os::unix::io::AsRawFd;

use std::array::IntoIter;
use std::iter::Cycle;

use std::fs::File;

use termios::*;
//use std::io;
use std::io::{Write, Read, stdin, stdout};

use ansi_escapes;

use colorful::Colorful;
//use colorful::Color;

// always at least 1
const SPINNER_OFFSET: usize = 4;

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

const SPINNER_CHARACTERS_SIZE: usize = 4;
const SPINNER_CHARACTERS: [&str; SPINNER_CHARACTERS_SIZE] = [
    "◜",
    "◝",
    "◞",
    "◟"
];

const SPINNER_CHARACTER_STOP: &str = "◉";
const SPINNER_CHARACTER_EMPTY: &str = "○";
const SPINNER_CHARACTER_SECURE: &str = "○";

#[derive(PartialEq)]
enum SpinnerWays {
    Empty,
    Stop,
    Forward,
    Backward,
    Secure,
}

fn spinner<'a>(tty: &mut File, way: SpinnerWays, iter: &mut Cycle<IntoIter<&str, SPINNER_CHARACTERS_SIZE>>, mut left_offset: u16) {
    // due to the -1
    // should just never enter a value that == 1
    if left_offset == 0 {
        left_offset = 1;
    }

    if way == SpinnerWays::Backward {
        for _ in 1..=SPINNER_CHARACTERS_SIZE-2 {
            iter.next();
        }
    }

    let mut next = iter.next();

    match way {
        SpinnerWays::Stop => {
            next = Some(SPINNER_CHARACTER_STOP);
        }
        SpinnerWays::Empty => {
            next = Some(SPINNER_CHARACTER_EMPTY);
        }
        SpinnerWays::Secure => {
            next = Some(SPINNER_CHARACTER_SECURE);
        }
        _ => {}
    }

    if next.is_some() {
        write!(tty, "{}", ansi_escapes::CursorHide).unwrap();
        write!(tty, "{}", ansi_escapes::CursorBackward(left_offset)).unwrap();

        // unless SPINNER_CHARACTERS is empty
        write!(tty, "{}", next.expect("should never panic").cyan()).unwrap();

        write!(tty, "{}", ansi_escapes::CursorForward(left_offset-1)).unwrap();
        write!(tty, "{}", ansi_escapes::CursorShow).unwrap();
    }
}

fn read(mut tty: File, secure: bool) -> String {
    let mut output = String::new();

    let mut spinner_characters_iter = SPINNER_CHARACTERS.into_iter().cycle();
    spinner(&mut tty, SpinnerWays::Empty, &mut spinner_characters_iter, (SPINNER_OFFSET + output.len()).try_into().unwrap());

    let mut c = stdin().bytes().next().unwrap().unwrap() as char;

    while c != '\n' {
        loop {
            match c {
                // \b is not allowed for some reason
                // also it returns 127 (7F) on backspace for some reason
                '\x08' | '\x7F' => {
                    if secure {
                        output.pop();
                        if output.bytes().last() == None {
                            spinner(&mut tty, SpinnerWays::Stop, &mut spinner_characters_iter, (SPINNER_OFFSET).try_into().unwrap());
                        }
                        break;
                    }

                    if output.len() > 0 {
                        if output.pop() != None {
                            write!(tty, "{}", ansi_escapes::CursorBackward(1)).unwrap();
                            write!(tty, " ").unwrap();
                            write!(tty, "{}", ansi_escapes::CursorBackward(1)).unwrap();
                            spinner(&mut tty, SpinnerWays::Backward, &mut spinner_characters_iter, (SPINNER_OFFSET + output.len()).try_into().unwrap());
                        }
                    }

                    if output.bytes().last() == None {
                        spinner(&mut tty, SpinnerWays::Stop, &mut spinner_characters_iter, (SPINNER_OFFSET + output.len()).try_into().unwrap());
                    }
                }
                c => {
                    if !secure {
                        spinner(&mut tty, SpinnerWays::Forward, &mut spinner_characters_iter, (SPINNER_OFFSET + output.len()).try_into().unwrap());
                        write!(tty, "*").unwrap();
                    } else {
                        spinner(&mut tty, SpinnerWays::Secure, &mut spinner_characters_iter, (SPINNER_OFFSET).try_into().unwrap());
                    }
                    output.push(c);
                }
            }
            break;
        }
        c = stdin().bytes().next().unwrap().unwrap() as char;
    }

    if !secure {
        spinner(&mut tty, SpinnerWays::Empty, &mut spinner_characters_iter, (SPINNER_OFFSET + output.len()).try_into().unwrap());
    }

    write!(tty, "\n").unwrap();

    return output;
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
    let string = read(tty, secure);

    write!(stdout(), "{}\n", string).unwrap();

    end(orig_termios);
}
