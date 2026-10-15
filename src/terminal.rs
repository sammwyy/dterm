//! PTY lifetime, bounded output delivery and VT state. No GUI work happens on workers.
use anyhow::{Context, Result};
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::{
    io::{Read, Write},
    path::Path,
    sync::mpsc::{self, SyncSender},
    thread,
};

pub const SCROLLBACK: usize = 10_000;
pub enum Event {
    Output(u64, Vec<u8>),
    Exited(u64, String),
    Configuration(Result<crate::config::Config, String>),
}

#[derive(Default)]
pub struct Callbacks {
    pub title: String,
    pub replies: Vec<u8>,
}
impl vt100::Callbacks for Callbacks {
    fn set_window_title(&mut self, _: &mut vt100::Screen, title: &[u8]) {
        self.title = String::from_utf8_lossy(title)
            .chars()
            .filter(|c| !c.is_control())
            .take(120)
            .collect();
    }
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        prefix: Option<u8>,
        _: Option<u8>,
        params: &[&[u16]],
        action: char,
    ) {
        let first = params.first().and_then(|p| p.first()).copied().unwrap_or(0);
        let reply = match (prefix, action, first) {
            (None, 'n', 5) => "\x1b[0n".into(),
            (None | Some(b'?'), 'n', 6) => {
                let (row, col) = screen.cursor_position();
                format!(
                    "\x1b[{}{};{}R",
                    if prefix.is_some() { "?" } else { "" },
                    row + 1,
                    col + 1
                )
            }
            (None, 'c', 0) => "\x1b[?1;2c".into(),
            (Some(b'>'), 'c', 0) => "\x1b[>0;1;0c".into(),
            (None, 't', 18) => {
                let (r, c) = screen.size();
                format!("\x1b[8;{r};{c}t")
            }
            _ => String::new(),
        };
        self.replies.extend_from_slice(reply.as_bytes());
    }
}

pub struct Session {
    pub id: u64,
    pub parser: vt100::Parser<Callbacks>,
    pub exited: Option<String>,
    pub search: crate::search::Search,
    pub control_mode: Option<crate::island::Mode>,
    master: Box<dyn MasterPty + Send>,
    input: Option<SyncSender<Vec<u8>>>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}
impl Session {
    pub fn spawn(
        id: u64,
        cwd: &Path,
        command: &[String],
        events: SyncSender<Event>,
    ) -> Result<Self> {
        let pair = native_pty_system().openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let mut cmd = if let Some(program) = command.first() {
            let mut cmd = CommandBuilder::new(program);
            cmd.args(&command[1..]);
            cmd
        } else {
            let shell = crate::config::Config::default()
                .shell_command()
                .map_err(anyhow::Error::msg)?;
            CommandBuilder::new(&shell[0])
        };
        cmd.cwd(cwd);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", env!("CARGO_PKG_NAME"));
        cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
        // An inherited size would override the PTY's negotiated dimensions in some shells.
        cmd.env_remove("LINES");
        cmd.env_remove("COLUMNS");
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .context("couldn't start the terminal process")?;
        let killer = child.clone_killer();
        drop(pair.slave);
        let (input, incoming) = mpsc::sync_channel::<Vec<u8>>(64);
        thread::spawn(move || {
            while let Ok(bytes) = incoming.recv() {
                if writer
                    .write_all(&bytes)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        thread::spawn(move || {
            let mut buffer = [0; 16 * 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if events
                            .send(Event::Output(id, buffer[..n].to_vec()))
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    // Linux PTYs return EIO when the slave closes.
                    Err(_) => break,
                }
            }
            let status = match child.wait() {
                Ok(status) => status.to_string(),
                Err(error) => format!("Process ended: {error}"),
            };
            let _ = events.send(Event::Exited(id, status));
        });
        Ok(Self {
            id,
            parser: vt100::Parser::new_with_callbacks(24, 80, SCROLLBACK, Callbacks::default()),
            exited: None,
            search: crate::search::Search::default(),
            control_mode: None,
            master: pair.master,
            input: Some(input),
            killer,
        })
    }
    pub fn process(&mut self, bytes: &[u8]) -> Result<()> {
        self.parser.process(bytes);
        self.refresh_mode();
        let replies = std::mem::take(&mut self.parser.callbacks_mut().replies);
        if !replies.is_empty() {
            self.write(replies)?;
        }
        Ok(())
    }
    pub fn write(&self, bytes: Vec<u8>) -> Result<()> {
        if let Some(input) = &self.input {
            input
                .try_send(bytes)
                .context("terminal input is unavailable or busy")?;
        }
        Ok(())
    }
    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        if self.parser.screen().size() != (rows, cols) {
            self.master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
            self.parser.screen_mut().set_size(rows, cols);
            self.refresh_mode();
        }
        Ok(())
    }
    pub fn finish(&mut self, status: String) {
        self.exited = Some(status);
        self.input = None;
    }
    pub fn set_mode(&mut self, mode: Option<crate::island::Mode>) {
        self.control_mode = mode;
        match mode {
            Some(crate::island::Mode::Search) => {
                self.refresh_mode();
                self.search.reveal(self.parser.screen_mut());
            }
            None => self.search.clear(),
        }
    }
    fn refresh_mode(&mut self) {
        match self.control_mode {
            Some(crate::island::Mode::Search) => self.search.refresh(self.parser.screen()),
            None => {}
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.input = None;
        if self.exited.is_none() {
            if let Some(group) = self
                .master
                .process_group_leader()
                .filter(|group| *group > 0)
            {
                // SAFETY: openpty/spawn_command creates a separate session. Signal
                // its foreground job group as a real terminal hangup would.
                unsafe {
                    libc::kill(-group, libc::SIGHUP);
                }
            }
            let _ = self.killer.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vt_state_title_queries_and_alternate_screen_survive_chunked_output() {
        let mut parser = vt100::Parser::new_with_callbacks(4, 20, 20, Callbacks::default());
        for chunk in b"\x1b]2;build\x07\x1b[31mhi\x1b[0m\x1b[6n".chunks(2) {
            parser.process(chunk);
        }
        assert_eq!(parser.callbacks().title, "build");
        assert_eq!(parser.callbacks().replies, b"\x1b[1;3R");
        assert_eq!(
            parser.screen().cell(0, 0).unwrap().fgcolor(),
            vt100::Color::Idx(1)
        );
        parser.process(b"\x1b[?1049hother\x1b[?1049l");
        assert_eq!(parser.screen().contents(), "hi");
    }
    #[test]
    fn real_pty_has_correct_size_environment_and_reaps_child() {
        let (tx, rx) = mpsc::sync_channel(128);
        let mut session = Session::spawn(
            7,
            Path::new("/tmp"),
            &[
                "/bin/sh".into(),
                "-c".into(),
                "read line; stty size; printf '\\033[32m%s:%s:%s' \"$TERM\" \"$PWD\" \"$line\""
                    .into(),
            ],
            tx,
        )
        .unwrap();
        session.resize(32, 100).unwrap();
        session.write(b"hello\r".to_vec()).unwrap();
        loop {
            match rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap() {
                Event::Output(_, bytes) => session.process(&bytes).unwrap(),
                Event::Exited(_, status) => {
                    session.finish(status);
                    break;
                }
                Event::Configuration(_) => unreachable!(),
            }
        }
        let text = session.parser.screen().contents();
        assert!(text.contains("32 100"), "{text}");
        assert!(text.contains("xterm-256color:/tmp:hello"), "{text}");
    }
}
