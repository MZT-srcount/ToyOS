use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;

/*
 * 实现Write特性，以使用write_fmt方法来实现println!宏的字符串打印函数
 */
impl Write for Stdout {
    fn write_str(&mut self, s: &str) ->fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

pub fn log(args: fmt::Arguments) {       
    Stdout.write_fmt(args).unwrap();
}

/*
 * 使用macro_rules!创建宏
 * 使用正则表达式调用print函数打印字符串，包括print和println
 */

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

/*
 * ****************实现log输出***********************
 * 根据std中的INFO、WARN、INFO、DEBUG、TRACE宏进行重定义，并实现彩色输出：
 * macro_rules ：宏声明？
 * 说明：$fmt，用于格式化和打印String，其中$用于捕捉宏,声明：
 *       fn fmt(&self, f: &mut fmt::Formatter)
 *       format_args!, 为其他字符串格式构造参数，格式：($fmt : expr)、($fmt   : expr, $($args : tt) *)
 *       \x1b[31m, 31为颜色编号，格式：\x1b[{}m
 *       concat!进行字符串拼接
 *
 * 详情参考Rust文档
 *  ***************************************************
 */

#[macro_export]
macro_rules! info {
    ($fmt: literal $(, target: $target:expr, $($arg:tt)*)?) => {
        $crate::console::log(format_args!("\x1b[31m[INFO]:{}\x1b[0m", format_args!(concat!($fmt, "\n") $(, target: $target, $($arg)*)?)));
    };
    ($fmt: literal $(, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[31m[INFO]:", $fmt, "\x1b[0m\n") $(, $($arg)*)?));
    };
}

#[macro_export]
macro_rules! error {
    ($fmt: literal $(, target: $target:expr, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[34m[ERROR]:", $fmt,"\x1b[0m\n") $(, $($arg)*)?));
    };
    ($fmt: literal $(, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[34m[ERROR]:", $fmt,"\x1b[0m\n") $(, $($arg)*)?));
    };
}

#[macro_export]
macro_rules! warn {
    ($fmt: literal $(, target: $target:expr, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[93m[WARN]:", $fmt, "\x1b[0m\n") $(, target: $target, $($arg)*)?));
    };
    ($fmt: literal $(, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[93m[WARN]:", $fmt, "\x1b[0m\n") $(, $($arg)*)?));
    };
}

#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, target: $target:expr, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[32m[DEBUG]:", $fmt, "\x1b[0m\n") $(, target: $target, $($arg)*)?));
    };
    ($fmt: literal $(, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[32m[DEBUG]:", $fmt, "\x1b[0m\n") $(, $($arg)*)?));
    };
}


#[macro_export]
macro_rules! trace {
    ($fmt: literal $(, target: $target:expr, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[31m[TRACE]:", $fmt, "\x1b[0m\n") $(, target: $target, $($arg)*)?));
    };
    ($fmt: literal $(, $($arg:tt)*)?) => {
        $crate::console::log(format_args!(concat!("\x1b[31m[TRACE]:", $fmt, "\x1b[0m\n") $(, $($arg)*)?));
    };
}
