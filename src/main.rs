#[allow(dead_code)]

use chrono::{DateTime, Utc};
use csv;
use fast_float as ff;
use std::{fs, vec};
#[allow(unused_imports)]
use std::any::type_name;
use reqwest as req;
use req::header;
use serde::{Deserialize, Serialize};
use yahoo_finance_api as yahoo;
use std::path::Path;

fn read_file(path: String) -> String {
    fs::read_to_string(path).expect("Unable to read file")
}

fn write_file(path: &String, content: String) {
    fs::write(path, content).expect("Unable to write file");
}

fn create_file(path: &String){
    fs::File::create(path).expect("Unable to create file");
}

fn load_json(data: String) -> Option<serde_json::Value>{
    // TODO rewrite data handling
    match data.len() {
        0 => panic!("data is empty"),
        1.. => match serde_json::from_str(&data) {
            Ok(data) => data,
            Err(error) => panic!("Problem parsing json: {:?}", error),
        }
        _ => panic!("unexpected error")
    }

}

fn dump_json(data: &Vec<Ticker>) -> String {
    serde_json::to_string_pretty(&data).expect("JSON does not have correct format.")
}

fn get_current_path() -> String {
    std::env::current_dir().expect("Unable to get current path").to_string_lossy().to_string()+"/"
}

fn get_settings() -> serde_json::Value{
    let path = get_current_path();
    load_json(read_file(path+"settings.json")).unwrap()
}

fn get_index_of(data: &Vec<Ticker>, temp_ticker: &Ticker) -> usize{
    data.iter().position(|r| r == temp_ticker ).unwrap()
}

#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum TickerType {
    STABLE,
    FIAT,
    CRYPTO,
    NOT_SPECIFIED
}

#[derive(Debug, Serialize, Clone)]
struct Ticker{
    name: String,
    cmc_id: String, 
    ticker_type: TickerType,
    amount: f32,
    price: f32,
}

impl PartialEq for Ticker {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[allow(non_snake_case)]
#[allow(dead_code)]
impl Ticker{
    fn create_ticker(name: String, cmc_id: String, amount: f32) -> Self{
        Self { name, cmc_id, ticker_type: TickerType::NOT_SPECIFIED, amount, price: 0.0 }
    }

    fn create_ticker_WO_cmc_id(name: String, amount: f32, ticker_type: TickerType) -> Self{
        Self { name, cmc_id: String::new() , ticker_type, amount, price: 0.0 }
    }

    fn createFiat(name: String, cmc_id: String, amount: f32) -> Self{
        Self { name, cmc_id, ticker_type: TickerType::FIAT, amount, price: 1.0 }
    }

    fn createCrypto(name: String, cmc_id: String, amount: f32) -> Self{
        Self { name, cmc_id, ticker_type: TickerType::CRYPTO , amount, price: 0.0 }
    }

    fn createStable(name: String, cmc_id: String, amount: f32) -> Self{
        Self { name, cmc_id, ticker_type: TickerType::STABLE , amount, price: 0.0 }
    }

    fn createbase_currency(name: String) -> Self{
        Ticker::createFiat(name.clone(), name, 0.0)
    }
}

#[allow(dead_code)]
struct WalletType{
    crypto_stable: bool,
    crypto_stable_fiat: bool
}

#[allow(dead_code)]
impl WalletType{
    fn create_wallet_crypto() -> Self { Self{crypto_stable: true, crypto_stable_fiat: false} }
    fn create_wallet_total() -> Self { Self{crypto_stable: false, crypto_stable_fiat: true} }
}

#[allow(dead_code)]
struct Wallet {
    cryptocurrencies: Vec<Ticker>,
    stable_coins: Vec<Ticker>,
    fiat: Vec<Ticker>,
    base_currency: Ticker,
    total_value: f32,
    date: DateTime<Utc>,
    cmc: CmcApi,
    wallet_type: WalletType 
}

impl Wallet { 
    fn new(base_currency: Ticker, wallet_type: WalletType) -> Self{
        let mut cryptocurrencies = Vec::new();
        let mut stable_coins = Vec::new();
        let mut fiat = Vec::new();

        let cmc = CmcApi::new();
        let tickers = cmc.parse_csv(get_current_path()+"input.csv");

        for mut tick in tickers{
            // ticker must have a type defined
            match tick.ticker_type {
                TickerType::CRYPTO => cryptocurrencies.push(tick),
                TickerType::STABLE => stable_coins.push(tick),
                TickerType::FIAT => {tick.price = 1.0;fiat.push(tick)},
                TickerType::NOT_SPECIFIED => panic!()
            }
        }
        Self { cryptocurrencies, stable_coins, fiat, base_currency, total_value: 0.0, date: Utc::now(), cmc, wallet_type } 
    }

