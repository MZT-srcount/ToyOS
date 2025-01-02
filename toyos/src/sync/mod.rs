mod up;
mod mutex;

pub use up::UPSafeCell;
pub use mutex::*;

pub fn test(){
}
/*
 *===============SpinMutex==================
 *
 *pub struct SpinMutex{
    state : STATE
 }
 *
 * SpinMutex对外开放接口：
 * 
 /*新建一个自旋锁*/
  pub fn new() -> Self{}
 
 /*上锁*/
  pub fn lock(&mut self){}
  
 /*解锁*/
  pub fn unlock(&mut self){}
 *
 *================================================
 * 
 *==============自旋锁SpinLock<T>================
 *
 *pub struct SpinLock<T: ?Sized>{
    state : STATE,//自旋锁状态
    data  : UpSafeCell<T>,//自旋锁封装的数据
  }
 *
 *SpinLock<T>对外开放接口：
 *
 /*新建一个自旋锁，data输入需要封装的变量，返回已被SpinMutex封装的变量*/
  pub fn new(data : T) -> SpinMutex<T>
 *
 /*加锁,返回可修改变量，若已加锁会进入忙等待，使用不当可能造成死锁！！！*/
  pub fn lock(&mut self) -> RefMut<'_, T>
 *
 /*解锁*/
 pub fn unlock(&mut self)
 *
 */

