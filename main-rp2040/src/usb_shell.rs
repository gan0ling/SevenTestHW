use core::fmt::Write as _;
use embassy_futures::select::{select, Either};
use embassy_futures::join::join;
use embassy_sync::pipe::Pipe;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::Driver;
use embassy_usb::{Builder, Config};
use log::{Metadata, Record};
use crate::shell::CmdParser;

type CS = embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

/// The logger state containing buffers that must live as long as the USB peripheral.
pub struct LoggerState<'d> {
    state: State<'d>,
    device_descriptor: [u8; 32],
    config_descriptor: [u8; 128],
    bos_descriptor: [u8; 16],
    control_buf: [u8; 64],
}

impl<'d> LoggerState<'d> {
    /// Create a new instance of the logger state.
    pub fn new() -> Self {
        Self {
            state: State::new(),
            device_descriptor: [0; 32],
            config_descriptor: [0; 128],
            bos_descriptor: [0; 16],
            control_buf: [0; 64],
        }
    }
}

/// The logger handle, which contains a pipe with configurable size for buffering log messages.
pub struct UsbShell<const N: usize> {
    buffer: Pipe<CS, N>,
}

impl<const N: usize> UsbShell<N> {
    /// Create a new logger instance.
    pub const fn new() -> Self {
        Self { 
            buffer: Pipe::new(),
        }
    }

    /// Run the USB logger using the state and USB driver. Never returns.
    pub async fn run<'d, D>(&'d self, state: &'d mut LoggerState<'d>, driver: D) -> !
    where
        D: Driver<'d>,
        Self: 'd,
    {
        let mut cmd_parser :CmdParser<256> = CmdParser::new();
        const MAX_PACKET_SIZE: u8 = 64;
        let mut config = Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial logger");
        config.serial_number = None;
        config.max_power = 100;
        config.max_packet_size_0 = MAX_PACKET_SIZE;

        // Required for windows compatiblity.
        // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
        config.device_class = 0xEF;
        config.device_sub_class = 0x02;
        config.device_protocol = 0x01;
        config.composite_with_iads = true;

        let mut builder = Builder::new(
            driver,
            config,
            &mut state.device_descriptor,
            &mut state.config_descriptor,
            &mut state.bos_descriptor,
            &mut state.control_buf,
            None,
        );

        // Create classes on the builder.
        let mut class = CdcAcmClass::new(&mut builder, &mut state.state, MAX_PACKET_SIZE as u16);

        // Build the builder.
        let mut device = builder.build();

        loop {
            let run_fut = device.run();
            let log_fut = async  {
                let mut log_buf: [u8; MAX_PACKET_SIZE as usize] = [0; MAX_PACKET_SIZE as usize];
                let mut recv_buf: [u8; MAX_PACKET_SIZE as usize] = [0; MAX_PACKET_SIZE as usize];
                loop {
                    class.wait_connection().await;
                    // let len = self.buffer.read(&mut log_buf[..]).await;
                    // let _ = class.write_packet(&log_buf[..len]).await;

                    match select(self.buffer.read(&mut log_buf[..]), class.read_packet(&mut recv_buf[..])).await {
                        Either::First(n) => {
                            let _ = class.write_packet(&log_buf[..n]).await;
                        },
                        Either::Second(Ok(n)) => {
                            //process cmd
                            cmd_parser.parse(&recv_buf[..]);
                        },
                        _ => {},
                    }
                }
            };
            join(run_fut, log_fut).await;
        }
    }
}

impl<const N: usize> log::Log for UsbShell<N> {
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