    fn get_price_of(&mut self, group: Vec<Ticker>) -> Vec<Ticker>{
        let (found, not_found) = self.cmc.convert_symbol_to_id(group);
        if not_found.len() > 0{
            let mut name_to_print: Vec<String> = Vec::new();
            for ticker in not_found{
                name_to_print.push(ticker.name);
            }
            println!("Not able to convert the following symbol(s): {:?}", name_to_print);
        } 
        self.cmc.get_price_of(found)
    }

    fn calc_total_value(&mut self) {
        if self.wallet_type.crypto_stable_fiat || self.wallet_type.crypto_stable{
            self.cryptocurrencies = self.get_price_of(self.cryptocurrencies.clone());
            self.stable_coins = self.get_price_of(self.stable_coins.clone());
            for crypto in &self.cryptocurrencies{
                self.total_value += crypto.amount * crypto.price; 
            }
            for stable in &self.stable_coins{
                self.total_value += stable.amount * stable.price;       
            }
        }
        
        if self.wallet_type.crypto_stable_fiat{
            for fiat in &self.fiat{
                if fiat.name == self.cmc.settings.currency.to_ascii_lowercase(){
                    self.total_value += fiat.amount * fiat.price;
                }else{
                    let price = self.get_forex_rate(self.base_currency.name.clone(), fiat.name.clone()) as f32;
                    self.total_value += fiat.amount * price;
                }
            }
        }
    }

    // yahoo finance
    fn get_forex_rate(&self, fiat1: String, fiat2: String) ->f64{
        let ticker = fiat1+&fiat2+"=X";
        println!("{:?}", ticker);
        let provider = yahoo::YahooConnector::new();
        let response = provider.get_quote_range(&ticker, "15m", "1d").unwrap();
        let quotes = response.quotes().unwrap();
        quotes.last().unwrap().close
    }
}

struct CmcApi{
    client: req::blocking::Client,
    base_url: String,
    settings: Settings,
    used_id: Vec<Ticker>
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Settings{
    currency: String,
    provider: String,
    fetch_symb: bool,
    cmc_key: String,
    path: String
}

impl Settings {
    fn new() -> Self { 
        serde_json::from_value(get_settings()).unwrap()
    }
}

impl CmcApi{
    fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        let settings = Settings::new();
        headers.insert("Accepts", header::HeaderValue::from_static("application/json"));
        headers.insert("Accept-Encoding", header::HeaderValue::from_static("deflate, gzip"));
        let val = match header::HeaderValue::from_str(&settings.cmc_key){
            Ok(v) => v,
            Err(_) => panic!()
        };
        headers.insert("X-CMC_PRO_API_KEY", val);
        let client = req::blocking::Client::builder()
            .default_headers(headers)
            .build().unwrap();
        
        let pathused_id = get_current_path()+"used_id_CMC.json";
        if ! Path::new(pathused_id.as_str()).exists(){
            create_file(&pathused_id);
        }

        let mut used_id: Vec<Ticker> = Vec::new();
        match load_json(read_file(pathused_id)) {
            Some(jsonused_id) => {
                for v in jsonused_id.as_array().unwrap(){
                    let arr_v = v.as_object().unwrap();
                    used_id.push(Ticker::createCrypto(arr_v["name"].as_str().unwrap().to_owned(), arr_v["cmc_id"].as_str().unwrap().to_owned(), 0.0))
                }
            },
            None => {},
        };
        let obj = Self {client:client, base_url:"https://pro-api.coinmarketcap.com/v1/".to_string(), settings, used_id };
        if obj.settings.fetch_symb{
            obj.fetch_id();
        }
        obj
    }

