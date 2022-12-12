#[allow(unused_imports)]
#[allow(dead_code)]

use chrono::{DateTime, Utc};
use csv;
use fast_float as ff;
use std::{fs};
use std::any::type_name;
use reqwest as req;
use req::header;
use serde::{Deserialize, Serialize};
use yahoo_finance_api as yahoo;

fn readFile(path: String) -> String {
    fs::read_to_string(path).expect("Unable to read file")
}

fn writeFile(path: String, content: String) {
    fs::write(path, content).expect("Unable to write file");
}

fn loadJson(data: String) -> serde_json::Value{
    serde_json::from_str(&data).expect("JSON does not have correct format.")
}

fn dumpJson(data: &Vec<Ticker>) -> String {
    serde_json::to_string_pretty(&data).expect("JSON does not have correct format.")
}

fn getCurrentPath() -> String {
    std::env::current_dir().expect("Unable to get current path").to_string_lossy().to_string()+"/"
}

fn getSettings() -> serde_json::Value{
    let path = getCurrentPath();
    loadJson(readFile(path+"settings.json"))
}

fn getIndexOf(data: &Vec<Ticker>, tempTicker: &Ticker) -> usize{
    data.iter().position(|r| r == tempTicker ).unwrap()
}

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
    cmcID: String, 
    ticker_type: TickerType,
    amount: f32,
    price: f32,
}

impl PartialEq for Ticker {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Ticker{
    fn createTicker(name: String, cmcID: String, amount: f32) -> Self{
        Self { name, cmcID, ticker_type: TickerType::NOT_SPECIFIED, amount, price: 0.0 }
    }

    fn createTickerWOcmcID(name: String, amount: f32, ticker_type: TickerType) -> Self{
        Self { name, cmcID: String::new() , ticker_type, amount, price: 0.0 }
    }

    fn createFiat(name: String, cmcID: String, amount: f32) -> Self{
        Self { name, cmcID, ticker_type: TickerType::FIAT, amount, price: 1.0 }
    }

    fn createCrypto(name: String, cmcID: String, amount: f32) -> Self{
        Self { name, cmcID, ticker_type: TickerType::CRYPTO , amount, price: 0.0 }
    }

    fn createStable(name: String, cmcID: String, amount: f32) -> Self{
        Self { name, cmcID, ticker_type: TickerType::STABLE , amount, price: 0.0 }
    }

    fn createBaseCurrency(name: String) -> Self{
        Ticker::createFiat(name.clone(), name, 0.0)
    }
}

struct WalletType{
    cryptoAndStable: bool,
    cryptoStableAndFiat: bool
}

impl WalletType{
    fn createWalletCrypto() -> Self { Self{cryptoAndStable: true, cryptoStableAndFiat: false} }
    fn createWalletTotal() -> Self { Self{cryptoAndStable: false, cryptoStableAndFiat: true} }
}

struct Wallet {
    cryptocurrencies: Vec<Ticker>,
    stableCoins: Vec<Ticker>,
    fiat: Vec<Ticker>,
    baseCurrency: Ticker,
    total_value: f32,
    date: DateTime<Utc>,
    cmc: cmcApi,
    walletType: WalletType 
}

impl Wallet { 
    fn new(baseCurrency: Ticker, walletType: WalletType) -> Self{
        let mut cryptocurrencies = Vec::new();
        let mut stableCoins = Vec::new();
        let mut fiat = Vec::new();

        let cmc = cmcApi::new();
        let tickers = cmc.parse_csv(getCurrentPath()+"input.csv");

        for mut tick in tickers{
            // ticker must have a type defined
            match tick.ticker_type {
                TickerType::CRYPTO => cryptocurrencies.push(tick),
                TickerType::STABLE => stableCoins.push(tick),
                TickerType::FIAT => {tick.price = 1.0;fiat.push(tick)},
                TickerType::NOT_SPECIFIED => panic!()
            }
        }
        Self { cryptocurrencies, stableCoins, fiat, baseCurrency, total_value: 0.0, date: Utc::now(), cmc, walletType } 
    }

    fn getPriceOf(&mut self, mut group: Vec<Ticker>) -> Vec<Ticker>{
        let (found, notFound) = self.cmc.convertSymbol2ID(group);
        if notFound.len() > 0{
            let mut nameToPrint: Vec<String> = Vec::new();
            for ticker in notFound{
                nameToPrint.push(ticker.name);
            }
            println!("Not able to convert the following symbol(s): {:?}", nameToPrint);
        } 
        self.cmc.getPriceOf(found)
    }

    fn calc_total_value(&mut self) {
        self.cryptocurrencies = self.getPriceOf(self.cryptocurrencies.clone());
        self.stableCoins = self.getPriceOf(self.stableCoins.clone());
        for crypto in &self.cryptocurrencies{
            self.total_value += crypto.amount * crypto.price; 
        }
        for stable in &self.stableCoins{
            self.total_value += stable.amount * stable.price;       
        }
        
        // TODO add option to include/exclude fiat
        for fiat in &self.fiat{
            if fiat.name == self.cmc.settings.currency.to_ascii_lowercase(){
                self.total_value += fiat.amount * fiat.price;
            }else{
                let price = self.getForexRate(self.baseCurrency.name.clone(), fiat.name.clone()) as f32;
                self.total_value += fiat.amount * price;
            }
            
        }
    }

