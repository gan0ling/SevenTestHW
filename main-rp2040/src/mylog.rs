use core::fmt::Write;
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::BufferedUartTx;
use embassy_sync::pipe::{Pipe, Reader as PipeReader, Writer as PipeWriter};
use embedded_io::asynch::{Read as AsyncRead, Write as AsyncWrite};
// use embassy_rp::uart::{BufferedUart, BufferedUartRx, BufferedUartTx};

type CS = embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

const LOG_BUFF_SIZE:usize = 1024;

pub static LOG_PIPE: Pipe<CS, LOG_BUFF_SIZE> = Pipe::new();

struct MyWriter<'d, const N: usize>(&'d Pipe<CS, N>);

impl<'d, const N: usize> core::fmt::Write for MyWriter<'d, N> {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        let _ = self.0.try_write(s.as_bytes());
        Ok(())
    }
}

struct MyLogger;
impl MyLogger {
    pub const fn new() -> Self {
        Self
    }
}

impl log::Log for MyLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
       if self.enabled(record.metadata()) {
            let _ = write!(MyWriter(&LOG_PIPE), "{}\n", record.args());
        } 
    }

    fn flush(&self) {
        
    }
}

pub fn init_log() {
    static LOGGER:MyLogger = MyLogger::new(); 
    unsafe {
        let _ = ::log::set_logger_racy(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Info));
    }
}

#[embassy_executor::task]
pub async fn log_task(mut tx: BufferedUartTx<'static, UART0>)
{
    //read data from LOG_PIPE and write to uart    
    let mut log_buf:[u8;32] = [0;32];
    // let reader = LOG_PIPE.reader();
    loop {
        let len = LOG_PIPE.read(&mut log_buf).await;
        tx.write_all(&log_buf[..len]).await.unwrap();
    }
}
