use anyhow::Context;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug,Deserialize,JsonSchema)]
pub struct WebSearchArgs{
    pub query:String,

    #[schemars(range(min=0,max=20))]
    #[serde(default="default_max_results")]
    pub max_result:u8,
    #[serde(default="default_topic")]
    pub topic:String,

    #[serde(default)]
    pub time_range:Option<String>,
}
pub fn default_max_results()->u8{
    2
}
pub fn default_topic()->String{
    "general".to_string()
}

#[derive(Debug,Serialize)]
struct TavilyRequest<'a>{
    api_key:&'a str,
    query:&'a str,
    max_result:u8,
    topic:&'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_range:Option<&'a str>,

    search_depth:&'a str,
    include_answer:bool
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct SearchResults{
    pub title:String,
    pub url:String,
    pub content:String,
}

#[derive(Debug,Deserialize)]
pub struct TavilyResponse{
    pub results:Vec<SearchResults>,
    pub answer:Option<String>,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct WebSearchOutput{

    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer:Option<String>,
    pub result:Vec<SearchResults>,
}



pub async fn web_search(args:WebSearchArgs)->anyhow::Result<WebSearchOutput>{
    let api_key = std::env::var("TAVILY_API_KEY").context("Environment variable TAVILY_API_KEY not set".to_string())?;

    let body = TavilyRequest{
        api_key:&api_key,
        query:&args.query,
        max_result:args.max_result,
        topic:&args.topic,
        time_range:args.time_range.as_deref(),
        search_depth:"advanced",
        include_answer:true
    };

    let resp = reqwest::Client::new()
        .post("https://api.tavily.com/search")
        .json(&body)
        .send()
        .await
        .context("Failed to send Tavily request")?;

    if !resp.status().is_success(){
        anyhow::bail!(resp.status());
    }

    let parsed:TavilyResponse = resp.json().await.context("Failed to parse Tavily response")?;

    Ok(WebSearchOutput{
        answer:parsed.answer,
        result:parsed.results
    })

}

