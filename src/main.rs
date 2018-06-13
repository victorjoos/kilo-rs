extern crate termion;

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, stdin, Stdin, stdout, Stdout, Write};
use std::time::SystemTime;
use termion::{clear, color, cursor, style};
use termion::event::Key;
use termion::input::{Keys, TermRead};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use termion::terminal_size;
use std::fmt;
use std::error::Error;


const KILO_TAB_STOP:usize = 8;
const KILO_QUIT_TIMES:u16 = 2;

enum Mode {
    Normal,
    Insert,
    Visual,
}



impl Mode {
    fn process_keypress(&mut self, editor:&mut Editor, c: Key) {
        match self {
            Mode::Insert => self.insert(editor, c),
            Mode::Normal => self.normal(editor, c),
            Mode::Visual => self.visual(editor, c),
        }
    }

    fn normal(&mut self, editor:&mut Editor, c: Key) {
        match c {
            Key::Char('i') => {},
            _ => {}
        }
    }

    fn insert(&mut self, editor:&mut Editor, c: Key) {
        match c {
            Key::Esc => {},
            Key::Ctrl('q') => {},
            Key::Ctrl('s') => {},
            Key::Char('\n') => {},
            Key::Char(ch) => {},
            c @ Key::Backspace | c @ Key::Ctrl('h') | c @ Key::Delete => {},
            Key::Up | Key::Down | Key::Left | Key::Right => {},
            _ => {}
        }
    }

    fn visual(&mut self, editor:&mut Editor, c: Key) {

    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (color, string) = match *self {
            Mode::Normal => (&color::Yellow as &color::Color , "Normal"),
            Mode::Insert => (&color::Blue as &color::Color, "Insert"),
            Mode::Visual => (&color::Red as &color::Color, "Visual"),
        };
        write!(f, "{}{} {}", color::Fg(color), string, color::Fg(color::Reset))
    }
}

#[derive(PartialEq)]
#[derive(Copy, Clone)]
enum Highlight {
    Normal,
    Number,
}

impl Highlight {
    fn to_color(& self) -> color::Fg<&color::Color> {
        match self {
            &Highlight::Normal => color::Fg(&color::Reset),
            &Highlight::Number => color::Fg(&color::Red),
        }
    }
}

struct Row {
    chars: String,
    render: String,
    highlight: Vec<Highlight>,
}

impl Row {
    fn new(chars: String) -> Row {
        let render = chars.clone();
        let mut row = Row {
            chars,
            render,
            highlight: Vec::new(),
        };
        row.update();
        row
    }

    fn update(&mut self) {

        self.render.clear();
        let mut idx = 0;

        for character in self.chars.chars() {
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
        self.update_syntax()
    }

    fn update_syntax(&mut self) {
        self.highlight.clear();
        for character in self.render.chars() {
            if character.is_digit(10) {
                self.highlight.push(Highlight::Number)
            } else {
                self.highlight.push(Highlight::Normal)
            }
        }

    }

    fn draw(&mut self, mut buffer: String, coloff: usize, len: usize) -> String {
        let mut current = Highlight::Normal;
        for (character, highlight) in self.render.chars().zip(self.highlight.clone()).skip(coloff).take(len) {
            match highlight {
                hl if hl == current => buffer.push(character),
                hl => {
                    buffer.push_str(format!("{}{}", hl.to_color(), character).as_str());
                    current = hl
                }
            }
        }
        buffer.push_str(format!("{}", Highlight::Normal.to_color()).as_str());
        buffer
    }

    fn insert_char(&mut self, mut at: usize, c: char) {
        if at > self.chars.len() {
            at = self.chars.len();
        }

        self.chars.insert(at, c);
        self.update();
    }

    fn append_string(&mut self, line: String) {
        self.chars += &line;
        self.update();
    }

    fn delete_char(&mut self, at: usize) {
        if at >= self.chars.len() {return}

        self.chars.remove(at);
        self.update();
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
    mode: Mode,
    dirty: bool,
    quit_times: u16,
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
            mode:Mode::Insert,
            screen,
            dirty:false,
            quit_times: KILO_QUIT_TIMES,
            filename:None,
            status_message: None,
            stdin:stdin().keys(),
        }
    }

    fn read_file(&mut self, filename: String) {
        self.filename = Some(filename.clone());
        let file = File::open(&filename);
        if let Err(err) = file {
            self.set_status_message(format!("{} [New file]", filename));
            return
        }
        let file = file.unwrap();
        let buf_reader = BufReader::new(file);

        let lines = buf_reader.lines();
        for line in lines {
            let line = line.unwrap();
            self.rows.push(Row::new(line));
        }
    }

