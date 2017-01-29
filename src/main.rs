extern crate termion;

use termion::{color, clear, cursor, style};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use std::cmp::{max, min};
use std::env;
use std::fs::File;
use std::path::Path;
use std::io;
use std::io::{BufRead, BufReader, Write, stdin, stdout};

type Point = usize;

// View into the buffer.
struct View {
    y: usize,
    height: usize // How many lines to draw.
}

impl View {
    fn adjust(&mut self, line: usize) {
        if line < self.y {
            self.y = line;
        }
        else if line >= self.y + self.height {
            self.y = line - self.height + 1;
        }
    }
}


// Ideally, this should be a Rope. Let's make the API the same, so it can later be replaced by a Rope
// implementation.
struct Text {
    length: usize,
    newlines: usize,
    text: Vec<String>
}

impl Text {
    fn from_file<P: AsRef<Path>>(file_path: P) -> io::Result<Text> {
        let f = try!(File::open(&file_path));
        let reader = BufReader::new(f);
        let mut text = Vec::new();
        let mut length = 0;
        let mut newlines = 0;
        for l in reader.lines() {
            let line = l.unwrap();
            length += line.len() + 1; // Count the newline. TODO: This won't work if the newline is not '\n'.
            newlines += 1;
            text.push(line);
        }
        Ok(Text{
            length: if length == 0 {0} else {length - 1}, // An extra line was added above.
            newlines: newlines,
            text: text
        })
    }

    // NOTE: This has to be blazingly fast, but this implementation will get *very* slow for big
    // amounts of lines.
    fn line_at(&self, p: Point) -> (usize, usize, usize) {
        let mut line = 0;
        let mut start = 0;
        let mut len = 0;
        for l in &self.text {
            len = l.len();
            if p > start + len {
                start += len + 1;
                line += 1;
            }
            else {
                break;
            }
        }
        (line, start, len)
    }

    fn insert(&mut self, pos: usize, s: String) {
        unimplemented!();
    }

    fn delete(&mut self, pos: usize, count: usize) {
        unimplemented!();
    }

    fn delete_line(&mut self, line: usize) {
        if self.newlines > 0 {
            if self.newlines == 1 {
                self.length = 0;
            }
            else {
                self.length -= self.text[line].len() + 1;
            }
            self.newlines -= 1;
            // Index sanity should be checked by the caller. Let remove() panic if not sane.
            self.text.remove(line);
        }
    }
}


struct Buffer {
    name: String,
    path: String,
    point: Point,
    view: View,
    data: Text
}

impl Buffer {
    fn load<P: AsRef<Path>>(file_path: P) -> io::Result<Buffer> {
        let size = termion::terminal_size().unwrap();
        // Leave space for the status bar.
        let view = View {y: 0, height: (size.1 - 1) as usize};
        let name = file_path.as_ref().file_name().unwrap().to_str().unwrap().to_string();
        let path = file_path.as_ref().to_str().unwrap().to_string();

        Ok(Buffer {name: name,
            path: path,
            point: 0,
            view: view,
            data: try!(Text::from_file(file_path))
        })
    }

    fn lines(&self) -> usize {
        self.data.newlines
    }

    fn move_right(&mut self) {
        self.point = max(0, min(self.data.length, self.point + 1));
        let (line, _, _) = self.data.line_at(self.point);
        self.view.adjust(line);
    }

    fn move_left(&mut self) {
        if self.point > 0 {
            self.point = max(0, self.point - 1);
        }
        let (line, _, _) = self.data.line_at(self.point);
        self.view.adjust(line);
    }

    fn move_end_of_line(&mut self) {
        let (_, start, len) = self.data.line_at(self.point);
        self.point = start + len;
    }

    fn move_start_of_line(&mut self) {
        let (_, start, _) = self.data.line_at(self.point);
        self.point = start;
    }

