extern crate termion;

use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use termion::event::Key;
use termion::input::{TermRead, Keys};
use termion::{clear, cursor};
use termion::terminal_size;

use std::io::{Write, stdout, stdin, stderr, Stdout, Stdin};
use std::error::Error;
use std::process;
// use std::{thread, time};

struct Row {
    chars: String,
    render: String,
}

impl Row {
    fn new(chars: String) -> Row {
        let render = chars.clone();
        Row {
            chars,
            render
        }
    }

    fn update(&mut self) {
        let mut tabs = 0;
        for ch in self.chars.chars() {
            if ch == '\t' {
                tabs += 1;
            }
        }

        self.render.clear();
        let mut idx = 0;
        let mut ch = self.chars.chars();

        for character in ch {
            // let character = ch.next().unwrap();
            if character == '\t' {
                self.render.push(' ');
                idx += 1;
                while idx % 8 != 0 {
                    self.render.push(' ');
                    idx += 1;
                }
            } else {
                self.render.push(character);
                idx += 1;
            }
        }
        println!("test");
    }

    fn print(&mut self) {
        writeln!(stdout(), "len: {}, {}, repr: {}, {}", self.chars.len(), self.render.len(), self.chars, self.render).unwrap();
    }
}

struct Editor {
    cx: u32,
    cy: u32,
    rx: u32,
    rowoff: i32,
    coloff: i32,
    screenrows: u16,
    screencols: u16,
    numrows: i32,
    erow: Vec<Row>,
    screen: AlternateScreen<RawTerminal<Stdout>>,
    stdin: Keys<Stdin>,
}

impl Editor {
    fn new() -> Editor {
        let stdout = stdout().into_raw_mode().unwrap();
        let screen = AlternateScreen::from(stdout);
        let (screencols, screenrows) = terminal_size().unwrap();
        Editor {
            cx:0,
            cy:0,
            rx:0,
            rowoff:0,
            coloff:0,
            screenrows,
            screencols,
            numrows:0,
            erow:Vec::new(),
            screen,
            stdin:stdin().keys(),
        }
    }

    fn write_char(&mut self, c: char) {
        write!(self.screen, "{}", c).unwrap();
        self.screen.flush().unwrap();
    }

    fn write(&mut self, string: &str) {
        write!(self.screen, "{}", string).unwrap();
        self.screen.flush().unwrap();
    }

    fn clear(&mut self) {
        write!(self.screen, "{}{}", clear::All, cursor::Goto(1,1)).unwrap();
        self.screen.flush().unwrap();
    }

    fn refresh(&mut self) {
        let mut buffer = String::new();
        writeln!(stderr(), "test").unwrap();
        for y in 0..self.screenrows {
            if self.numrows == 0 && y == self.screenrows / 3 {
                let welcome = "Kilo editor for Rust -- version 0.0.1";
                let welcome_len = welcome.len() as u16;
                writeln!(stderr(), "{}, {}", self.screencols, welcome_len);
                let mut padding = (self.screencols - welcome_len) / 2;
                if padding > 0 {
                    buffer.push('~');
                    padding -= 1;
                }
                for _ in 0..padding {
                    buffer.push(' ');
                }
                buffer.push_str(welcome);
            } else {
                buffer.push('~');
            }
            buffer.push_str("\r\n");
        }
        self.clear();
        self.write(buffer.as_str());
    }

    fn process_keypress(&mut self) -> Result<i32, i32> {
        let c = self.stdin.next().unwrap().unwrap();
        match c {
            Key::Ctrl('q') => return Err(1),
            Key::Char(ch) => self.write_char(ch),
            _ => {}
        }
        return Ok(0)
    }
}

fn init_editor() {
    let stdin = stdin();
    let mut ret = Ok(1);
    {
        let mut editor = Editor::new();
        editor.clear();
        editor.write("Hey there, how are you");
        while let Ok(_) = ret {
            editor.refresh();
            ret = editor.process_keypress();
        }
    }
    println!("{:?}", ret);
    println!("bye !");
}

fn main() {
    init_editor();
    return;
}
