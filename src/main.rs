use std::{fs, io, env};
use actix_rt::time::interval;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::process::exit;
use std::sync::Arc;
mod ths_client;
use ths_client::{ThsClient,process_buffer,process_error};
use crate::ths_client::{Account, Server};
use std::collections::HashMap;
use std::io::Write;
use std::thread::sleep;

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
}

// ----------------------
// 请求参数结构体
// ----------------------

#[derive(Debug, Deserialize)]
struct SendOrderParams {
    gddm: String,
    gpdm: String,
    price: f32,
    quantity: i32,
}

#[derive(Debug, Deserialize)]
struct HistoryDataParams {
    begin_date: String,
    end_date: String,
}

// ----------------------
// 后台任务
// ----------------------
async fn background_task(client: Arc<ThsClient>) {
    let mut interval = interval(std::time::Duration::from_secs(20));

    loop {
        interval.tick().await;
        match client.query_data(1) {
            // 查询分类1的数据
            Ok(_data) => {}
            Err(_e) => {}
        }
    }
}

// ----------------------
// 控制器实现
// ----------------------
#[get("/query/{category}")]
async fn query_data(
    client: web::Data<Arc<ThsClient>>,
    category: web::Path<String>,
) -> impl Responder {
    // 映射字符串到数字
    let category_num = match category.to_lowercase().as_str() {
        "zijin" => 0,
        "chicang" => 1,
        "weituo" => 2,
        "chengjiao" => 3,
        "weituokeche" => 4,
        "gudong" => 5,
        _ => {
            // 无效参数返回 400
            return HttpResponse::BadRequest().json(ApiResponse {
                success: false,
                data: Some("参数错误,应为 zijin, chicang, weituo, chengjiao, weituokeche, gudong"),
            });
        }
    };
    match client.query_data(category_num) {
        Ok(data) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(process_buffer(&*data)),
        }),
        Err(e) => HttpResponse::BadRequest().json(ApiResponse {
            success: false,
            data: Some(process_error(&*e)),
        }),
    }
}

#[post("/order/{category}")]
async fn send_order(
    client: web::Data<Arc<ThsClient>>,
    category: web::Path<String>,
    params: web::Json<SendOrderParams>,
) -> impl Responder {
    let category_num = match category.to_lowercase().as_str() {
        "buy" => 0,
        "sell" => 1,
        _ => {
            // 无效参数返回 400
            return HttpResponse::BadRequest().json(ApiResponse {
                success: false,
                data: Some("url路径参数错误,应为 buy, sell"),
            });
        }
    };
    match client.send_order(
        category_num,
        &params.gddm,
        &params.gpdm,
        params.price,
        params.quantity,
    ) {
        Ok(data) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(process_buffer(&*data)),
        }),
        Err(e) => HttpResponse::BadRequest().json(ApiResponse {
            success: false,
            data: Some(process_error(&*e)),
        }),
    }
}

#[get("/order/cancel/{order_id}")]
async fn cancel_order(
    client: web::Data<Arc<ThsClient>>,
    order_id: web::Path<String>,
) -> impl Responder {
    match client.cancel_order(&order_id) {
        Ok(data) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(process_buffer(&*data)),
        }),
        Err(e) => HttpResponse::BadRequest().json(ApiResponse {
            success: false,
            data: Some(process_error(&*e)),
        }),
    }
}

#[post("/history/{category}")]
async fn query_history_data(
    client: web::Data<Arc<ThsClient>>,
    category: web::Path<String>,
    params: web::Query<HistoryDataParams>,
) -> impl Responder {
    let category_num = match category.to_lowercase().as_str() {
        "weituo" => 0,
        "chengjiao" => 1,
        _ => {
            // 无效参数返回 400
            return HttpResponse::BadRequest().json(ApiResponse {
                success: false,
                data: Some("参数错误,应为 weituo, chengjiao"),
            });
        }
    };
    match client.query_history_data(category_num, &params.begin_date, &params.end_date) {
        Ok(data) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(process_buffer(&*data)),
        }),
        Err(e) => HttpResponse::BadRequest().json(ApiResponse {
            success: false,
            data: Some(process_error(&*e)),
        }),
    }
}

const MAX_LOGIN_ATTEMPTS: u8 = 5;
const RETRY_DELAY_SECS: u64 = 5;

