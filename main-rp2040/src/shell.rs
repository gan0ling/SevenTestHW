#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::{cell::RefCell, f32::consts::E};
use core::str::FromStr;
use heapless::String;
use ashell::{
                ShellResult,Environment, 
                autocomplete::{FnAutocomplete, Autocomplete}, 
                history::{LRUHistory, History}, AShell
            };
use heapless::FnvIndexMap;
use embassy_sync::{blocking_mutex::ThreadModeMutex, mutex::MutexGuard};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
// use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::Mutex;

use crate::mylog::LOG_PIPE;
// use embassy_sync::blocking_mutex::CriticalSectionMutex;

// type ShellMutex = ThreadModeRawMutex;


pub const MAX_CMD_LEN:usize = 64;
pub const TOTAL_CMDS:usize = 16;
pub const LOG_BUFF_SIZE:usize = 1024;

// pub static CMD_LIST:[&str;TOTAL_CMDS] = [
    // "help",
    // "pwmin"
// ];

pub type SevenShell = AShell<FnAutocomplete<MAX_CMD_LEN>, LRUHistory<MAX_CMD_LEN, TOTAL_CMDS>, MAX_CMD_LEN, LOG_BUFF_SIZE>;

pub static mut SHELL_ENV: SevenShellEnv<TOTAL_CMDS> = SevenShellEnv::new();

// pub struct SevenShellEnv<'a, const N: usize> {
    // env_map: FnvIndexMap<&'static str, &'a mut dyn Environment, N>,
    // cmd_names: [&'static str; N],
// }

type CmdHandler = fn(&str, &str) -> ShellResult;
// type CmdHandler = impl Fn(&str, &str) -> ShellResult;
pub struct SevenShellEnv<const N:usize> 
{
    inner: Mutex<ThreadModeRawMutex, RefCell<FnvIndexMap<&'static str, CmdHandler, N>>>
}

unsafe impl<const N:usize> Sync for  SevenShellEnv<N> {}

impl<const N: usize> SevenShellEnv<N> 
{
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(RefCell::new(FnvIndexMap::new())),
        }
    }

    // pub async fn lock(&mut self) -> MutexGuard<ThreadModeRawMutex, RefCell<FnvIndexMap<&'static str, CmdHandler, N>>> {
    pub fn lock<R>(&self, f: impl FnOnce(&RefCell<FnvIndexMap<&'static str, CmdHandler, N>>)->R) -> R {
        // self.inner.lock().await
        self.inner.lock(f)
    }

    pub fn register_cmd(&mut self, cmd_name: &'static str, handler: CmdHandler){
        // let env_map = self.inner.lock().await;
        // let env_map = &mut *env_map.borrow_mut();
        // env_map.insert(cmd_name, handler);
        self.inner.lock(|map| {
            let mut map = map.borrow_mut();
            map.insert(cmd_name, handler);
        })
    }

    pub fn unregister_cmd(&mut self, cmd_name: &'static str) {
        // let env_map = self.inner.lock().await;
        // let env_map = &mut *env_map.borrow_mut();
        // env_map.remove(cmd_name);
        self.inner.lock(|map| {
            let mut map = map.borrow_mut();
            map.remove(cmd_name);
        });
    }

}

pub fn register_shell_cmd(name: &'static str, handler: CmdHandler)
{
    unsafe { SHELL_ENV.register_cmd(name, handler); }
}

pub fn unregister_shell_cmd(name: &'static str) {
    unsafe { SHELL_ENV.unregister_cmd(name); }
}

// impl<A, H, const CMD_LEN:usize, const LOG_LEN:usize> Environment<A, H, CMD_LEN, LOG_LEN> for SevenShellEnv 
// where
//     // S: AsyncRead + AsyncWrite,
//     A: Autocomplete<CMD_LEN>,
//     H: History<CMD_LEN>
impl<const N:usize> Environment for SevenShellEnv<N>
{

    async fn command(
        &mut self,
        // _shell: &mut AShell<A, H, CMD_LEN, LOG_LEN>,
        cmd: &str,
        args: &str,
    ) -> ShellResult 
    {
        // match cmd {
        //     "help" => log::info!("help for cmds"),
        //     "pwmin" => {
        //         //create pwmin task
        //         // let mut pins :[u8;8]= [0xFF;8];
        //         args.split_ascii_whitespace().map(|a| {a.parse::<u32>().unwrap()}).for_each(|pin| {
        //                 log::info!("create pio task for pin:{}", pin);
        //         });
        //         // log::info!("pins:{:?}-{:?}", pins.next(), pins.next());
        //         // let p = unsafe {Peripherals::steal()};
        //         // let pio0 = p.PIO0;
        //         // let pio1 = p.PIO1;
        //     },
        //     _ => log::info!("unknown cmd"),
        // }

        // let map= self.inner.lock().await;
        // let map = &mut *map.borrow_mut();
        // if let Some(handler) = map.get_mut(cmd) {
            // handler.command(cmd, args)
            // handler(cmd, args)
        // }
        // else
        // {
            // log::info!("unkonwn cmd");
            // Err(ashell::ShellError::CommandNotFound)
        // }

        self.inner.lock(|map| {
            let map = map.borrow();
            if let Some(handler) = map.get(cmd) {
                handler(cmd, args)
            }
            else {
                log::info!("unknown cmd");
                Err(ashell::ShellError::CommandNotFound)
            }
        })
    }

    async fn control(
        &mut self, 
        // _shell: &mut AShell<A, H, CMD_LEN, LOG_LEN>, 
        code: u8
    ) -> ShellResult
    {
        // let map= self.inner.lock().await;
        // let map = &mut *map.borrow_mut();
        // if let Some(handler) = map.get_mut(cmd) {
        //     handler.borrow_mut().control(code)
        // }
        // else
        // {
        //     Err(ashell::ShellError::KeyNotFound)
        // }
        Ok(())
    }
}

fn shell_cmd_complete(prefix: &str) -> Option<String<MAX_CMD_LEN>> {
    if prefix.len() == 0 {
        return None;
    }

    // let env = SHELL_ENV.lock().await;
    // let env = env.borrow();

    // for cmd_name in env.keys() {
    //     if cmd_name.starts_with(prefix) {
    //         let (_, suffix) = cmd_name.split_at(prefix.len());
    //         return String::from_str(suffix).ok();
    //     }
    // }
    // None
    unsafe {
        SHELL_ENV.lock(|map| {
            let map = map.borrow();
            for cmd_name in map.keys() {
                if cmd_name.starts_with(prefix) {
                    let (_, suffix) = cmd_name.split_at(prefix.len());
                    return String::from_str(suffix).ok();
                }
            }
            None
        })
    }
}

pub async fn create_shell() -> SevenShell {
    SevenShell::new(
        FnAutocomplete(shell_cmd_complete),
        LRUHistory::default(),
        &LOG_PIPE
    ).await
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
