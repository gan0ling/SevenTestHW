#![no_std]
#![deny(unsafe_code)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

// extern crate embedded_hal as hal;
extern crate heapless;
extern crate nb;
extern crate uluru;
extern crate embedded_io;

use core::{fmt, str::Utf8Error};
use embedded_io::asynch::{Read as AsyncRead, Write as AsyncWrite};

pub mod autocomplete;
pub mod control;
pub mod history;

mod shell;

pub use shell::*;

pub enum ShellError 
{
    ReadError,
    WriteError,
    HistoryError,
    FormatError(fmt::Error),
    ExecuteError(i32),
    BadInputError(Utf8Error),
}

impl From<Utf8Error> for ShellError
{
    fn from(err:Utf8Error) -> Self {
        ShellError::BadInputError(err)
    }
}

impl From<fmt::Error> for ShellError
{
    fn from(err:fmt::Error) -> Self {
        ShellError::FormatError(err)
    }
}

impl From<i32> for ShellError
{
    fn from(err:i32) -> Self {
        ShellError::ExecuteError(err)
    }
}

pub enum Input<'a> {
    Control(u8),
    Command((&'a str, &'a str)),
}

pub trait Environment<S, A, H, const CMD_LEN: usize, const LOG_SIZE: usize>
where
    S: AsyncRead + AsyncWrite,
    A: autocomplete::Autocomplete<CMD_LEN>,
    H: history::History<CMD_LEN>,
{
    async fn command(
        &mut self,
        shell: &mut AShell<S, A, H, CMD_LEN, LOG_SIZE>,
        cmd: &str,
        args: &str,
    ) -> Result<(), i32>;

    async fn control(
        &mut self, 
        shell: &mut AShell<S, A, H, CMD_LEN, LOG_SIZE>, 
        code: u8
    ) -> Result<(), i32>;
}

// pub struct Serial<T, TX: Write, RX: Read> {
//     w: PhantomData<T>,
//     tx: TX,
//     rx: RX,
// }

// impl<TX: Write, RX: Read> Serial<TX, RX> {
//     pub fn from_parts(tx: TX, rx: RX) -> Self {
//         Self {
//             tx,
//             rx,
//             w: PhantomData,
//         }
//     }

//     pub fn tx(&mut self) -> &mut TX {
//         &mut self.tx
//     }

//     pub fn rx(&mut self) -> &mut RX {
//         &mut self.rx
//     }

//     pub fn split(self) -> (TX, RX) {
//         (self.tx, self.rx)
//     }
// }

// impl<W, TX: Write, RX: Read> Write for Serial<TX, RX> {
    // type Error = TX::Error;

    // fn write(&mut self, word: W) -> nb::Result<(), Self::Error> {
        // self.tx.write(word)
    // }

    // fn flush(&mut self) -> nb::Result<(), Self::Error> {
        // self.tx.flush()
    // }
// }

// impl<W, TX: Write<W>, RX: Read<W>> Read<W> for Serial<W, TX, RX> {
    // type Error = RX::Error;

    // fn read(&mut self) -> nb::Result<W, Self::Error> {
        // self.rx.read()
    // }
// }
