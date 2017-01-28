extern crate termion;

use termion::{color, clear, cursor, style};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use std::cmp::{max, min};
use std::{env, fmt};
use std::fs::File;
use std::path::Path;
use std::io;
use std::io::{Read, Write, stdin, stdout};

struct Point {
    column: u32,
    line: u32
}

impl Point {
    fn new() -> Point {
        Point {column: 1, line: 1}
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.column, self.line)
    }
}

struct View {
    y: u32,
    height: u32
}

impl View {
    fn adjust(&mut self, point: &Point) {
        if point.line < self.y {
            self.y = point.line;
        // Add 1 (and 2 instead of 1) because termion starts indexing at 1...
        } else if point.line + 1 >= self.y + self.height {
            self.y = point.line - self.height + 2;
        }
    }
}

struct Buffer {
    name: String,
    path: String,
    point: Point,
    view: View,
    data: Vec<String>
}

impl Buffer {
    fn load<P: AsRef<Path>>(file_path: P) -> io::Result<Buffer> {
        let mut f = try!(File::open(&file_path));
        let mut data = String::new();
        try!(f.read_to_string(&mut data));

        let size = termion::terminal_size().unwrap();
        let view = View {y: 1, height: size.1 as u32};
        let name = file_path.as_ref().file_name().unwrap().to_str().unwrap().to_string();
        let path = file_path.as_ref().to_str().unwrap().to_string();

        let mut lines = Vec::new();
        for l in data.lines() {
            lines.push(l.to_string());
        }

        Ok(Buffer {name: name,
            path: path,
            point: Point::new(),
            view: view,
            data: lines
        })
    }

    fn lines(&self) -> u32 {
        self.data.len() as u32
    }

    fn move_down(&mut self) {
        self.point.line = min(self.lines(), self.point.line + 1);
        self.view.adjust(&self.point)
    }

    fn move_up(&mut self) {
        self.point.line = max(1, self.point.line - 1);
        self.view.adjust(&self.point)
    }

    fn move_start(&mut self) {
        self.point.line = 1;
        self.view.adjust(&self.point)
    }

    fn move_end(&mut self) {
        self.point.line = self.lines();
        self.view.adjust(&self.point)
    }
}


fn display(stdout: &mut io::Stdout, buffer: &Buffer) {
    display_lines(stdout, buffer);
    display_status_line(stdout, buffer);
    display_point(stdout, &buffer.point, &buffer.view);
    stdout.flush().unwrap();
}

fn display_lines(stdout: &mut io::Stdout, buffer: &Buffer) {
    let mut ln = 1;
    let i = (buffer.view.y - 1) as usize;

    for l in &buffer.data[i..] {
        write!(stdout, "{goto}{line}{clear}",
               goto = cursor::Goto(1, ln),
               line = l,
               clear = clear::UntilNewline).unwrap();
        ln += 1;
        if ln > buffer.view.height as u16 {
            break;
        }
    }
}

fn display_status_line(stdout: &mut io::Stdout, buffer: &Buffer) {
    write!(stdout, "{goto}", goto = cursor::Goto(1, buffer.view.height as u16)).unwrap();
    write!(stdout, "{bold}{color}{name} [{path}]  {point}({lines}){boldreset}{colorreset}",
           bold = style::Bold,
           color = color::Fg(color::Blue),
           name = buffer.name,
           path = buffer.path,
           point = buffer.point,
           lines = buffer.lines(),
           boldreset = style::Reset,
           colorreset = color::Fg(color::Reset)).unwrap();
    write!(stdout, "{clear}", clear = clear::UntilNewline).unwrap();
}

fn display_point(stdout: &mut io::Stdout, point: &Point, view: &View) {
    write!(stdout, "{}",
           // Add 1 because termion starts indexing at 1...
           cursor::Goto(1, (point.line - view.y + 1) as u16)).unwrap();
}

macro_rules! die {
    ($($arg:tt)*) => {{
        let mut stderr = std::io::stderr();
        write!(stderr, $($arg)*).unwrap();
        std::process::exit(1);
    }}
}

fn main() {
    let mut args = env::args();
    let file = match args.nth(1) {
        Some(f) => { f }
        None => { die!("Please specify a file you want to open.\n") }
    };
    let mut buf = match Buffer::load(&file) {
        Ok(b) => { b }
        Err(e) => { die!("Could not open file: '{}'.\n", e.to_string()); }
    };

    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode().unwrap();
    let size = termion::terminal_size().unwrap();
    print!("{}", clear::All);

    display(&mut stdout, &buf);

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Down | Key::Char('k') => { buf.move_down() }
            Key::Up   | Key::Char('i') => { buf.move_up() }
            Key::PageDown | Key::Char('K') => {
                for _ in 0..size.1 {
                    buf.move_down();
                }
            }
            Key::PageUp   | Key::Char('I') => {
                for _ in 0..size.1 {
                    buf.move_up();
                }
            }
            Key::Char('>') => { buf.move_end() }
            Key::Char('<') => { buf.move_start() }
            Key::Char('q') => { break; }
            _ => { }
        }

        display(&mut stdout, &buf);
    }

    // Reset the cursor to the bottom.
    println!("{}", cursor::Goto(1, size.1 + 1));
}
