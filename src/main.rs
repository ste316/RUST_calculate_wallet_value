use chrono::{DateTime, Utc};
use csv;
use fast_float as ff;
use std::fs;

#[derive(Debug)]
enum TickerType {
    STABLE,
    FIAT,
    CRYPTO
}

#[derive(Debug)]
struct Ticker{
    name: String,
    ticker_type: TickerType,
    amount: f32,
    price: f32,
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

// TODO fn performRequest

fn parseCSV(path: String) -> Result<Vec<Ticker>, csv::Error> {
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
    println!("{:?}", data);
    Ok(data)
}

fn parseJson(path: String) -> Result<serde_json::Value, serde_json::Error>{
    let data = fs::read_to_string(path)
        .expect("Unable to read file");
    let json: serde_json::Value = serde_json::from_str(&data)
        .expect("JSON does not have correct format.");
    Ok(json) // json["property"]
}

fn getSettings() -> Result<serde_json::Value, serde_json::Error>{
    let path = std::env::current_dir().unwrap().to_string_lossy().to_string();
    parseJson(path+"/settings.json")
}

fn main(){
    

}

