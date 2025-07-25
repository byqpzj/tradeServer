use encoding_rs::GBK;
use libloading::{Library, Symbol};
use serde::Deserialize;
use serde_json::Value;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_short, c_void};
use std::{ptr};
use once_cell::sync::OnceCell;
// ----------------------
// 客户端连接对象
// ----------------------
static LIBRARY: OnceCell<Library> = OnceCell::new();
/// unsafe extern "stdcall" –  表示 x86_32 上 Win32 API 
static LOGON_FN: OnceCell<unsafe extern "stdcall" fn(c_int, *const c_char, c_short, *const c_char, *const c_char, c_short, *const c_char, *const c_char, *const c_char, bool, *mut c_char) -> *mut c_void> = OnceCell::new();
static QUERY_DATA_FN: OnceCell<unsafe extern "stdcall" fn(*mut c_void, c_int, *mut c_char) -> c_int> = OnceCell::new();
static SEND_ORDER_FN: OnceCell<unsafe extern "stdcall" fn(*mut c_void, c_int, *const c_char, *const c_char, f32, c_int, *mut c_char) -> c_int> = OnceCell::new();
static CANCEL_ORDER_FN: OnceCell<unsafe extern "stdcall" fn(*mut c_void, *const c_char, *mut c_char) -> c_int> = OnceCell::new();
static QUERY_HISTORY_DATA_FN: OnceCell<unsafe extern "stdcall" fn(*mut c_void, c_int, *const c_char, *const c_char, *mut c_char) -> c_int> = OnceCell::new();

///                        {  \n \t  "   d  a  t  a  " 空格 : 空格 n   u   l   l  \n  }  \n
static DATA_NULL:&[u8] = &[123,10,9,34,100,97,116,97,34,32,58,32,110,117,108,108,10,125,10];

static DATA_PREFIX:&[u8] = &[123,10,9,34,100,97,116,97,34];

pub struct ThsClient {
    lib: &'static Library,
    client_id: *mut c_void,
}

// ----------------------
// 核心方法实现
// ----------------------
impl ThsClient {
    // 初始化方法
    pub fn new(dll_path: &str) -> Result<Self, String> {
        let lib: &Library  = LIBRARY.get_or_init(|| unsafe {Library::new(dll_path).unwrap()});
        Ok(Self {
            lib,
            client_id: ptr::null_mut(),
        })
    }

    // 登录
    pub fn logon(&mut self,server: &Server,account: &Account, address: &Address) -> Result<(), Vec<u8>> {
        let logon = LOGON_FN.get_or_init(|| unsafe {*self.lib.get(b"Logon").unwrap()});
        let mut buffer = vec![0u8; 1024];
        let client = unsafe {
            logon(
                server.qsid,
                to_cstr(&address.host)?.as_ptr(),
                address.port,
                to_cstr(&*server.version)?.as_ptr(),
                to_cstr(&account.yyb_id)?.as_ptr(),
                0,
                to_cstr(&*account.account)?.as_ptr(),
                to_cstr(&*account.password)?.as_ptr(),
                to_cstr(&*account.comm_password)?.as_ptr(),
                false,
                buffer.as_mut_ptr() as *mut c_char,
            )
        };

        if client.is_null() {
            Err(buffer)
        } else {
            self.client_id = client;
            Ok(())
        }
    }

    /// 查询交易数据 0 资金，1 持仓，2 当日委托，3 当日成交，4 委托可撤，5 股东账户
    pub fn query_data(&self, category: i32) -> Result<Vec<u8>, Vec<u8>> {
        let query_data = QUERY_DATA_FN.get_or_init(|| unsafe {*self.lib.get(b"QueryData").unwrap()});
        let mut buffer = vec![0u8; 1024 * 512];
        let ret = unsafe { query_data(self.client_id, category, buffer.as_mut_ptr() as *mut c_char) };
        if ret > 0 { Ok(buffer) } else { Err(buffer) }
    }

    // 委托下单
    pub fn send_order(&self, category: i32, gddm: &str, gpdm: &str, price: f32, quantity: i32) -> Result<Vec<u8>, Vec<u8>> {
        let send_order = SEND_ORDER_FN.get_or_init(|| unsafe {*self.lib.get(b"SendOrder").unwrap()});
        let mut buffer = vec![0u8; 1024];
        let ret = unsafe {
            send_order(
                self.client_id,
                category,
                to_cstr(gddm)?.as_ptr(),
                to_cstr(gpdm)?.as_ptr(),
                price,
                quantity,
                buffer.as_mut_ptr() as *mut c_char,
            )
        };
        if ret > 0 { Ok(buffer) } else { Err(buffer) }
    }