    fn move_down(&mut self) {
        let (_, start, len) = self.data.line_at(self.point);
        self.point = min(self.data.length, start + len + 1);
        let (line, _, _) = self.data.line_at(self.point);
        self.view.adjust(line);
    }

    fn move_up(&mut self) {
        let (_, start, _) = self.data.line_at(self.point);
        if start == 0 {
            self.point = 0;
        }
        else {
            self.point = start - 1;
            let (_, start, _) = self.data.line_at(self.point);
            self.point = start;
        }
        let (line, _, _) = self.data.line_at(self.point);
        self.view.adjust(line)
    }

    fn move_start(&mut self) {
        self.point = 0;
        self.view.adjust(0);
    }

    fn move_end(&mut self) {
        let line = self.lines() - 1;
        self.point = self.data.length;
        self.view.adjust(line);
    }

    fn delete_line(&mut self) {
        let (line, start, _) = self.data.line_at(self.point);
        self.data.delete_line(line);
        self.point = max(0, min(self.data.length, start));
        let (line, _, _) = self.data.line_at(self.point);
        self.view.adjust(line);
    }
}


fn display(stdout: &mut io::Stdout, buffer: &Buffer) {
    display_lines(stdout, buffer);
    display_status_line(stdout, buffer);
    display_point(stdout, buffer);
    stdout.flush().unwrap();
}

fn display_lines(stdout: &mut io::Stdout, buffer: &Buffer) {
    if buffer.lines() == 0 {
        write!(stdout, "{}", clear::All).unwrap();
        return;
    }

    let mut ln = 0;
    let i = buffer.view.y;
    let lines = &buffer.data.text[i..];
    let count = lines.len();

    for l in lines {
        write!(stdout, "{goto}{line}{clear}",
               // Add 1 because termion starts indexing at 1...
               goto = cursor::Goto(1, ln + 1),
               line = l,
               clear = clear::UntilNewline).unwrap();
        ln += 1;
        if ln as usize >= count {
            write!(stdout, "{}", clear::AfterCursor).unwrap();
            break;
        }
        // Render buffer.view.height lines.
        else if ln as usize >= buffer.view.height {
            break;
        }
    }
}

fn display_status_line(stdout: &mut io::Stdout, buffer: &Buffer) {
    let (line, start, _) = buffer.data.line_at(buffer.point);
    write!(stdout, "{goto}", goto = cursor::Goto(1, (buffer.view.height + 1) as u16)).unwrap();
    write!(stdout, "{bold}{color}{name} [{path}]  {column}:{line}/{lines}{boldreset}{colorreset}",
           bold = style::Bold,
           color = color::Fg(color::Blue),
           name = buffer.name,
           path = buffer.path,
           column = buffer.point - start + 1,
           line = line + 1,
           lines = buffer.lines(),
           boldreset = style::Reset,
           colorreset = color::Fg(color::Reset)).unwrap();
    write!(stdout, "{clear}", clear = clear::UntilNewline).unwrap();
}

fn display_point(stdout: &mut io::Stdout, buffer: &Buffer) {
    let (line, start, _) = buffer.data.line_at(buffer.point);
    write!(stdout, "{}",
           // Add 1 because termion starts indexing at 1...
           cursor::Goto((buffer.point - start + 1) as u16,
                        (line - buffer.view.y + 1) as u16)).unwrap();
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
            Key::Right | Key::Char('l') => { buf.move_right() }
            Key::Left  | Key::Char('j') => { buf.move_left() }
            Key::Down  | Key::Char('k') => { buf.move_down() }
            Key::Up    | Key::Char('i') => { buf.move_up() }
            Key::End   | Key::Char('L') => { buf.move_end_of_line() }
            Key::Home  | Key::Char('J') => { buf.move_start_of_line() }
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

            Key::Char('d') => { buf.delete_line() }
            Key::Char('q') => { break; }
            _ => { }
        }

        display(&mut stdout, &buf);
    }

    // Reset the cursor to the bottom.
    println!("{}", cursor::Goto(1, size.1 + 1));
}
