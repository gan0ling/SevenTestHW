use core::fmt::Write as _;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_executor::_export::StaticCell;
use embassy_futures::join::join;
use embassy_futures::join;
use embassy_rp::interrupt;
use embassy_time::{Duration, Timer};
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::{BufferedUart, BufferedUartRx, BufferedUartTx, Config};
// use embassy_time::{Duration, Timer};
use embedded_io::asynch::{Read, Write};
use embassy_sync::pipe::Pipe;
use log::{Metadata, Record};
// use crate::shell::CmdParser;
use crate::shell::{shell_command_cb, shell_print_cb};
use crate::esh::*;

// type CS = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
type CS = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;


pub struct UartShell<const N:usize> {
    buffer: Pipe<CS,N>,
}

#[embassy_executor::task]
async fn rx_fut(mut rx:BufferedUartRx<'static, UART0>) {
    let mut rx_buf:[u8; 32] = [0;32];
    // let mut cmd_parser :CmdParser = CmdParser::new();
    let esh = Esh::init().unwrap(); 
    esh.register_command(shell_command_cb);
    esh.register_print(shell_print_cb);

    loop {
        let len = rx.read(&mut rx_buf).await.unwrap();
        for i in 0..len {
            esh.rx(rx_buf[i]);
        }
    }
}


impl<const N: usize> UartShell<N> {
    pub const fn new() -> Self {
        Self {
            buffer: Pipe::new(),
        }
    }

    pub async fn run(&self, spawner: Spawner, mut rx: BufferedUartRx<'static, UART0>, mut tx: BufferedUartTx<'static, UART0>) {
        // join(rx_fut, tx_fut).await;
        // Spawner::for_current_executor().await.spawn(rx_fut(rx)).unwrap();
        spawner.spawn(rx_fut(rx)).unwrap();

        let mut log_buf:[u8;32] = [0;32];
        loop {
            let len = self.buffer.read(&mut log_buf[..]).await;
            tx.write_all(&log_buf[..len]).await.unwrap();
        }
    }
}

impl<const N: usize> log::Log for UartShell<N> {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let _ = write!(Writer(&self.buffer), "{}\r\n", record.args());
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
