#[allow(unused_imports)]
#[allow(dead_code)]

use chrono::{DateTime, Utc};
use csv;
use fast_float as ff;
use std::{fs, borrow::Borrow};
use std::any::type_name;
use reqwest as req;
use req::header;
use serde::{Serialize, Deserialize};

fn parse_csv(path: String) -> Result<Vec<Ticker>, csv::Error> {
    let mut file = csv::Reader::from_path(path + "/input.csv")?;
    let mut data: Vec<Ticker> = Vec::new();
    for result in file.records() {
        let record = result?;
        let ticker_type = match record.get(0) {
            Some("usd") | Some("eur") => TickerType::FIAT,
            Some("usdc") | Some("usdt") => TickerType::STABLE,
            Some(v) => TickerType::CRYPTO,
            None => panic!()
        };
        let tick = Ticker{name: record.get(0).unwrap().to_string(), amount: ff::parse(record.get(1).unwrap()).unwrap(), price: 0.0, ticker_type };
        data.push(tick);
    }
    Ok(data)
}

fn parseJson(path: String) -> Result<serde_json::Value, serde_json::Error>{
    let data = fs::read_to_string(path)
        .expect("Unable to read file");
    let json: serde_json::Value = serde_json::from_str(&data)
        .expect("JSON does not have correct format.");
    Ok(json) // json["property"]
}

fn getPath() -> String {
    std::env::current_dir().unwrap().to_string_lossy().to_string()
}

fn getSettings() -> Result<serde_json::Value, serde_json::Error>{
    let path = getPath();
    parseJson(path+"/settings.json")
}

fn writeFile(filename: &str, content: String) {
    let path = getPath()+"/"+filename;
    fs::write(path, content).expect("Unable to write file");
}

#[derive(Debug)]
pub enum TickerType {
    STABLE,
    FIAT,
    CRYPTO
}

#[derive(Debug)]
pub struct Ticker{
    pub name: String,
    pub ticker_type: TickerType,
    pub amount: f32,
    pub price: f32,
}

impl Ticker{
    fn createTicker(name: String, ticker_type: TickerType, amount: f32) -> Self{
        Self { name, ticker_type, amount, price: 0.0 }
    }

    fn createFiat(name: String, amount: f32) -> Self{
        Self { name, ticker_type: TickerType::FIAT, amount, price: 1.0 }
    }

    fn createCrypto(name: String, amount: f32) -> Self{
        Self { name, ticker_type: TickerType::CRYPTO , amount, price: 0.0 }
    }

    fn createStable(name: String, amount: f32) -> Self{
        Self { name, ticker_type: TickerType::STABLE , amount, price: 0.0 }
    }
}

struct Wallet {
    cryptocurrencies: Vec<Ticker>,
    stableCoins: Vec<Ticker>,
    baseCurrency: Ticker,
    total_value: f32,
    date: DateTime<Utc>
}

impl Wallet { 
    fn new(cryptocurrencies: Vec<Ticker>, stableCoins: Vec<Ticker>, baseCurrency: Ticker) -> Self{
        Self { cryptocurrencies, stableCoins, baseCurrency, total_value: 0.0, date: Utc::now() } 
    }

    fn calc_total_value(&mut self) {
        for crypto in &self.cryptocurrencies{
            self.total_value += crypto.amount * crypto.price; 
        }
        for stable in &self.stableCoins{
            self.total_value += stable.amount * stable.price;       
        }
    }
}

struct cmcApi{
    client: req::blocking::Client,
    baseUrl: String,
    settings: Settings
}

#[derive(Deserialize, Debug)]
struct Settings{
    currency: String,
    provider: String,
    fetchSymb: bool,
    CMC_key: String,
    path: String
}

impl Settings{
    fn new() -> Self{
        match getSettings(){
            Ok(v) => serde_json::from_value(v).unwrap(),
            Err(_) => panic!()
        }
    }
}

impl cmcApi{
    fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        let settings = Settings::new();
        headers.insert("Accepts", header::HeaderValue::from_static("application/json"));
        headers.insert("Accept-Encoding", header::HeaderValue::from_static("deflate, gzip"));
        let val = match header::HeaderValue::from_str(&settings.CMC_key){
            Ok(v) => v,
            Err(_) => panic!()
        };
        headers.insert("X-CMC_PRO_API_KEY", val);
        let client = req::blocking::Client::builder()
            .default_headers(headers)
            .build().unwrap();
        
        Self {client:client, baseUrl:"https://pro-api.coinmarketcap.com/v1/".to_string(), settings: settings }
    }

    fn fetchID(self){
        let path = "cryptocurrency/map";
        let res: String = self.client.get(self.baseUrl+path)
            .send().unwrap().text().unwrap();
        writeFile("cached_id_CMC.json", res)
    }

    // receive a vector of String(ID)
    fn getPriceOf(self, symbols: Vec<String>) -> Vec<f64>{
        let path = "cryptocurrency/quotes/latest";
        let currency = self.settings.currency.clone();
        let param = [
            ("id", &symbols.join(",")),
            ("convert", &currency)
        ];

        let url = req::Url::parse_with_params(&(self.baseUrl+path),  param).unwrap();
        //println!("{:?}", url);
        let res = self.client.get(url).send().unwrap().text().unwrap();
        let a:serde_json::Value = serde_json::from_str(&res).expect("JSON does not have correct format.");
        let mut toReturn:Vec<f64> = Vec::new();

        for symb in symbols{
            toReturn.push(a["data"][symb]["quote"][&currency]["price"].as_f64().unwrap());
        }
        println!("{:?}", toReturn); 
        toReturn
    }
    /*
        def getPriceOf(self, symbol: list):

        try:
            response = self.session.get(self.baseurl+path, params=parameters)
            data = json.loads(response.text)
            for symb, id in convertedSymbol.items():
                toReturn[symb] = data['data'][id]["quote"][self.currency]["price"] # store only price

        except (ConnectionError, Timeout, TooManyRedirects):
            data = json.loads(response.text)
        
        # if one or more symbols are not found for any kind of problem 
        # return also the missing one(s) and data
        if (set(convertedSymbol.keys()) & set(toReturn.keys())) != set(symbol):
            return (toReturn, False, set(symbol) - set(toReturn.keys()), data)
        
        return (toReturn, True)
    */

}

fn main(){
    let cmc = cmcApi::new();
    cmc.getPriceOf(vec!["1027".to_string(),"1".to_string()]);
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}