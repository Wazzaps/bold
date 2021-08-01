#![allow(clippy::never_loop)]

use crate::framebuffer_console;
use crate::ktask;
use crate::AsciiStr;
use crate::{fonts, ipc};
use crate::{println, queue_write, queue_writeln, spawn_task};
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use futures::future::BoxFuture;
use futures::stream;
use futures::StreamExt;

struct KShell {
    pub input: ipc::IpcRef,
    pub output: ipc::IpcRef,
    pub root: ipc::IpcRef,
    pub cwd: Vec<u64>,
    pub colors: bool,
}

impl KShell {
    pub async fn read_char(&mut self) -> u8 {
        let mut buf = [0u8; 1];
        loop {
            let res_len = self.input.queue_read(&mut buf).await.unwrap();
            if res_len > 0 {
                return buf[0];
            }
            ktask::yield_now().await;
        }
    }

    pub async fn read_line(&mut self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut readline_cursor = 0usize;

        fn rerender(output: &ipc::IpcRef, buf: &[u8]) {
            queue_write!(output.clone(), "\x1b[K");
            for c in buf {
                queue_write!(output.clone(), "{}", *c as char);
            }
            for _ in buf {
                queue_write!(output.clone(), "\x08");
            }
        }

        fn skip_word_left(buf: &[u8]) -> usize {
            let mut counter = 0;

            // Skip all whitespace
            for c in buf.iter().rev() {
                if *c != b' ' {
                    break;
                }
                counter += 1;
            }

            // Skip word
            for c in buf[..buf.len() - counter].iter().rev() {
                if !(b'!'..=b'~').contains(c) {
                    break;
                }
                counter += 1;
            }

            counter
        }

        fn skip_word_right(buf: &[u8]) -> usize {
            let mut counter = 0;

            // Skip all whitespace
            for c in buf.iter() {
                if *c != b' ' {
                    break;
                }
                counter += 1;
            }

            // Skip word
            for c in buf[counter..].iter() {
                if !(b'!'..=b'~').contains(c) {
                    break;
                }
                counter += 1;
            }

            counter
        }

        loop {
            let c = self.read_char().await;
            match c {
                // Normal letters
                b' '..=b'~' => {
                    queue_write!(self.output.clone(), "{}", c as char);
                    buf.insert(readline_cursor, c);
                    readline_cursor += 1;
                    rerender(&self.output, &buf[readline_cursor..]);
                }
                // Backspace
                0x7f | 0x08 => {
                    if readline_cursor > 0 {
                        buf.remove(readline_cursor - 1);
                        readline_cursor -= 1;
                        queue_write!(self.output.clone(), "\x08 \x08");
                        rerender(&self.output, &buf[readline_cursor..]);
                    }
                }
                // Delete word left
                0x17 => {
                    let word_len = skip_word_left(&buf[..readline_cursor]);
                    for _ in 0..word_len {
                        buf.remove(readline_cursor - word_len);
                        queue_write!(self.output.clone(), "\x08");
                    }
                    readline_cursor -= word_len;
                    rerender(&self.output, &buf[readline_cursor..]);
                }
                // Escape sequence
                0x1b => {
                    match self.read_char().await {
                        // Control Sequence Introducer
                        b'[' => {
                            // Get operation
                            match self.read_char().await {
                                // Up and Down unsupported, ring bell
                                b'A' | b'B' => {
                                    queue_write!(self.output.clone(), "\x07");
                                }
                                // Left
                                b'D' => {
                                    if readline_cursor != 0 {
                                        readline_cursor -= 1;
                                        queue_write!(self.output.clone(), "\x1b[D");
                                    }
                                }
                                // Right
                                b'C' => {
                                    if readline_cursor < buf.len() {
                                        readline_cursor += 1;
                                        queue_write!(self.output.clone(), "\x1b[C");
                                    }
                                }
                                // Home
                                b'H' => {
                                    for _ in 0..readline_cursor {
                                        queue_write!(self.output.clone(), "\x1b[D");
                                    }
                                    readline_cursor = 0;
                                }
                                // End
                                b'F' => {
                                    for _ in readline_cursor..buf.len() {
                                        queue_write!(self.output.clone(), "\x1b[C");
                                    }
                                    readline_cursor = buf.len();
                                }
                                // Probably Delete
                                b'3' => match self.read_char().await {
                                    // Delete
                                    b'~' => {
                                        if readline_cursor != buf.len() {
                                            buf.remove(readline_cursor);
                                            rerender(&self.output, &buf[readline_cursor..]);
                                        }
                                    }
                                    c => {
                                        queue_write!(self.output.clone(), "<{:02x}>", c);
                                    }
                                },
                                c => {
                                    queue_write!(self.output.clone(), "<{:02x}>", c);
                                }
                            }
                        }
                        c => {
                            queue_write!(self.output.clone(), "<{:02x}>", c);
                        }
                    }
                }
                // Newline
                b'\r' | b'\n' => {
                    queue_write!(self.output.clone(), "\n");
                    return buf;
                }
                // Unknown
                _ => {
                    queue_write!(self.output.clone(), "<{:02x}>", c);
                }
            };
        }
    }

