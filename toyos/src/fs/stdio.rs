use super::File;
use crate::memory::UserBuffer;
use crate::sbi::console_getchar;
use crate::task::suspend_and_rnext;
use core::fmt;
use alloc::boxed::Box;
use k210_hal::{clock::Clocks, fpioa, pac, prelude::*};
use lazy_static::*;
use spin::Mutex;
use embedded_hal::serial::{Read, Write};
use nb::block;
pub struct Stdin;
pub struct Stdout;

lazy_static!{
    pub static ref STDOUTLOCK:Mutex<usize> = Mutex::new(0);
    pub static ref STDINLOCK:Mutex<usize> = Mutex::new(0);
}

impl File for Stdin {
    fn readable(&self) -> bool { true }
    fn writable(&self) -> bool { false }
    fn read(&self, mut user_buf: UserBuffer) -> usize {
        let lock = STDINLOCK.lock();
        let mut c: usize;
        let mut count = 0;
        if user_buf.len() > 1{
            return 0;
        }
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_and_rnext();
                continue;
            } else {
                break;
            }
        }
        let ch = c as u8;
        unsafe { 
            user_buf.buffer[0].as_mut_ptr().write_volatile(ch);
        }
        return 1
    }
    fn write(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
}

impl File for Stdout {
    fn readable(&self) -> bool { false }
    fn writable(&self) -> bool { true }
    fn read(&self, _user_buf: UserBuffer) -> usize{
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        let lock = STDOUTLOCK.lock();
        for buffer in user_buf.buffer.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }
}

pub trait LegacyStdio: Send {
    fn getchar(&mut self) -> u8;
    fn putchar(&mut self, ch: u8);
}

struct EmbeddedHalSerial<T> {
    inner: T,
}

impl<T> EmbeddedHalSerial<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Send> LegacyStdio for EmbeddedHalSerial<T>
where T: Read<u8> + Write<u8>,
{
    fn getchar(&mut self) -> u8 {
        block!(self.inner.read()).ok().unwrap()
    }
    fn putchar(&mut self, ch: u8) {
        block!(self.inner.write(ch)).ok();
        block!(self.inner.flush()).ok();
    }
}

struct Fused<T, R>(T, R);

impl<T, R> LegacyStdio for Fused<T, R>
where
    T: Write<u8> + Send + 'static,
    R: Read<u8> + Send + 'static,
{
    fn getchar(&mut self) -> u8 {
        block!(self.1.read()).ok().unwrap()
    }
    fn putchar(&mut self, ch: u8) {
        block!(self.0.write(ch)).ok();
        block!(self.0.flush()).ok();
    }
}

lazy_static::lazy_static! {
    static ref LEGACY_STDIO: Mutex<Option<Box<dyn LegacyStdio>>> =
        Mutex::new(None);
}

#[cfg(feature = "board_qemu")]
pub fn init(){
    let serial = crate::drivers::Ns16550a::new(0x10000000, 0 );
    init_legacy_stdio_embedded_hal(serial);
}

#[cfg(feature = "board_k210")]
pub fn init(){
    println!("0");
    let p = pac::Peripherals::take().unwrap();
    println!("1");
    let mut sysctl = p.SYSCTL.constrain();
    println!("2");
    let fpioa = p.FPIOA.split(&mut sysctl.apb0);
    println!("3");
    let clocks = Clocks::new();
    println!("4");
    let _uarths_tx = fpioa.io5.into_function(fpioa::UARTHS_TX);
    let _uarths_rx = fpioa.io4.into_function(fpioa::UARTHS_RX);
    println!("5");
    let serial = p.UARTHS.configure(115_200.bps(), &clocks);
    let (tx, rx) = serial.split();
    println!("6");
    init_legacy_stdio_embedded_hal_fuse(tx, rx);
}

#[doc(hidden)]
pub fn init_legacy_stdio_embedded_hal<T: Read<u8> + Write<u8> + Send + 'static>(serial: T) {
    let serial = EmbeddedHalSerial::new(serial);
    *LEGACY_STDIO.lock() = Some(Box::new(serial));
}

#[doc(hidden)]
pub fn init_legacy_stdio_embedded_hal_fuse<T, R>(tx: T, rx: R)
where
    T: Write<u8> + Send + 'static,
    R: Read<u8> + Send + 'static,
{
    let serial = Fused(tx, rx);
    *LEGACY_STDIO.lock() = Some(Box::new(serial));
}

pub(crate) fn legacy_stdio_putchar(ch: u8) {
    if let Some(stdio) = LEGACY_STDIO.lock().as_mut() {
        stdio.putchar(ch)
    }
}

pub(crate) fn legacy_stdio_getchar() -> u8 {
    if let Some(stdio) = LEGACY_STDIO.lock().as_mut() {
        stdio.getchar()
    } else {
        0
    }
}

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(stdio) = LEGACY_STDIO.lock().as_mut() {
            for byte in s.as_bytes() {
                stdio.putchar(*byte)
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    Stdout.write_fmt(args).unwrap();
}

