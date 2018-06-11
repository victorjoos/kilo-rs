extern crate termion;

use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use termion::event::Key;
use termion::input::{TermRead, Keys};
use termion::{clear, cursor, color, style};
use termion::terminal_size;
use std::fs::File;

use std::io::{Write, stdout, stdin, stderr, Stdout, Stdin, BufReader, BufRead};
use std::error::Error;
use std::{process, env};
use std::time::SystemTime;
// use std::{thread, time};
const KILO_TAB_STOP:usize = 8;
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
                while idx % KILO_TAB_STOP != 0 {
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
    cx: usize,
    cy: usize,
    rx: usize,
    rowoff: usize,
    coloff: usize,
    screenrows: u16,
    screencols: u16,
    rows: Vec<Row>,
    dirty: bool,
    filename: Option<String>,
    status_message: Option<(String, SystemTime)>,
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
            screenrows:screenrows-2,
            screencols,
            rows:Vec::new(),
            screen,
            dirty:false,
            filename:None,
            status_message: None,
            stdin:stdin().keys(),
        }
    }

    fn read_file(&mut self, filename: String) {
        self.filename = Some(filename.clone());
        let file = File::open(&filename).unwrap();
        let mut buf_reader = BufReader::new(file);

        let lines = buf_reader.lines();
        for line in lines {
            let line = line.unwrap();
            self.rows.push(Row::new(line));
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

    fn statusbar(&mut self, mut buffer: String) -> String {
        buffer.push_str(format!("{}", style::Invert).as_str());
        let filename = self.filename.clone().unwrap_or("[None]".to_string());
        let status = format!(" {} - {} lines", filename, self.rows.len());
        let rstatus = format!("{}/{} ", self.cy+1, self.rows.len());
        let mut status_size = status.len();
        let rstatus_size = rstatus.len();
        status_size = if status_size as u16 > self.screencols {self.screencols as usize} else {status_size};
        buffer = buffer + &status[..status_size];

        for _ in 0..self.screencols as i16-status_size as i16 - rstatus_size as i16 {
            buffer.push(' ');
        }
        if self.screencols as i16 - status_size as i16 - rstatus_size as i16 >= 0 {
            buffer = buffer + &rstatus;
        }
        buffer.push_str(format!("{}\r\n", style::NoInvert).as_str());
        buffer
    }

    fn message_bar(&mut self, mut buffer: String) -> String {
        if self.status_message.is_some() {
            let (message, time) = self.status_message.clone().unwrap();
            let mut message_len = message.len();
            message_len = if message_len > self.screencols as usize {self.screencols as usize} else {message_len};
            match time.elapsed() {
                Ok(elapsed) if elapsed.as_secs() < 5 => buffer + &message,
                Ok(_) | Err(_) => buffer
            }
        } else {
            buffer
        }
    }

    fn set_status_message(&mut self, message: String) {
        self.status_message = Some((message, SystemTime::now()));
    }

    fn refresh(&mut self) {
        let mut buffer = String::new();
        for y in 0..self.screenrows as usize {
            let file_row = y + self.rowoff;
            if file_row >= self.rows.len() {
                if self.rows.len() == 0 && y == self.screenrows as usize / 3 {
                    let welcome = "Kilo editor for Rust -- version 0.0.1";
                    let welcome_len = welcome.len() as u16;
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
                    buffer.push_str(format!("{}", style::Bold).as_str());
                    buffer.push('~');
                    buffer.push_str(format!("{}", style::Reset).as_str());
                }
            } else {
                let mut len =
                    if self.rows[file_row as usize].chars.len() < self.coloff as usize {
                        0
                    } else {
                        self.rows[file_row as usize].chars.len() - self.coloff as usize
                    };
                // let mut len = self.erow[file_row as usize].chars.len() - self.coloff;
                // if len < 0 {len = 0}
                if len > self.screencols as usize {len = self.screencols as usize}
                eprintln!("coloff: {}, len: {}", self.coloff, len);
                if len > 0 {
                    let render = &self.rows[file_row as usize]
                        .render[self.coloff as usize..(self.coloff as usize + len) as usize];
                    buffer.push_str(render);
                }
            }
            buffer.push_str("\r\n");
        }
        self.clear();
        buffer = self.statusbar(buffer);
        buffer = self.message_bar(buffer);
        buffer.push_str(
            format!("{}", cursor::Goto(
                (self.rx - self.coloff + 1) as u16, (self.cy-self.rowoff+1) as u16)).as_str());
        self.write(buffer.as_str());
        // eprintln!("cursor: {}, {}", self.cx, self.cy);
    }

    fn process_keypress(&mut self) -> Result<i32, i32> {
        let c = self.stdin.next().unwrap().unwrap();
        match c {
            Key::Ctrl('q') => return Err(1),
            Key::Char(ch) => self.write_char(ch),
            Key::Up | Key::Down | Key::Left | Key::Right => self.move_cursor(c),
            _ => {}
        }
        return Ok(0)
    }

    fn row_cx_to_rx(&mut self, row: usize, cx: usize) -> usize {
        let mut rx = 0;
        self.rows[row].chars.chars();
        for j in self.rows[row].chars.chars().take(self.cx) {
            if j == '\t' {
                rx += (KILO_TAB_STOP - 1) - (rx % KILO_TAB_STOP);
            }
            rx += 1;
        }
        return rx;
    }

    fn move_cursor(&mut self, key: Key) {
        // let mut rowInput = None;
        {
            let row = if self.cy >= self.rows.len() {
                None
            } else {
                Some(self.rows[self.cy].render.as_str())
            };

            match key {
                Key::Down => self.cy += if self.cy < self.rows.len() { 1 } else { 0 },
                Key::Up => self.cy -= if self.cy > 0 { 1 } else { 0 },
                Key::Right => if let Some(row) = row {
                    if self.cx < row.len() {
                        self.cx += 1
                    }
                },
                Key::Left => self.cx -= if self.cx > 0 { 1 } else { 0 },
                _ => panic!("only call with cursor keys")
            }
        }

        if !(self.cy >= self.rows.len()) {
            let rowlen = self.rows[self.cy].render.len();
            if self.cx > rowlen{
                self.cx = rowlen;
            }
        };

        self.rx = 0;
        if self.cy < self.rows.len() {
            let (cx, cy) = (self.cx, self.cy);
            self.rx = self.row_cx_to_rx(cy, cx);
        }

        if self.cy < self.rowoff {
            self.rowoff = self.cy;
        }
        if self.cy >= self.rowoff + self.screenrows as usize {
            self.rowoff = self.cy - self.screenrows as usize + 1;
        }
        if self.rx < self.coloff {
            self.coloff = self.rx;
        }
        if self.rx >= self.coloff + self.screencols as usize {
            self.coloff = self.rx - self.screencols as usize + 1;
        }
    }

}

fn init_editor() {
    let args: Vec<String> = env::args().collect();
    println!("{:?}, {}", args, args.len());
    let stdin = stdin();
    let mut ret = Ok(1);
    {
        let mut editor = Editor::new();
        if args.len() > 1 {
            editor.read_file(args[1].clone());
        } else {
            editor.read_file("src/main.rs".to_string());
        }
        editor.clear();
        editor.write("Hey there, how are you");
        editor.set_status_message("HELP: Ctrl-Q = quit".to_string());
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