    async fn navigate_to_path(&mut self, path: &[u64]) -> Option<ipc::IpcRef> {
        let mut root = self.root.clone();
        for part in path {
            root = match root.dir_get(*part).await {
                Some(new_root) => new_root,
                None => return None,
            }
        }

        Some(root)
    }

    fn parse_path(cwd: &[u64], path: &[u8]) -> Result<Vec<u64>, ()> {
        let mut root = if path.starts_with(b"/") {
            vec![]
        } else {
            cwd.to_vec()
        };

        for part in path.split(|c| *c == b'/') {
            if part == b"." || part == b"" {
                // Do nothing
            } else if part == b".." {
                root.pop();
            } else {
                let as_str = core::str::from_utf8(part).map_err(|_| ())?;
                root.push(u64::from_str_radix(as_str, 16).map_err(|_| ())?);
            }
        }

        Ok(root)
    }

    pub async fn run(&mut self) {
        queue_write!(self.output.clone(), "--- Bold KShell ---\n");
        queue_write!(
            self.output.clone(),
            "Type `help` to see available commands\n"
        );
        loop {
            // Draw prompt
            if self.colors {
                queue_write!(self.output.clone(), "\x1b[32m");
            }
            queue_write!(self.output.clone(), "kernel");
            if self.colors {
                queue_write!(self.output.clone(), "\x1b[0m");
            }
            queue_write!(self.output.clone(), "@bold ");
            if self.colors {
                queue_write!(self.output.clone(), "\x1b[32m");
            }
            queue_write!(self.output.clone(), "/");
            for dir in &self.cwd {
                queue_write!(self.output.clone(), "{:x}/", *dir);
            }
            if self.colors {
                queue_write!(self.output.clone(), "\x1b[0m");
            }
            queue_write!(self.output.clone(), "> ");

            // Get components
            let line = &self.read_line().await;
            let words = line
                .split(|c| *c == b' ')
                .filter(|s| s != b"")
                .collect::<Vec<_>>();
            if let Some(word) = words.first() {
                match *word {
                    b"help" => {
                        queue_writeln!(
                            self.output.clone(),
                            "help        : Print list of available commands\n\
                             ls <PATH>   : List directory\n\
                             tree <PATH> : List directory recursively\n\
                             cd <PATH>   : Change directory\n\
                             info        : Display system info\n\
                             font <FONT> : Change framebuffer font"
                        );
                    }
                    b"ls" => self.handle_cmd_ls(&words).await,
                    b"tree" => self.handle_cmd_tree(&words).await,
                    b"cd" => self.handle_cmd_cd(&words).await,
                    b"font" => self.handle_cmd_font(&words).await,
                    b"info" => self.handle_cmd_info(&words).await,
                    _ => {
                        queue_writeln!(
                            self.output.clone(),
                            "Error: Unknown command `{}`",
                            AsciiStr(*word)
                        );
                    }
                };
            }
        }
    }

    async fn handle_cmd_ls(&mut self, words: &[&[u8]]) {
        let path = match words.len() {
            1 => self.cwd.clone(),
            2 => match KShell::parse_path(&self.cwd, words[1]) {
                Ok(path) => path,
                Err(_) => {
                    queue_writeln!(self.output.clone(), "Error: Invalid Path");
                    return;
                }
            },
            _ => {
                queue_writeln!(self.output.clone(), "Error: Invalid Usage, see `help`");
                return;
            }
        };

        if let Some(cwd) = self.navigate_to_path(&path).await {
            if cwd.describe() == *b"DIR " {
                if let Some(mut stream) = cwd.dir_list() {
                    while let Some(item) = stream.next().await {
                        queue_writeln!(self.output.clone(), "{:?}", item);
                    }
                } else {
                    queue_writeln!(self.output.clone(), "Error: Failed to list given path");
                }
            } else {
                queue_writeln!(self.output.clone(), "Error: Not a directory");
            }
        } else {
            queue_writeln!(self.output.clone(), "Error: Directory doesn't exist");
        }
    }

