#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use ashell::{ShellResult,Environment, autocomplete::{StaticAutocomplete, Autocomplete}, history::{LRUHistory, History}, AShell};
use embassy_rp::peripherals::UART0;
use embassy_rp::uart::{BufferedUart, BufferedUartRx, BufferedUartTx, Config};
use embassy_executor::_export::StaticCell;
use embedded_io::asynch::{Read as AsyncRead, Write as AsyncWrite};


pub const MAX_CMD_LEN:usize = 64;
pub const TOTAL_CMDS:usize = 2;
pub const LOG_BUFF_SIZE:usize = 1024;

pub static CMD_LIST:[&str;TOTAL_CMDS] = [
    "help",
    "pwmin"
];

pub type SevenShell<'a> = AShell<StaticAutocomplete<TOTAL_CMDS>, LRUHistory<MAX_CMD_LEN, TOTAL_CMDS>, MAX_CMD_LEN, LOG_BUFF_SIZE>;


#[derive(Default)]
pub struct SevenShellEnv;

impl<A, H, const CMD_LEN:usize, const LOG_LEN:usize> Environment<A, H, CMD_LEN, LOG_LEN> for SevenShellEnv 
where
    // S: AsyncRead + AsyncWrite,
    A: Autocomplete<CMD_LEN>,
    H: History<CMD_LEN>
{

    async fn command(
        &mut self,
        shell: &mut AShell<A, H, CMD_LEN, LOG_LEN>,
        cmd: &str,
        args: &str,
    ) -> ShellResult 
    {
        Ok(())
    }

    async fn control(
        &mut self, 
        shell: &mut AShell<A, H, CMD_LEN, LOG_LEN>, 
        code: u8
    ) -> ShellResult
    {
        Ok(())
    }
}

// #[embassy_executor::task]
// pub async fn shell_task(ser:BufferedUart<'_, UART0>) {
//     let env = SevenShellEnv::default();
//     let history = LRUHistory::default();
//     let completer = StaticAutocomplete(CMD_LIST);
//     let shell:SevenShell = AShell::new(ser, completer, history);
//     let logger:&'static mut SevenShell = LOGGER.init(shell);
//     unsafe {
//             let _ = ::log::set_logger_racy(logger).map(|()| log::set_max_level(log::LevelFilter::Info));
//     }
// }