    // 撤单
    pub fn cancel_order(&self, order_id: &str) -> Result<Vec<u8>, Vec<u8>> {
        // let func: Symbol<
        //     unsafe extern "stdcall" fn(*mut c_void, *const c_char, *mut c_char) -> c_int,
        // > = unsafe { self.lib.get(b"CancelOrder").map_err(|e| format!("获取 CancelOrder 函数失败: {}", e))? };

        let cancel_order = CANCEL_ORDER_FN.get_or_init(|| unsafe {*self.lib.get(b"CancelOrder").unwrap()});
        let mut buffer = vec![0u8; 1024];
        let ret = unsafe {
            cancel_order(
                self.client_id,
                to_cstr(order_id)?.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
            )
        };
        if ret > 0 { Ok(buffer) } else { Err(buffer) }
    }

    // 查询历史数据
    pub fn query_history_data(
        &self,
        category: i32,
        begin_date: &str,
        end_date: &str,
    ) -> Result<Vec<u8>, Vec<u8>> {
        // let query_history: Symbol<
        //     unsafe extern "stdcall" fn(
        //         *mut c_void,
        //         c_int,
        //         *const c_char,
        //         *const c_char,
        //         *mut c_char,
        //     ) -> c_int,
        // > = unsafe {
        //     self.lib
        //         .get(b"QueryHistoryData")
        //         .map_err(|e| format!("获取 QueryHistoryData 函数失败: {}", e))?
        // };
        let query_history = QUERY_HISTORY_DATA_FN.get_or_init(|| unsafe {*self.lib.get(b"QueryHistoryData").unwrap()});
        let mut buffer = vec![0u8; 1024*1024*2];
        let ret = unsafe {
            query_history(
                self.client_id,
                category,
                to_cstr(begin_date)?.as_ptr(),
                to_cstr(end_date)?.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
            )
        };
        if ret > 0 { Ok(buffer) } else { Err(buffer) }
    }
}


// 如果 DLL 是线程安全的
unsafe impl Send for ThsClient {}
unsafe impl Sync for ThsClient {}
// 如果 DLL 是线程安全的

// ----------------------
// 自动资源清理
// ----------------------
impl Drop for ThsClient {
    fn drop(&mut self) {
        if !self.client_id.is_null() {
            unsafe {
                let logoff: Symbol<unsafe extern "stdcall" fn(*mut c_void)> =
                    self.lib.get(b"Logoff").unwrap();
                logoff(self.client_id);
            }
        }
    }
}

// ----------------------
// 公共工具函数
// ----------------------

/// 处理API返回缓冲区的公共逻辑
pub fn process_buffer(buffer: &[u8]) -> Value {
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    let bytes = cstr.to_bytes();
    // 直接用 DATA_NULL 判断
    if  bytes.len() == 0 || bytes==DATA_NULL {
        return Value::Null;
    }
    // 这里使用 &bytes[13..bytes.len() - 2] 去掉了前面的 {\n\t"data" : 11个字符 和最后的 \n} 两个字符 
    let (text, _, _had_errors) = GBK.decode(
        if bytes.len() > 10 && &bytes[..9] == DATA_PREFIX  {
            &bytes[11..bytes.len() - 2]
        } else { 
            bytes
        }
    );
    // 解析 JSON 字符串
    let value: Value = serde_json::from_str(&*text).unwrap();
    value
}

pub fn process_error(buffer: &[u8]) -> String {
    let cstr = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
    let bytes = cstr.to_bytes();
    let (text, _, _had_errors) = GBK.decode(bytes);
    // 解析 JSON 字符串
    text.to_string()
}

/// 安全转换字符串到CString
fn to_cstr(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|e| format!("转换字符串失败: {}", e))
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub qsid: i32,
    pub address: Vec<Address>,
    pub version: String,
    pub trader_server_port: u16
}

#[derive(Debug, Deserialize)]
pub struct Address{
    pub host: String,
    pub port: i16,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub name: String,
    pub yyb_id: String,
    pub account: String,
    pub password: String,
    pub comm_password: String,
}
