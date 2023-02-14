use core::{fmt::Write, str::from_utf8};
// use hal::serial;
use embedded_io::asynch::{Read as AsyncRead, Write as AsyncWrite};
// use nb::block;
use heapless::Vec;

use log::{Metadata, Record};
use crate::autocomplete::Autocomplete;
use crate::history::History;
use crate::*;
use embassy_sync::pipe::Pipe;

pub type ShellResult = Result<(), ShellError>;
pub type SpinResult = Result<(), ShellError>;
// pub type PollResult<'a, S> = Result<Option<Input<'a>>, ShellError>;
type CS = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

pub struct AShell<S, A, H, const CMD_LEN: usize, const LOG_LEN:usize> 
where 
    S: AsyncRead + AsyncWrite,
    A: Autocomplete<CMD_LEN>,
    H: History<CMD_LEN>
{
    serial: S,
    autocomplete: A,
    history: H,
    // env: Environment,
    editor_buf: Vec<u8, CMD_LEN>,
    log_buffer: Pipe<CS,LOG_LEN>,
    editor_len: usize,
    cursor: usize,
    control: bool,
    escape: bool,
    autocomplete_on: bool,
    history_on: bool,
}

impl<S, A, H, const CMD_LEN: usize, const LOG_LEN: usize> AShell<S, A, H, CMD_LEN, LOG_LEN>
where
    S: AsyncRead + AsyncWrite,
    A: Autocomplete<CMD_LEN>,
    H: History<CMD_LEN>,
{
    pub fn new(serial: S, autocomplete: A, history: H) -> Self {
        Self {
            serial,
            autocomplete,
            history,
            // env,
            cursor: 0,
            editor_buf: Vec::new(),
            log_buffer: Pipe::new(),
            editor_len: 0,
            autocomplete_on: true,
            history_on: true,
            control: false,
            escape: false,
        }
    }

    pub fn autocomplete(&mut self, autocomplete_on: bool) {
        self.autocomplete_on = autocomplete_on;
    }

    pub fn history(&mut self, history_on: bool) {
        self.history_on = history_on;
    }

    pub fn get_autocomplete_mut(&mut self) -> &mut A {
        &mut self.autocomplete
    }

    pub fn get_history_mut(&mut self) -> &mut H {
        &mut self.history
    }

    pub fn get_serial_mut(&mut self) -> &mut S {
        &mut self.serial
    }

    pub fn reset(&mut self) {
        self.control = false;
        self.escape = false;
        self.cursor = 0;
        self.editor_len = 0;
    }

    // pub fn spin<E, ENV: Environment<S, A, H, E, CMD_LEN>>(
    //     &mut self,
    //     env: &mut ENV,
    // ) -> SpinResult<S, E> {
    //     loop {
    //         match self.poll() {
    //             Err(ShellError::WouldBlock) => return Ok(()),
    //             Err(err) => return Err(SpinError::ShellError(err)),
    //             Ok(None) => continue,
    //             Ok(Some(Input::Control(code))) => env.control(self, code)?,
    //             Ok(Some(Input::Command((cmd, args)))) => {
    //                 let mut cmd_buf = [0; CMD_LEN];
    //                 cmd_buf[..cmd.len()].copy_from_slice(cmd.as_bytes());
    //                 let cmd = core::str::from_utf8(&cmd_buf[..cmd.len()])?;

    //                 let mut args_buf = [0; CMD_LEN];
    //                 args_buf[..args.len()].copy_from_slice(args.as_bytes());
    //                 let args = core::str::from_utf8(&args_buf[..args.len()])?;

    //                 env.command(self, cmd, args)?
    //             }
    //         };
    //     }
    // }
    pub async fn tx_fut(&mut self)
    {
        let mut log_buf:[u8;32] = [0;32];
        loop {
            let len = self.log_buffer.read(&mut log_buf[..]).await;
            self.serial.write_all(&log_buf[..len]).await.unwrap();
        }

    }

    pub async fn rx_fut(&mut self, env: &mut impl Environment<S, A, H, CMD_LEN, LOG_LEN>) -> ShellResult
    {
        const ANSI_ESCAPE: u8 = b'[';

        let mut buf:[u8;32] = [0; 32];
        loop {
            let rx_len = self.serial.read(&mut buf).await.unwrap();

            for byte in &buf[..rx_len] {
                match *byte {
                    ANSI_ESCAPE if self.escape => {
                        self.control = true;
                    }
                    control::ESC => {
                        self.escape = true;
                    }
                    control_byte if self.control => {
                        self.escape = false;
                        self.control = false;

                        const UP: u8 = 0x41;
                        const DOWN: u8 = 0x42;
                        const RIGHT: u8 = 0x43;
                        const LEFT: u8 = 0x44;
                        match control_byte {
                            LEFT => self.dpad_left().await?,
                            RIGHT => self.dpad_right().await?,
                            UP => self.dpad_up().await?,
                            DOWN => self.dpad_down().await?,
                            _ => {}
                        }
                    }
                    _ if self.escape => {
                        self.escape = false;
                        self.control = false;
                    }
                    control::TAB => {
                        if self.autocomplete_on {
                            self.suggest().await?
                        } else {
                            self.bell().await?
                        }
                    }
                    control::DEL | control::BS => self.delete_at_cursor().await?,
                    control::CR => {
                        let mut cmd_buf = [0; CMD_LEN];
                        cmd_buf[..self.editor_len].copy_from_slice(&self.editor_buf[..self.editor_len]);
                        // let line = from_utf8(&self.editor_buf[..self.editor_len])?;
                        let line = from_utf8(&cmd_buf[..self.editor_len])?;
                        self.history
                            .push(line)
                            .map_err(|_| ShellError::HistoryError)?;
                        self.editor_len = 0;
                        self.cursor = 0;
                        let (cmd, args) = line.split_once(" ").unwrap_or((line, &""));
                        env.command(self, cmd, args).await?;
                    }
                    _ => {
                        let ch = *byte as char;
                        if ch.is_ascii_control() {
                            env.control(self, *byte).await?;
                        } else {
                            self.write_at_cursor(*byte).await?;
                        }
                    }
                };
            }
        }
    }

    pub fn clear(&mut self) -> ShellResult {
        self.cursor = 0;
        self.editor_len = 0;
        self.write_str("\x1b[H\x1b[2J")?;
        Ok(())
    }

    pub async fn bell(&mut self) -> ShellResult {
        // block!(self.serial.write(control::BELL)).map_err(ShellError::WriteError)
        match self.serial.write(&[control::BELL as u8]).await {
            Ok(_) => Ok(()),
            Err(_) => Err(ShellError::WriteError)
        }
    }

    pub fn push_history(&mut self, line: &str) -> ShellResult {
        self.history
            .push(line)
            .map_err(|_| ShellError::HistoryError)
    }

    async fn write_at_cursor(&mut self, byte: u8) -> ShellResult {
        if self.cursor == self.editor_buf.len() {
            self.bell().await?;
        } else if self.cursor < self.editor_len {
            self.serial.write(&[byte]).await.map_err(|_| ShellError::WriteError)?;

            self.editor_buf
                .copy_within(self.cursor..self.editor_len, self.cursor + 1);
            self.editor_buf[self.cursor] = byte;
            self.cursor += 1;
            self.editor_len += 1;

            self.write_str("\x1b[s\x1b[K")?;
            // for b in &self.editor_buf[self.cursor..self.editor_len] {
                // self.serial.write(*b).await.map_err(ShellError::WriteError)?;
            // }
            self.serial.write(&self.editor_buf[self.cursor..self.editor_len]).await.map_err(|_| ShellError::WriteError)?;
            self.write_str("\x1b[u")?;
        } else {
            self.editor_buf[self.cursor] = byte;
            self.cursor += 1;
            self.editor_len += 1;
            self.serial.write(&[byte]).await.map_err(|_| ShellError::WriteError)?;
        }
        Ok(())
    }

    async fn delete_at_cursor(&mut self) -> ShellResult {
        if self.cursor == 0 {
            self.bell().await?;
        } else if self.cursor < self.editor_len {
            self.editor_buf
                .copy_within(self.cursor..self.editor_len, self.cursor - 1);
            self.cursor -= 1;
            self.editor_len -= 1;
            self.write_str("\x1b[D\x1b[s\x1b[K")?;
            // for b in &self.editor_buf[self.cursor..self.editor_len] {
                // self.serial.write(*b).await.map_err(|_| ShellError::WriteError)?;
            // }
            self.serial.write(&self.editor_buf[self.cursor..self.editor_len]).await.map_err(|_| ShellError::WriteError)?;
            self.write_str("\x1b[u")?;
        } else {
            self.cursor -= 1;
            self.editor_len -= 1;
            self.write_str("\x08 \x08")?;
        }
        Ok(())
    }

    async fn dpad_left(&mut self) -> ShellResult {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.write_str("\x1b[D")?;
        } else {
            self.bell().await?;
        }
        Ok(())
    }

    async fn dpad_right(&mut self) -> ShellResult {
        if self.cursor < self.editor_len {
            self.cursor += 1;
            self.write_str("\x1b[C")?;
        } else {
            self.bell().await?;
        }
        Ok(())
    }

    async fn dpad_up(&mut self) -> ShellResult {
        if self.cursor != self.editor_len || !self.history_on {
            return self.bell().await;
        }
        match self.history.go_back() {
            None => self.bell().await,
            Some(line) => self.replace_editor_buf(line.as_str()),
        }
    }

    async fn dpad_down(&mut self) -> ShellResult {
        if self.cursor != self.editor_len || !self.history_on {
            return self.bell().await;
        }
        match self.history.go_forward() {
            None => self.bell().await,
            Some(line) => self.replace_editor_buf(line.as_str()),
        }
    }

    async fn suggest(&mut self) -> ShellResult {
        let prefix = from_utf8(&self.editor_buf[..self.cursor])?;
        match self.autocomplete.suggest(prefix) {
            None => self.bell().await?,
            Some(suffix) => {
                let bytes = suffix.as_bytes();
                self.editor_buf[self.cursor..(self.cursor + bytes.len())].copy_from_slice(bytes);
                self.cursor += bytes.len();
                self.editor_len = self.cursor;
                write!(self, "\x1b[K{}", suffix.as_str())?;
            }
        }
        Ok(())
    }

    fn replace_editor_buf(&mut self, line: &str) -> ShellResult {
        let cursor = self.cursor;
        if cursor > 0 {
            write!(self, "\x1b[{}D", cursor)?;
        }

        let bytes = line.as_bytes();
        self.editor_len = bytes.len();
        self.cursor = bytes.len();
        self.editor_buf[..bytes.len()].copy_from_slice(bytes);
        write!(self, "\x1b[K{}", line)?;
        Ok(())
    }
}

impl<S, A, H, const CMD_LEN: usize, const LOG_LEN: usize> core::fmt::Write for AShell<S, A, H, CMD_LEN, LOG_LEN>
where
    S: AsyncRead + AsyncWrite,
    A: Autocomplete<CMD_LEN>,
    H: History<CMD_LEN>,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.log_buffer.try_write(s.as_bytes())?;
        Ok(())
    }
}

impl<S, A, H, const CMD_LEN:usize, const LOG_SIZE:usize> log::Log for AShell<S, A, H, CMD_LEN, LOG_SIZE> 
where
    S: AsyncRead + AsyncWrite + Send + Sync,
    A: Autocomplete<CMD_LEN> + Send + Sync,
    H: History<CMD_LEN> + Send + Sync,
{
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let _ = write!(Writer(&self.log_buffer), "{}\r\n", record.args());
            // let _ = write!(self, "{}\r\n", record.args());
        }
    }

    fn flush(&self) {}
}

struct Writer<'d, const N: usize>(&'d Pipe<CS, N>);

impl<'d, const N: usize> core::fmt::Write for Writer<'d, N> {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        let _ = self.0.try_write(s.as_bytes());
        Ok(())
    }
}
