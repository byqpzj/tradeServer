## 配置
参考示例配置文件

## 启动
无参数启动 ./tradeServer.exe ,启动后根据配置文件选择账户

参数指定账户 ./tradeServer.exe xx券商 

### wine
使用wine 10以上版本，需要 linux 修改默认的字符集为 zh_CN.UTF-8

wine tradeServer.exe xx券商-xxx 启动

## 接口
```
GET /query/zijin            查询资金
GET /query/chicang          查询当前持仓
GET /query/weituo           查询当日委托
GET /query/chengjiao        查询当日成交
GET /query/weituokeche      查询成交可撤
GET /query/gudong           查询股东账户

POST /order/buy             买入下单，参数  {"gddm":"股东账号", "gpdm": "股票代码", "price": 价格, "quantity": 数量}
POST /order/sell            卖出下单，参数  {"gddm":"股东账号", "gpdm": "股票代码", "price": 价格, "quantity": 数量}
GET  /order/cancel/<???>    撤单，最后参数是合同编号

POST /history/weituo        查询历史委托 参数 {"begin_date":"20190101","end_date":"20191231"}
POST /history/chengjiao     查询历史成交 参数 {"begin_date":"20190101","end_date":"20191231"}
```


## 编译
rustup target add i686-pc-windows-msvc

cargo build --bin tradeServer --target i686-pc-windows-msvc --profile release