    fn parse_csv(&self, path: String) ->Vec<Ticker> {
        let mut file = csv::Reader::from_path(path).unwrap();
        let mut data: Vec<Ticker> = Vec::new();
        // check every row of the csv
        for result in file.records() {
            let record = result.unwrap();
            if record.get(0).unwrap().to_string() == "total_invested"{
                // for now skip total invested row
                continue;
            } // retrieve TickerType
            let ticker_type = match record.get(0) {
                Some("usd") | Some("eur") => TickerType::FIAT,
                Some("usdc") | Some("usdt") => TickerType::STABLE,
                Some(_) => TickerType::CRYPTO,
                None => {println!("Unexpected error...");panic!()}
            };
            let tick = Ticker::create_ticker_WO_cmc_id(record.get(0).unwrap().to_string(), ff::parse(record.get(1).unwrap()).unwrap(), ticker_type);
            // check if tick is already in data
            // in case yes -> sum it's amount to one's amount in data
            if data.contains(&tick){
                let index = get_index_of(&data, &tick);
                data[index].amount += tick.amount; 
            }else {data.push(tick);}
        }
        data
    }

    fn fetch_id(&self){
        let path = "cryptocurrency/map";
        let res: String = self.client.get(self.base_url.to_owned()+path)
            .send().unwrap().text().unwrap();
        write_file(&(get_current_path()+"cached_id_CMC.json"), res)
    }

    // receive a vector of Vec<Ticker> and return it filled with price
    // retrieved from CoinMarketCap
    fn get_price_of(&self, symbols: Vec<Ticker>) -> Vec<Ticker>{
        let path = "cryptocurrency/quotes/latest";
        let currency = self.settings.currency.clone();
        let mut id: Vec<String> = Vec::new();
        for i in symbols.clone(){
            id.push(i.cmc_id);
        }
        let param = [
            ("id", &id.join(",").to_owned()),
            ("convert", &currency)
        ];

        let url = req::Url::parse_with_params(&(self.base_url.to_owned()+path),  param).unwrap();
        let res = self.client.get(url).send().unwrap().text().unwrap();
        let a:serde_json::Value = serde_json::from_str(&res).expect("JSON does not have correct format.");
        let mut to_return:Vec<Ticker> = Vec::new();

        for mut symb in symbols{
            symb.price = a["data"][&symb.cmc_id]["quote"][&currency]["price"].as_f64().unwrap() as f32;
            to_return.push(symb);
        }
        to_return
    }

    fn dumpused_id(&self){
        write_file(&(get_current_path()+"used_id_CMC.json") , dump_json(&self.used_id));
    }

    fn convert_symbol_to_id(&mut self, mut symbols: Vec<Ticker>) -> (Vec<Ticker>, Vec<Ticker>){
        /*
            return a vec of Ticker with cmc_id field and eventually a vec of Ticker not found
        */
        let symbo_lenght = symbols.len();
        let start_symbols = symbols.clone();

        let mut converted_id: Vec<Ticker> = Vec::new();
        for (_, ticker) in self.used_id.iter().enumerate(){
            if symbols.contains(ticker){
                let mut new_ticker = ticker.clone();
                new_ticker.amount = symbols[get_index_of(&symbols, ticker)].amount;
                converted_id.push(new_ticker);
                symbols.remove(get_index_of(&symbols, ticker)); // remove from searching list
            }
        }

        if symbols.len() > 0{
            let mut found:u8 = 0;
            let cached_id = load_json(read_file(get_current_path()+"cached_id_CMC.json")).unwrap(); // once in a while run fetch_id() to update it 

            for v in cached_id["data"].as_array().unwrap().iter(){
                let temp_ticker = Ticker::create_ticker_WO_cmc_id(v["symbol"].as_str().unwrap().to_ascii_lowercase(), 0.0, TickerType::NOT_SPECIFIED);
                if symbols.contains(&temp_ticker){
                    converted_id.push(Ticker::createCrypto(v["symbol"].as_str().unwrap().to_ascii_lowercase(), v["id"].as_u64().unwrap().to_string(), symbols[get_index_of(&symbols, &temp_ticker)].amount));
                    found +=1;
                    symbols.remove(get_index_of(&symbols, &temp_ticker)); // remove from searching list
                }
            }
            
            if found > 0{
                self.used_id.extend(converted_id.clone());
                self.dumpused_id();
            }
        }

        let mut difference: Vec<Ticker> = vec![];
        if symbo_lenght != converted_id.len(){
            for i in &converted_id {
                if !start_symbols.contains(&i) {
                    difference.push(i.clone());
                }
            }
        }
        (converted_id, difference)
    }

}

fn main(){
    let mut wallet = Wallet::new(Ticker::createbase_currency("eur".to_string()), WalletType::create_wallet_crypto() );
    wallet.calc_total_value();
    println!("{:?}", wallet.total_value)
    // TODO add json dumper to D:\crypto\walletValue.json D:\crypto\walletGeneralOverview
}

#[allow(dead_code)]
fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}
