use std::{env, time::Duration};

use base64::{Engine, prelude::BASE64_STANDARD};
use reqwest::{
    StatusCode,
    blocking::Client,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::versioning;
use versioning::ManagedExtensions;

pub enum FileOutputFormat {
    UTF8,
    BASE64,
}

#[derive(Debug, Deserialize)]
pub struct GetContentDirectoryEntryResponse {
    #[serde(rename = "type")]
    pub etype: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct GetBranchResponse {
    pub commit: Commit,
}

#[derive(Debug, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitCommit,
}

#[derive(Debug, Deserialize)]
pub struct CommitCommit {
    pub tree: Tree,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Tree {
    pub sha: String,
}

#[derive(Debug, Serialize)]
struct CreateBlobRequest {
    content: String,
    encoding: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlobResponse {
    pub sha: String,
}

#[derive(Debug, Serialize)]
struct CreateTreeRequest {
    pub base_tree: String,
    pub tree: Vec<RequestFile>,
}

#[derive(Debug, Serialize)]
struct RequestFile {
    pub path: String,
    pub mode: String,
    #[serde(rename = "type")]
    pub ftype: String,
    pub sha: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTreeResponse {
    pub sha: String,
}

#[derive(Debug, Serialize)]
struct CreateCommitRequest {
    message: String,
    tree: String,
    parents: Vec<String>,
    author: Author,
}

#[derive(Debug, Serialize)]
struct Author {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommitResponse {
    pub sha: String,
}

#[derive(Debug, Serialize)]
struct UpdateReferenceRequest {
    sha: String,
}

pub struct Requests {
    client: Client,
}

impl Requests {
    pub fn new() -> Result<Requests, ()> {
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
            HeaderValue::from_str(format!("Bearer {}", env::var("GITHUB_TOKEN").unwrap()).as_str())
                .unwrap(),
        );

        match Client::builder()
            .user_agent("paperback-community/registry-manager")
            .default_headers(headers)
            .timeout(Duration::new(15, 0))
            .build()
        {
            Ok(client) => Ok(Requests { client }),
            Err(err) => {
                error!(
                    "Something went wrong while creating the request client: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn get_file(
        &self,
        repository: &String,
        path: &String,
        branch: &String,
        output_format: &FileOutputFormat,
    ) -> Result<String, bool> {
        match self
            .client
            .get(format!(
                "https://api.github.com/repos/{}/contents/{}?ref={}",
                &repository, &path, &branch
            ))
            .header("Accept", "application/vnd.github.raw+json")
            .send()
        {
            Ok(raw_response) => {
                match raw_response.status() {
                    StatusCode::OK => (),
                    StatusCode::NOT_FOUND => {
                        error!("The requested file was not found");
                        return Err(false);
                    }
                    _ => {
                        error!(
                            "The response was undesired, status code: {}",
                            &raw_response.status(),
                        );
                        return Err(true);
                    }
                }

                match output_format {
                    FileOutputFormat::UTF8 => match raw_response.text() {
                        Ok(response) => Ok(response),
                        Err(err) => {
                            error!(
                                "Something went wrong while deserializing the raw response to UTF-8: {}",
                                &err
                            );
                            Err(true)
                        }
                    },
                    FileOutputFormat::BASE64 => match raw_response.bytes() {
                        Ok(response) => Ok(BASE64_STANDARD.encode(response)),
                        Err(err) => {
                            error!(
                                "Something went wrong while serialzing the raw response to base64: {}",
                                &err
                            );
                            Err(true)
                        }
                    },
                }
            }
            Err(err) => {
                error!("Something went wrong while making the request: {}", &err);
                Err(true)
            }
        }
    }

    pub fn get_directory(
        &self,
        repository: &String,
        path: &String,
        branch: &String,
    ) -> Result<Vec<GetContentDirectoryEntryResponse>, ()> {
        match self
            .client
            .get(format!(
                "https://api.github.com/repos/{}/contents/{}?ref={}",
                &repository, &path, &branch
            ))
            .send()
        {
            Ok(raw_response) => {
                if raw_response.status() != StatusCode::OK {
                    error!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(());
                }

                match raw_response.json::<Vec<GetContentDirectoryEntryResponse>>() {
                    Ok(response) => Ok(response),
                    Err(err) => {
                        error!(
                            "Something went wrong while deserializing the response to JSON: {}",
                            &err
                        );
                        Err(())
                    }
                }
            }
            Err(err) => {
                error!("Something went wrong while making the request: {}", &err);
                Err(())
            }
        }
    }

    pub fn get_branch(
        &self,
        repository: &String,
        branch: &String,
    ) -> Result<GetBranchResponse, ()> {
        match self
            .client
            .get(format!(
                "https://api.github.com/repos/{}/branches/{}",
                &repository, &branch
            ))
            .send()
        {
            Ok(raw_response) => {
                if raw_response.status() != 200 {
                    error!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(());
                }

                match raw_response.json::<GetBranchResponse>() {
                    Ok(response) => Ok(response),
                    Err(err) => {
                        error!(
                            "Something went wrong while deserializing the response to JSON: {}",
                            &err
                        );
                        Err(())
                    }
                }
            }
            Err(err) => {
                error!("Something went wrong while making the request: {}", &err);
                Err(())
            }
        }
    }

    pub fn create_blob(&self, content: String, encoding: String) -> Result<CreateBlobResponse, ()> {
        let body = CreateBlobRequest { content, encoding };

        let p_response = match serde_json::to_string(&body) {
            Ok(body_string) => self
                .client
                .post("https://api.github.com/repos/paperback-community/extensions/git/blobs")
                .body(body_string)
                .send(),
            Err(err) => {
                error!(
                    "Something went wrong while serializing the request body to JSON: {}",
                    &err
                );
                return Err(());
            }
        };

        match p_response {
            Ok(raw_response) => {
                if raw_response.status() != 201 {
                    error!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(());
                }

                match raw_response.json::<CreateBlobResponse>() {
                    Ok(response) => Ok(response),
                    Err(err) => {
                        error!(
                            "Something went wrong while deserializing the response to JSON: {}",
                            &err
                        );
                        Err(())
                    }
                }
            }
            Err(err) => {
                error!("Something went wrong while making the request: {}", &err);
                Err(())
            }
        }
    }

    pub fn create_tree(
        &self,
        base_tree: String,
        managed_extensions: ManagedExtensions,
    ) -> Result<CreateTreeResponse, ()> {
        let mut tree = vec![];
        for managed_extension in managed_extensions {
            for managed_extension_file in managed_extension.2.keys() {
                let file = RequestFile {
                    path: managed_extension_file.clone(),
                    mode: String::from("100644"),
                    ftype: String::from("blob"),
                    sha: managed_extension
                        .2
                        .get(&managed_extension_file.clone())
                        .cloned()
                        .unwrap(),
                };

                tree.push(file);
            }
        }

        let body = CreateTreeRequest { base_tree, tree };

        let p_response = match serde_json::to_string(&body) {
            Ok(body_string) => self
                .client
                .post("https://api.github.com/repos/paperback-community/extensions/git/trees")
                .body(body_string)
                .send(),
            Err(err) => {
                error!(
                    "Something went wrong while serializing the request body to JSON: {}",
                    &err
                );
                return Err(());
            }
        };

        match p_response {
            Ok(raw_response) => {
                if raw_response.status() != 201 {
                    error!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(());
                }

                match raw_response.json::<CreateTreeResponse>() {
                    Ok(response) => Ok(response),
                    Err(err) => {
                        error!(
                            "Something went wrong while deserializing the response to JSON: {}",
                            &err
                        );
                        Err(())
                    }
                }
            }
            Err(err) => {
                error!("Something went wrong while making the request: {}", &err);
                Err(())
            }
        }
    }

    pub fn create_commit(
        &self,
        message: String,
        tree_sha: String,
        parent_commit_sha: String,
        author_name: String,
        author_email: String,
    ) -> Result<CreateCommitResponse, ()> {
        let body = CreateCommitRequest {
            message,
            tree: tree_sha,
            parents: vec![parent_commit_sha],
            author: Author {
                name: author_name,
                email: author_email,
            },
        };

        let p_response = match serde_json::to_string(&body) {
            Ok(body_string) => self
                .client
                .post("https://api.github.com/repos/paperback-community/extensions/git/commits")
                .body(body_string)
                .send(),
            Err(err) => {
                error!(
                    "Something went wrong while serializing the request body to JSON: {}",
                    &err
                );
                return Err(());
            }
        };

        match p_response {
            Ok(raw_response) => {
                if raw_response.status() != 201 {
                    error!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(());
                }

                match raw_response.json::<CreateCommitResponse>() {
                    Ok(response) => Ok(response),
                    Err(err) => {
                        error!(
                            "Something went wrong while deserializing the response to JSON: {}",
                            &err
                        );
                        Err(())
                    }
                }
            }
            Err(err) => {
                error!("Something went wrong while making the request: {}", &err);
                Err(())
            }
        }
    }

    pub fn update_reference(&self, commit_sha: String) -> Result<(), ()> {
        let body = UpdateReferenceRequest { sha: commit_sha };

        let p_body_string = serde_json::to_string(&body);

        let p_response = match p_body_string {
            Ok(body_string) => self
                .client
                .post(
                    "https://api.github.com/repos/paperback-community/extensions/git/refs/heads/master",
                )
                .body(body_string)
                .send(),
            Err(err) => {
                error!(
                    "Something went wrong while serializing the request body to JSON: {}",
                    &err
                );
                return Err(());
            }
        };

        match p_response {
            Ok(raw_response) => {
                if raw_response.status() != 200 {
                    error!(
                        "The response was undesired, status code: {}",
                        &raw_response.status(),
                    );
                    return Err(());
                }
                Ok(())
            }
            Err(err) => {
                error!("Something went wrong while making the request: {}", &err);
                Err(())
            }
        }
    }
}