    fn rows_to_string(&self) -> String {
        let mut buffer = String::new();
        /*self.rows.foreach(|row| {
           buffer += &row.chars
        });*/
        for row in &self.rows {
            buffer += &row.chars;
            buffer.push('\n');
        }
        buffer
    }

    fn save(&mut self, save_as: bool) {
        if let None = self.filename.clone() {
            self.filename = self.prompt("Save as: ".to_string());
        } else if save_as {
            self.filename = self.prompt("Save as: ".to_string());
        }

        if let Some(filename) = self.filename.clone() {
            let mut file = match File::create(&filename) {
                Ok(file) => file,
                Err(_err) => {
                    self.set_status_message("Can't save, I/O error".to_string());
                    return;
                }
            };
            let buffer = self.rows_to_string();
            file.write_all(buffer.as_bytes()).expect("Unable to write data");
            self.dirty = false;
            self.set_status_message(format!("{} bytes written in \"{}\"", buffer.len(), filename));
        } else {
            self.set_status_message("Not saved :(".to_string());
        }
    }

    fn insert_char(&mut self, c: char) {
        self.dirty = true;
        if self.cy == self.rows.len() {
            self.rows.push(Row::new("".to_string()));
        }
        // eprintln!("insert at: {}, {}", self.cx, self.cy);
        self.rows[self.cy].insert_char(self.cx, c);
        self.cx += 1;
    }

    fn delete_char(&mut self) {
        if self.cy == self.rows.len() {return}
        if self.cx == 0 && self.cy == 0 {return}

        self.dirty = true;

        if self.cx > 0 {
            self.rows[self.cy].delete_char(self.cx - 1);
            self.cx -= 1;
        } else {
            self.cx = self.rows[self.cy - 1].chars.len();
            let previous_row = self.rows.remove(self.cy).chars;
            self.rows[self.cy - 1].append_string(previous_row);
            self.cy -= 1;
        }
    }

    fn insert_row(&mut self, at: usize, s: String) {
        if at > self.rows.len() {return}

        self.rows.insert(at, Row::new(s));
    }

    fn insert_newline(&mut self) {
        self.dirty = true;
        if self.cx == 0 {
            let cy = self.cy;
            self.insert_row(cy, "".to_string());
        } else {
            let (cx, cy) = (self.cx, self.cy);
            let next_row = self.rows[self.cy].chars.split_off(cx);
            self.rows[self.cy].update();
            self.insert_row(cy + 1, next_row);

        }
        self.cy += 1;
        self.cx = 0;
    }

    fn write(&mut self, string: &str) {
        write!(self.screen, "{}{}{}",clear::All, cursor::Goto(1,1), string).unwrap();
        self.screen.flush().unwrap();
    }

    fn status_bar(&mut self, mut buffer: String) -> String {
        buffer.push_str(format!("{}", style::Invert).as_str());
        let filename = self.filename.clone().unwrap_or("[None]".to_string());
        let modified = if self.dirty {"(modified)"} else {""};
        let status = format!("{} {} - {} lines {}", self.mode, filename, self.rows.len(), modified);
        let rstatus = format!("{}/{} ", self.cy+1, self.rows.len());
        let mut status_size = status.len();
        let rstatus_size = rstatus.len();
        status_size = if status_size as u16 > self.screencols {self.screencols as usize} else {status_size};
        buffer = buffer + &status[..status_size];

        for _ in 0..self.screencols as i16-status_size as i16 - rstatus_size as i16 + 14 {
            buffer.push(' ');
        }
        if self.screencols as i16 - status_size as i16 - rstatus_size as i16 >= 0 {
            buffer = buffer + &rstatus;
        }
        buffer.push_str(format!("{}\r\n", style::NoInvert).as_str());
        buffer
    }

    fn message_bar(&mut self, buffer: String) -> String {
        if self.status_message.is_some() {
            let (message, time) = self.status_message.clone().unwrap();
            let mut message_len = message.len();
            message_len = if message_len > self.screencols as usize {self.screencols as usize} else {message_len};
            match time.elapsed() {
                Ok(elapsed) if elapsed.as_secs() < 5 => buffer + &message[..message_len],
                Ok(_) | Err(_) => buffer
            }
        } else {
            buffer
        }
    }

    fn set_status_message(&mut self, message: String) {
        self.status_message = Some((message, SystemTime::now()));
    }

