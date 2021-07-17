use crate::ipc;
use crate::ktask;
use crate::AsciiStr;
use crate::{print, println, spawn_task};
use alloc::boxed::Box;
use alloc::vec::Vec;

struct KShell {
    input: ipc::IpcRef,
}

impl KShell {
    pub fn new(input: ipc::IpcRef) -> Self {
        println!("--- Bold KShell ---");
        Self { input }
    }

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

        fn rerender(buf: &[u8]) {
            print!("\x1b[K");
            for c in buf {
                print!("{}", *c as char);
            }
            for _ in buf {
                print!("\x08");
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
                    print!("{}", c as char);
                    buf.insert(readline_cursor, c);
                    readline_cursor += 1;
                    rerender(&buf[readline_cursor..]);
                }
                // Backspace
                0x7f | 0x08 => {
                    if readline_cursor > 0 {
                        buf.remove(readline_cursor - 1);
                        readline_cursor -= 1;
                        print!("\x08 \x08");
                        rerender(&buf[readline_cursor..]);
                    }
                }
                // Delete word left
                0x17 => {
                    let word_len = skip_word_left(&buf[..readline_cursor]);
                    for _ in 0..word_len {
                        buf.remove(readline_cursor - word_len);
                        print!("\x08");
                    }
                    readline_cursor -= word_len;
                    rerender(&buf[readline_cursor..]);
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
                                    print!("\x07");
                                }
                                // Left
                                b'D' => {
                                    if readline_cursor != 0 {
                                        readline_cursor -= 1;
                                        print!("\x1b[D");
                                    }
                                }
                                // Right
                                b'C' => {
                                    if readline_cursor < buf.len() {
                                        readline_cursor += 1;
                                        print!("\x1b[C");
                                    }
                                }
                                // Home
                                b'H' => {
                                    for _ in 0..readline_cursor {
                                        print!("\x1b[D");
                                    }
                                    readline_cursor = 0;
                                }
                                // End
                                b'F' => {
                                    for _ in readline_cursor..buf.len() {
                                        print!("\x1b[C");
                                    }
                                    readline_cursor = buf.len();
                                }
                                // Probably Delete
                                b'3' => match self.read_char().await {
                                    // Delete
                                    b'~' => {
                                        if readline_cursor != buf.len() {
                                            buf.remove(readline_cursor);
                                            rerender(&buf[readline_cursor..]);
                                        }
                                    }
                                    c => {
                                        print!("<{:02x}>", c);
                                    }
                                },
                                c => {
                                    print!("<{:02x}>", c);
                                }
                            }
                        }
                        c => {
                            print!("<{:02x}>", c);
                        }
                    }
                }
                // Newline
                b'\r' | b'\n' => {
                    print!("\r\n");
                    return buf;
                }
                // Unknown
                _ => {
                    print!("<{:02x}>", c);
                }
            };
        }
    }

    pub async fn run(&mut self) {
        loop {
            print!("\x1b[32m");
            print!("kernel");
            print!("\x1b[0m");
            print!("@bold ");
            print!("\x1b[32m");
            print!("/");
            print!("\x1b[0m");
            print!("# ");
            let line = &self.read_line().await;
            // Get components
            for word in line.split(|c| *c == b' ').filter(|s| s != b"") {
                println!("[{}]", AsciiStr(word));
            }
        }
    }
}

pub fn launch() {
    spawn_task!({
        // Open the input queue
        let root = ipc::ROOT.read().as_ref().unwrap().clone();
        let input_queue = root.dir_get(0xcafe).await.unwrap();

        // Create shell
        KShell::new(input_queue).run().await
    });
}