/// 多次重试保证登录正常
fn try_login_and_check(client: &mut ThsClient, server: &Server, account: &Account, attempt: u8) -> Result<(),()> {
    println!("尝试第 {} 次登录...", attempt);
    let address = &*server.address.get(0).unwrap();
    match client.logon(server, account, address) {
        Ok(_) => {
            println!("第{}次登录成功", attempt);
            sleep(std::time::Duration::from_secs(2));
            match client.query_data(1) {
                Ok(_) => {
                    sleep(std::time::Duration::from_secs(2));
                    match client.query_data(2) {
                        Ok(_) => {
                            println!("第{}次服务器响应正常", attempt);
                            Ok(())
                        }
                        Err(msg) => {
                            println!("第{}次服务器响应异常 {}", attempt, process_error(&*msg));
                            Err(())
                        }
                    }
                }
                Err(msg) => {
                    println!("第{}次服务器响应异常 {}", attempt, process_error(&*msg));
                    Err(())
                }
            }
        }
        Err(msg) => {
            println!("第{}次登录失败 {}", attempt, process_error(&*msg));
            Err(())
        }
    }
}

fn login_with_retry(client: &mut ThsClient, server: &Server, account: &Account) -> Result<(),()> {
    for attempt in 1..=MAX_LOGIN_ATTEMPTS {
        match try_login_and_check(client, server, account, attempt) {
            Ok(_) => return Ok(()),
            Err(_) => {
                if attempt == MAX_LOGIN_ATTEMPTS {
                    return Err(());
                }
                println!("等待 {} 秒后进行第 {} 次重试...", RETRY_DELAY_SECS, attempt + 1);
                sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
            }
        }
    }
    Err(())
}

// ----------------------
// 服务配置
// ----------------------
#[actix_web::main]
async fn main() -> io::Result<()> {

    let content = fs::read_to_string("account.json").map_err(|_e| "无法找到 account.json 配置文件").unwrap();
    let accounts: Vec<Account> = serde_json::from_str(&content).map_err(|_e| "解析 account.json 失败").unwrap();

    // 获取命令行参数
    let args: Vec<String> = env::args().collect();
    
    let selected_key = if args.len() > 1 {
        // 通过账户名称查找账户
        match accounts.iter().position(|a| a.name == args[1]) {
            Some(index) => index,
            None => {
                println!("未找到账户名称: {}", args[1]);
                println!("可用的账户名称:");
                for a in accounts.iter() {
                    println!("- {}", a.name);
                }
                exit(0);
            }
        }
    } else {
        // 如果没有参数，使用交互式选择
        println!("请选择账户（输入序号）：");
        for (i, a) in accounts.iter().enumerate() {
            println!("{}. {}", i + 1, a.name);
        }
        
        // 循环读取用户输入直到有效
        let selected: usize = loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            if let Err(e) = io::stdin().read_line(&mut input) {
                eprintln!("读取输入失败: {}", e);
                continue;
            }

            let input = input.trim();
            if input.is_empty() {
                println!("输入不能为空，请重新输入。");
                continue;
            }
            
            match input.parse() {
                Ok(n) if (1..=accounts.len()).contains(&n) => break n,
                _ => {
                    println!("请输入 1 到 {} 之间的数字。", accounts.len());
                    continue;
                }
            }
        };
        selected - 1
    };

    let account = accounts.get(selected_key).unwrap();
    
    let content2 = fs::read_to_string("server.json").map_err(|_e| "无法找到 server.json 配置文件").unwrap();
    let servers: HashMap<String,Server> = serde_json::from_str(&content2).map_err(|_e| "解析 server.json 失败").unwrap();
    if account.qs_name.is_empty() {
        panic!("券商名称不能为空");
    }
    let server = servers.get(&account.qs_name).unwrap();
    
    if server.version.is_empty() {
        panic!("版本不能为空");
    }

    let trade_port = server.trader_server_port;
    
    // 初始化客户端 dll 使用实际路径
    let dll_path = r"tradej.dll";
    let mut client1 = ThsClient::new(dll_path).expect("无法初始化客户端");

    // 使用新的重试逻辑
    if login_with_retry(&mut client1, server, account).is_err() {
        println!("登录失败次数过多，程序退出");
        exit(0);
    }

    let client = Arc::new(client1);

    // todo 克隆客户端用于后台任务
    // let bg_client = Arc::clone(&client);
    // actix_rt::spawn(async move { background_task(bg_client).await });
    // hide_console_window();
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
            .service(query_data)
            .service(send_order)
            .service(cancel_order)
            .service(query_history_data)
    })
    .bind(("0.0.0.0", trade_port))?
    .run()
    .await
}

