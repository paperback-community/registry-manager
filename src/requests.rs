use std::{env, time::Duration};

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Method, Request, Url,
};
use serde::{Deserialize, Serialize};
use tower::{limit::RateLimit, util::ServiceFn};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetContent {
    Struct(GetContentFile),
    List(Vec<GetContentDirectory>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetContentFile {
    #[serde(rename = "type")]
    pub _type: String,
    pub encoding: String,
    pub size: u64,
    pub name: String,
    pub path: String,
    pub content: String,
    pub sha: String,
    pub url: String,
    pub git_url: Option<String>,
    pub html_url: Option<String>,
    pub download_url: Option<String>,
    pub _links: Links,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetContentDirectory {
    #[serde(rename = "type")]
    pub _type: String,
    pub size: u64,
    pub name: String,
    pub path: String,
    pub content: Option<String>,
    pub sha: String,
    pub url: String,
    pub git_url: Option<String>,
    pub html_url: Option<String>,
    pub download_url: Option<String>,
    pub _links: Links,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Links {
    #[serde(rename = "self")]
    pub _self: String,
    pub git: Option<String>,
    pub html: Option<String>,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct PutFileRequestBody {}
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct PutFileResponse {
//     pub content: Option<Content>,
//     pub commit: Commit,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Content {
//     pub name: String,
//     pub path: String,
//     pub sha: String,
//     pub size: u64,
//     pub url: String,
//     pub html_url: String,
//     pub git_url: String,
//     pub download_url: String,
//     #[serde(rename = "type")]
//     pub _type: String,
//     pub _links: Links,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Commit {
//     pub sha: String,
//     pub node_id: String,
//     pub url: String,
//     pub html_url: String,
//     pub author: Author,
//     pub committer: Committer,
//     pub message: String,
//     pub tree: Tree,
//     pub parents: Vec<Parents>,
//     pub verification: Verification,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Author {
//     pub date: String,
//     pub name: String,
//     pub email: String,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Committer {
//     pub date: String,
//     pub name: String,
//     pub email: String,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Tree {
//     pub url: String,
//     pub sha: String,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Parents {
//     pub url: String,
//     pub html_url: String,
//     pub sha: String,
// }
//
// #[derive(Debug, Serialize, Deserialize)]
// pub struct Verification {
//     pub verified: bool,
//     pub reason: String,
//     pub signature: Option<String>,
//     pub payload: Option<String>,
//     pub verified_at: Option<String>,
// }

pub struct Requests<T> {
    service: RateLimit<ServiceFn<T>>,
}

pub fn new<T>() -> Result<Requests<T>, u8> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Accept",
        HeaderValue::from_str("application/vnd.github+json").unwrap(),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_str("2022-11-28").unwrap(),
    );
    headers.insert(
        "Authorization",
        HeaderValue::from_str(
            format!("Bearer {}", env::var("REGISTRY_MANAGER_PAT").unwrap()).as_str(),
        )
        .unwrap(),
    );

    match reqwest::Client::builder()
        .user_agent("paperback-community/registry-manager")
        .default_headers(headers)
        .timeout(Duration::new(10, 0))
        .build()
    {
        Ok(client) => {
            let service = tower::ServiceBuilder::new()
                .rate_limit(1, Duration::new(1, 0))
                .service(tower::service_fn(move |req| client.execute(req)));

            Ok(Requests { service })
        }
        Err(err) => {
            println!(
                "Something went wrong while creating the request client: {}",
                &err
            );
            Err(0x1)
        }
    }
}

impl<T> Requests<T> {
    #[tokio::main]
    pub async fn get_files(
        &self,
        repository: &String,
        path: &String,
        branch: &String,
    ) -> Result<GetContent, u8> {
        let request = Request::new(
            Method::GET,
            Url::parse(
                format!(
                    "https://api.github.com/repos/{}/contents/{}?ref={}",
                    &repository, &path, &branch
                )
                .as_str(),
            )
            .unwrap(),
        );

        let f_response = self.service.ready_and().await.call(request).await;

        match f_response {
            Ok(raw_response) => {
                if raw_response.status() != 200 {
                    eprintln!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(0x1);
                }

                match raw_response.json::<GetContent>().await {
                    Ok(response) => Ok(response),
                    Err(err) => {
                        eprintln!(
                            "Something went wrong when deserializing the response to JSON: {}",
                            &err
                        );
                        Err(0x1)
                    }
                }
            }
            Err(err) => {
                eprintln!("Something went wrong when making the request: {}", &err);
                Err(0x1)
            }
        }
    }
}