    // yahoo finance
    fn getForexRate(&self, fiat1: String, fiat2: String) ->f64{
        let ticker = fiat1+&fiat2+"=X";
        println!("{:?}", ticker);
        let provider = yahoo::YahooConnector::new();
        let response = provider.get_quote_range(&ticker, "15m", "1d").unwrap();
        let quotes = response.quotes().unwrap();
        quotes.last().unwrap().close
    }
}

struct cmcApi{
    client: req::blocking::Client,
    baseUrl: String,
    settings: Settings,
    usedID: Vec<Ticker> // ("ticker", "id")
}

#[derive(Deserialize, Debug)]
struct Settings{
    currency: String,
    provider: String,
    fetchSymb: bool,
    CMC_key: String,
    path: String
}

impl Settings {
    fn new() -> Self { 
        serde_json::from_value(getSettings()).unwrap()
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
        
        let jsonUsedID = loadJson(readFile(getCurrentPath()+"used_id_CMC.json"));
        let mut usedID: Vec<Ticker> = Vec::new();
        for v in jsonUsedID.as_array().unwrap(){
            let arrV = v.as_object().unwrap();
            usedID.push(Ticker::createCrypto(arrV["name"].as_str().unwrap().to_owned(), arrV["cmcID"].as_str().unwrap().to_owned(), 0.0))
        }
        let obj = Self {client:client, baseUrl:"https://pro-api.coinmarketcap.com/v1/".to_string(), settings: settings, usedID };
        if obj.settings.fetchSymb{
            obj.fetchID();
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
                Some(v) => TickerType::CRYPTO,
                None => {println!("Unexpected error...");panic!()}
            };
            let tick = Ticker::createTickerWOcmcID(record.get(0).unwrap().to_string(), ff::parse(record.get(1).unwrap()).unwrap(), ticker_type);
            // check if tick is already in data
            // in case yes -> sum it's amount to one's amount in data
            if data.contains(&tick){
                let index = getIndexOf(&data, &tick);
                data[index].amount += tick.amount; 
            }else {data.push(tick);}
        }
        data
    }

    fn fetchID(&self){
        let path = "cryptocurrency/map";
        let res: String = self.client.get(self.baseUrl.to_owned()+path)
            .send().unwrap().text().unwrap();
        writeFile(getCurrentPath()+"cached_id_CMC.json", res)
    }

    // receive a vector of Vec<Ticker> and return it filled with price
    // retrieved from CoinMarketCap
    fn getPriceOf(&self, symbols: Vec<Ticker>) -> Vec<Ticker>{
        let path = "cryptocurrency/quotes/latest";
        let currency = self.settings.currency.clone();
        let mut id: Vec<String> = Vec::new();
        for i in symbols.clone(){
            id.push(i.cmcID);
        }
        let param = [
            ("id", &id.join(",").to_owned()),
            ("convert", &currency)
        ];

        let url = req::Url::parse_with_params(&(self.baseUrl.to_owned()+path),  param).unwrap();
        let res = self.client.get(url).send().unwrap().text().unwrap();
        let a:serde_json::Value = serde_json::from_str(&res).expect("JSON does not have correct format.");
        let mut toReturn:Vec<Ticker> = Vec::new();

        for mut symb in symbols{
            symb.price = a["data"][&symb.cmcID]["quote"][&currency]["price"].as_f64().unwrap() as f32;
            toReturn.push(symb);
        }
        toReturn
    }

    fn dumpUsedID(&self){
        writeFile(getCurrentPath()+"used_id_CMC.json" , dumpJson(&self.usedID));
    }

    fn convertSymbol2ID(&mut self, mut symbols: Vec<Ticker>) -> (Vec<Ticker>, Vec<Ticker>){
        /*
            return a vec of Ticker with cmcID field and eventually a vec of Ticker not found
        */
        let symboLenght = symbols.len();
        let startSymbols = symbols.clone();

        let mut convertedId: Vec<Ticker> = Vec::new();
        for (i, ticker) in self.usedID.iter().enumerate(){
            if symbols.contains(ticker){
                let mut newTicker = ticker.clone();
                newTicker.amount = symbols[getIndexOf(&symbols, ticker)].amount;
                convertedId.push(newTicker);
                symbols.remove(getIndexOf(&symbols, ticker)); // remove from searching list
            }
        }

        if symbols.len() > 0{
            let mut found:u8 = 0;
            let cachedID = loadJson(readFile(getCurrentPath()+"cached_id_CMC.json")); // once in a while run fetchID() to update it 

            for v in cachedID["data"].as_array().unwrap().iter(){
                let tempTicker = Ticker::createTickerWOcmcID(v["symbol"].as_str().unwrap().to_ascii_lowercase(), 0.0, TickerType::NOT_SPECIFIED);
                if symbols.contains(&tempTicker){
                    convertedId.push(Ticker::createCrypto(v["symbol"].as_str().unwrap().to_ascii_lowercase(), v["id"].as_u64().unwrap().to_string(), symbols[getIndexOf(&symbols, &tempTicker)].amount));
                    found +=1;
                    symbols.remove(getIndexOf(&symbols, &tempTicker)); // remove from searching list
                }
            }
            
            if found > 0{
                self.usedID.extend(convertedId.clone());
                self.dumpUsedID();
            }
        }

        let mut difference: Vec<Ticker> = vec![];
        if symboLenght != convertedId.len(){
            for i in &convertedId {
                if !startSymbols.contains(&i) {
                    difference.push(i.clone());
                }
            }
        }
        (convertedId, difference)
    }

}

fn main(){
    let mut wallet = Wallet::new(Ticker::createBaseCurrency("eur".to_string()), WalletType::createWalletCrypto() );
    wallet.calc_total_value();
    println!("{:?}", wallet.total_value)
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}