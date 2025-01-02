use core::ptr::NonNull;

use alloc::vec::Vec;
use lazy_static::*;

//获取当前加载的应用数量
pub fn get_num_app() -> usize {
    extern "C" { fn _num_app(); }
    let a : usize = 10;
    unsafe {
    (_num_app as usize as *const usize).read_volatile()}
}
/*读取用户程序数据*/
pub fn load_app(app_id: usize) -> &'static [u8] {
     extern "C"{ fn _num_app();}
     let num_ptr = _num_app as usize as *const usize;
     let num = get_num_app();
     let app_start = unsafe{
         core::slice::from_raw_parts(num_ptr.add(1), num + 1)
     };
     assert!(app_id < num);
     unsafe{
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id]
        )
     }
}


lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        //获取所有应用的名称，并且存储到Vec数据类型中
        let num_app = get_num_app();
        extern "C" { fn _app_names(); }
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                //每个应用程序以\0标记结束，注意是一个个字节进行读取的
                while end.read_volatile() != '\0' as u8 {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8(slice).unwrap();
                v.push(str);
                start = end.add(1);
            }
        }
        v
    };
}


#[allow(unused)]
pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    let num_app = get_num_app();
    let res = (0..num_app)
        .find(|&i| {
            APP_NAMES[i] == name})
        .map(load_app);
    if res == None {
        println!("can not find initproc..");
    }
    res
}

//打印数本次加载的所有程序
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in APP_NAMES.iter() {
        println!("{}", app);
    }
    println!("**************/");
}

