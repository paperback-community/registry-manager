use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};

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

pub struct Requests {
    client: Client,
}

pub fn new() -> Requests {
    Requests {
        client: reqwest::blocking::Client::new(),
    }
}

impl Requests {
    pub fn get_files(
        &self,
        repository: &String,
        path: &String,
        branch: &String,
    ) -> Result<GetContent, u8> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept",
            HeaderValue::from_str("application/vnd.github+json").unwrap(),
        );
        headers.insert(
            "User-Agent",
            HeaderValue::from_str("paperback-community/registry-manager").unwrap(),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_str("2022-11-28").unwrap(),
        );

        let p_response = self
            .client
            .get(format!(
                "https://api.github.com/repos/{}/contents/{}?ref={}",
                &repository, &path, &branch
            ))
            .headers(headers)
            .send();

        match p_response {
            Ok(raw_response) => {
                if raw_response.status() != 200 {
                    eprintln!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(0x1);
                }

                match raw_response.json::<GetContent>() {
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

    //// This method will not be used, instead blobs will be created
    // pub fn put_file(&self, file: &String, pat: &String) -> Result<(), u8> {
    //     let mut headers = HeaderMap::new();
    //     headers.insert(
    //         "Accept",
    //         HeaderValue::from_str("application/vnd.github+json").unwrap(),
    //     );
    //     headers.insert(
    //         "User-Agent",
    //         HeaderValue::from_str("paperback-community/registry-manager").unwrap(),
    //     );
    //     headers.insert(
    //         "X-GitHub-Api-Version",
    //         HeaderValue::from_str("2022-11-28").unwrap(),
    //     );
    //     headers.insert(
    //         "Authorization",
    //         HeaderValue::from_str(format!("Bearer {}", pat).as_str()).unwrap(),
    //     );

    //     let p_response = self.client.put(format!("https://api.github.com/repos/paperback-community/extensions-test/contents/versioning.json"))
    //         .headers(headers)
    //         // TODO: Make struct and serialize it to json
    //         .body("")
    //         .send();

    //     match p_response {
    //         Ok(raw_response) => {
    //             if raw_response.status() != 200 {
    //                 eprintln!(
    //                    "The response was undesired, status code: {}",
    //                     &raw_response.status(),
    //                 );
    //                 return Err(0x1);
    //             }

    //             Ok(())
    //         }
    //         Err(err) => {
    //             eprintln!("Something went wrong when making the request: {}", &err);
    //             Err(0x1)
    //         }
    //     }
    // }
}