    async fn handle_cmd_tree(&mut self, words: &[&[u8]]) {
        let path = match words.len() {
            1 => self.cwd.clone(),
            2 => match KShell::parse_path(&self.cwd, words[1]) {
                Ok(path) => path,
                Err(_) => {
                    queue_writeln!(self.output.clone(), "Error: Invalid Path");
                    return;
                }
            },
            _ => {
                queue_writeln!(self.output.clone(), "Error: Invalid Usage, see `help`");
                return;
            }
        };

        if let Some(cwd) = self.navigate_to_path(&path).await {
            if cwd.describe() == *b"DIR " {
                if let Some(stream) = cwd.dir_list() {
                    fn rec_print(
                        output: ipc::IpcRef,
                        mut stream: stream::BoxStream<ipc::IpcRef>,
                        depth: usize,
                    ) -> BoxFuture<()> {
                        Box::pin(async move {
                            while let Some(item) = stream.next().await {
                                queue_write!(output.clone(), "{}", "| ".repeat(depth));
                                queue_writeln!(output.clone(), "{:?}", item);
                                if let Some(stream) = item.dir_list() {
                                    rec_print(output.clone(), stream, depth + 1).await;
                                }
                            }
                        })
                    }

                    rec_print(self.output.clone(), stream, 0).await;
                } else {
                    queue_writeln!(self.output.clone(), "Error: Failed to list given path");
                }
            } else {
                queue_writeln!(self.output.clone(), "Error: Not a directory");
            }
        } else {
            queue_writeln!(self.output.clone(), "Error: Directory doesn't exist");
        }
    }

    async fn handle_cmd_font(&mut self, words: &[&[u8]]) {
        if words.len() != 2 {
            queue_writeln!(
                self.output.clone(),
                "Usage: font <vga|terminus|thin|round|tremolo>"
            );
            return;
        }

        let font;
        font = match words[1] {
            b"vga" => fonts::VGA,
            b"terminus" => fonts::TERMINUS,
            b"thin" => fonts::ISO,
            b"round" => fonts::ISO88591,
            b"tremolo" => fonts::TREMOLO,
            _ => {
                queue_writeln!(self.output.clone(), "Unknown font");
                return;
            }
        };
        crate::framebuffer_console::set_font(font);
    }

    async fn handle_cmd_cd(&mut self, words: &[&[u8]]) {
        let path = match words.len() {
            1 => vec![],
            2 => match KShell::parse_path(&self.cwd, words[1]) {
                Ok(path) => path,
                Err(_) => {
                    queue_writeln!(self.output.clone(), "Error: Invalid Path");
                    return;
                }
            },
            _ => {
                queue_writeln!(self.output.clone(), "Error: Invalid Usage, see `help`");
                return;
            }
        };

        if let Some(given_dir) = self.navigate_to_path(&path).await {
            if given_dir.describe() == *b"DIR " {
                self.cwd = path;
            } else {
                queue_writeln!(self.output.clone(), "Error: Not a directory");
            }
        } else {
            queue_writeln!(self.output.clone(), "Error: Directory doesn't exist");
        }
    }

    async fn handle_cmd_info(&mut self, _words: &[&[u8]]) {
        queue_writeln!(
            self.output.clone(),
            "Framebuffer Console:\n  {:?}",
            framebuffer_console::perf_report()
        );
        queue_writeln!(
            self.output.clone(),
            "Scheduler:\n  {:?}",
            ktask::perf_report()
        );
    }
}

pub fn launch(input_queue: ipc::IpcRef, output_queue: ipc::IpcRef, colors: bool) {
    println!("[INFO] Starting kshell");
    spawn_task!({
        let root = ipc::ROOT.read().as_ref().unwrap().clone();

        // Create shell
        KShell {
            input: input_queue,
            output: output_queue,
            root,
            cwd: vec![],
            colors,
        }
        .run()
        .await
    });
}
