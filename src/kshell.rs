#![allow(clippy::never_loop)]

use crate::ipc;
use crate::ipc::IpcNode;
use crate::ktask;
use crate::AsciiStr;
use crate::{queue_write, queue_writeln, spawn_task};
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use futures::StreamExt;

struct KShell {
    pub input: ipc::IpcRef,
    pub output: ipc::IpcRef,
    pub root: ipc::IpcRef,
    pub cwd: Vec<u64>,
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
                    queue_write!(self.output.clone(), "\r\n");
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

    fn parse_path(output: &ipc::IpcRef, cwd: &[u64], path: &[u8]) -> Result<Vec<u64>, ()> {
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
            } else if part.starts_with(b":") {
                let as_str = core::str::from_utf8(&part[1..]).map_err(|_| ())?;
                root.push(u64::from_str_radix(as_str, 16).map_err(|_| ())?);
            } else {
                queue_writeln!(
                    output.clone(),
                    "Error: Filenames unsupported in IPC namespace, Use /:1234/:cafe/ notation"
                );
                return Err(());
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
            queue_write!(self.output.clone(), "\x1b[32m");
            queue_write!(self.output.clone(), "kernel");
            queue_write!(self.output.clone(), "\x1b[0m");
            queue_write!(self.output.clone(), "@bold ipc:");
            queue_write!(self.output.clone(), "\x1b[32m");
            queue_write!(self.output.clone(), "/");
            for dir in &self.cwd {
                queue_write!(self.output.clone(), ":{:x}/", *dir);
            }
            queue_write!(self.output.clone(), "\x1b[0m");
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
                            "help      : Print list of available commands"
                        );
                        queue_writeln!(self.output.clone(), "ls <PATH> : List current directory");
                        queue_writeln!(self.output.clone(), "cd <PATH> : Change directory");
                    }
                    b"ls" => loop {
                        // Loops for a single iteration, just so we can break out

                        let path = match words.len() {
                            1 => self.cwd.clone(),
                            2 => match KShell::parse_path(&self.output, &self.cwd, words[1]) {
                                Ok(path) => path,
                                Err(_) => {
                                    queue_writeln!(self.output.clone(), "Error: Invalid Path");
                                    break;
                                }
                            },
                            _ => {
                                queue_writeln!(
                                    self.output.clone(),
                                    "Error: Invalid Usage, see `help`"
                                );
                                break;
                            }
                        };
                        if let Some(cwd) = self.navigate_to_path(&path).await {
                            if let IpcNode::Dir(_) = *cwd.inner {
                                if let Some(mut stream) = cwd.dir_list() {
                                    while let Some(item) = stream.next().await {
                                        queue_writeln!(self.output.clone(), "{:?}", item);
                                    }
                                } else {
                                    queue_writeln!(
                                        self.output.clone(),
                                        "Error: Failed to list given path"
                                    );
                                }
                            } else {
                                queue_writeln!(self.output.clone(), "Error: Not a directory");
                            }
                        } else {
                            queue_writeln!(self.output.clone(), "Error: Directory doesn't exist");
                        }
                        break;
                    },
                    b"cd" => loop {
                        // Loops for a single iteration, just so we can break out

                        let path = match words.len() {
                            1 => vec![],
                            2 => match KShell::parse_path(&self.output, &self.cwd, words[1]) {
                                Ok(path) => path,
                                Err(_) => {
                                    queue_writeln!(self.output.clone(), "Error: Invalid Path");
                                    break;
                                }
                            },
                            _ => {
                                queue_writeln!(
                                    self.output.clone(),
                                    "Error: Invalid Usage, see `help`"
                                );
                                break;
                            }
                        };

                        if let Some(given_dir) = self.navigate_to_path(&path).await {
                            if let IpcNode::Dir(_) = *given_dir.inner {
                                self.cwd = path;
                            } else {
                                queue_writeln!(self.output.clone(), "Error: Not a directory");
                            }
                        } else {
                            queue_writeln!(self.output.clone(), "Error: Directory doesn't exist");
                        }

                        break;
                    },
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
}

pub fn launch() {
    spawn_task!({
        // Open the input queue
        let root = ipc::ROOT.read().as_ref().unwrap().clone();
        let input_queue = root.dir_get(0xcafe).await.unwrap();
        let output_queue = root.dir_get(0xbabe).await.unwrap();

        // Create shell
        KShell {
            input: input_queue,
            output: output_queue,
            root,
            cwd: vec![],
        }
        .run()
        .await
    });
}