    fn prompt(&mut self, message: String) -> Option<String> {
        let mut buffer = String::new();
        loop {
            self.set_status_message(format!("{}{}", message, buffer));
            self.draw();
            let c = self.stdin.next().unwrap().unwrap();
            match c {
                Key::Delete | Key::Backspace => {buffer.pop();()},
                Key::Esc => {
                    self.set_status_message("".to_string());
                    return None;
                },
                Key::Char('\n') => {
                    if buffer.len() != 0 {
                        self.set_status_message("".to_string());
                        return Some(buffer);
                    }
                },
                Key::Char(c) => buffer.push(c),
                _ => (),
            }
        }
    }

    fn draw(&mut self) {
        let mut buffer = String::with_capacity(((self.screencols) * self.screenrows) as usize);
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
                    if self.rows[file_row as usize].render.len() < self.coloff as usize {
                        0
                    } else {
                        self.rows[file_row as usize].render.len() - self.coloff as usize
                    };

                if len > self.screencols as usize {len = self.screencols as usize}
                if len > 0 {
                    buffer = self.rows[file_row].draw(buffer, self.coloff, len);
                    // let len_diff = self.rows[file_row].render_hl.len() - self.rows[file_row].render.len();
                    // eprintln!("len_diff: {}", len_diff);
                    // let render = &self.rows[file_row]
                    //     .render_hl[self.coloff as usize..(self.coloff as usize + len + len_diff) as usize];
                    // buffer.push_str(render); // &self.rows[file_row].render_hl);
                }
            }
            buffer.push_str("\r\n");
        }
        buffer = self.status_bar(buffer);
        buffer = self.message_bar(buffer);
        self.scroll_cursor();
        buffer.push_str(
            format!("{}", cursor::Goto(
                (self.rx - self.coloff + 1) as u16, (self.cy-self.rowoff+1) as u16)).as_str());

        self.write(buffer.as_str());
        // eprintln!("cursor: {}, {}", self.cx, self.cy);
    }

    fn process_keypress(&mut self) -> Result<i32, i32> {
        let c = self.stdin.next().unwrap().unwrap();
        match c {
            Key::Ctrl('q') => {
                if self.dirty && self.quit_times > 0 {
                    let quit = self.quit_times;
                    self.set_status_message(format!("WARNING!!! file has unsaved changes. Press Ctrl-Q {} more times to quit", quit));
                    self.quit_times -= 1;
                    return Ok(1)
                } else {
                    return Err(1)
                }
            },
            Key::Ctrl('s') => self.save(false),
            Key::Char('\n') => self.insert_newline(),
            Key::Char(ch) => self.insert_char(ch),
            c @ Key::Backspace | c @ Key::Ctrl('h') | c @ Key::Delete => {
                if c == Key::Delete {
                    self.move_cursor(Key::Right);
                }
                self.delete_char()
            },
            Key::Up | Key::Down | Key::Left | Key::Right => self.move_cursor(c),
            _ => {}
        }
        self.quit_times = KILO_QUIT_TIMES;
        return Ok(0)
    }

    fn row_cx_to_rx(&mut self, row: usize, cx: usize) -> usize {
        let mut rx = 0;
        for j in self.rows[row].chars.chars().take(cx) {
            if j == '\t' {
                rx += (KILO_TAB_STOP - 1) - (rx % KILO_TAB_STOP);
            }
            rx += 1;
        }
        return rx;
    }

    fn scroll_cursor(&mut self) {
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

    fn move_cursor(&mut self, key: Key) {
        // let mut rowInput = None;
        {
            let row_length = if self.cy >= self.rows.len() {
                0
            } else {
                self.rows[self.cy].chars.len()
            };

            match key {
                Key::Down => self.cy += if self.cy < self.rows.len() { 1 } else { 0 },
                Key::Up => self.cy -= if self.cy > 0 { 1 } else { 0 },
                Key::Right => if row_length > 0 {
                    if self.cx < row_length {
                        self.cx += 1
                    }
                },
                Key::Left => self.cx -= if self.cx > 0 { 1 } else { 0 },
                _ => panic!("only call with cursor keys")
            }
        }

        if !(self.cy >= self.rows.len()) {
            let rowlen = self.rows[self.cy].chars.len();
            if self.cx > rowlen{
                self.cx = rowlen;
            }
        };
    }

}

fn init_editor() {
    let args: Vec<String> = env::args().collect();
    let mut ret = Ok(1);
    {
        let mut editor = Editor::new();
        editor.set_status_message("HELP: Ctrl-S = save | Ctrl-Q = quit".to_string());
        if args.len() > 1 {
            editor.read_file(args[1].clone());
        }

        while let Ok(_) = ret {
            editor.draw();
            ret = editor.process_keypress();
        }
    }

    println!("bye !");
}

fn main() {
    init_editor();
    return;
}
