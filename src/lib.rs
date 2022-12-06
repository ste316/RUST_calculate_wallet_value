#[allow(unused_imports)]

use reqwest as req;
use tokio;
use req::header;

#[allow(dead_code)]
struct cmcApi{
    key: String,
    client: req::Client
}

impl cmcApi{
    fn new(key: String) -> Self { Self { key, client: req::Client::new() } }

    fn set_headers(&mut self) -> Result<(), req::Error>{
        let mut headers = header::HeaderMap::new();
        headers.insert("Accepts", header::HeaderValue::from_static("application/json"));
        headers.insert("Accept-Encoding", header::HeaderValue::from_static("deflate, gzip"));
        headers.insert("X-CMC_PRO_API_KEY", header::HeaderValue::from_static(""));
        self.client.default_headers(headers).build()?;
        Ok(())
    }

    fn req(self){
        let response = self.client
            .get("https://api.spotify.com/v1/search")
            .header(AUTHORIZATION, "Bearer [AUTH_TOKEN]")
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .send();
    }

    
}
// let res = client.get("https://www.rust-lang.org").send().await?;
// https://blog.logrocket.com/making-http-requests-rust-reqwest/

