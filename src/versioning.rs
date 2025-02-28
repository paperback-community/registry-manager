use std::collections::HashMap;

use base64::prelude::*;
use chrono::Utc;
use node_semver::Version;
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Versioning {
    build_time: String,
    built_with: BuiltWith,
    repository: Repository,
    sources: Vec<Source>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltWith {
    toolchain: String,
    types: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    name: String,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    id: String,
    name: String,
    description: String,
    version: String,
    icon: String,
    language: Option<String>,
    content_rating: String,
    badges: Vec<Option<Badges>>,
    capabilities: Option<Capabilities>,
    developers: Vec<Option<Developers>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Badges {
    label: String,
    text_color: String,
    background_color: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Capabilities {
    List(Vec<u8>),
    Primtitive(u8),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Developers {
    name: String,
    website: Option<String>,
    github: Option<String>,
}

type UpdatedExtensions = Vec<(String, HashMap<String, String>)>;

impl Versioning {
    pub fn new(response_content: &str) -> Result<Versioning, ()> {
        match BASE64_STANDARD.decode(response_content.replace("\n", "")) {
            Ok(bytes) => match serde_json::from_slice(bytes.as_slice()) {
                Ok(versioning) => Ok(versioning),
                Err(err) => {
                    error!(
                        "An error occurred while deserializing the response content to JSON: {}",
                        &err
                    );
                    Err(())
                }
            },
            Err(err) => {
                error!(
                    "An error occurred while base64 decoding the response content: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn to_base64(&self) -> Result<String, ()> {
        match serde_json::to_string(&self) {
            Ok(versioning_string) => Ok(BASE64_STANDARD.encode(&versioning_string)),
            Err(err) => {
                error!(
                    "An error occurred while encoding the versioning file into base64: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn update(&mut self, repository_versioning: &Versioning) -> Result<UpdatedExtensions, ()> {
        let mut updated_extensions = vec![];

        if self
            .built_with
            .types
            .parse::<Version>()
            .unwrap_or_else(|_| Version::parse("0.9.0").unwrap())
            > repository_versioning
                .built_with
                .types
                .parse::<Version>()
                .unwrap_or_else(|_| Version::parse("0.0.0").unwrap())
        {
            error!(
                "The repository was build with an invalid @paperback/types version {}, expected version {} or higher",
                repository_versioning.built_with.types, self.built_with.types
            );
            return Err(());
        }

        for repository_extension in repository_versioning.sources.iter() {
            let mut new = true;

            for (index, registry_extension) in self.sources.iter().enumerate() {
                if repository_extension.id != registry_extension.id {
                    continue;
                }

                new = false;

                if repository_extension
                    .version
                    .parse::<Version>()
                    .unwrap_or_else(|_| Version::parse("0.0.0").unwrap())
                    > registry_extension.version.parse::<Version>().unwrap()
                {
                    self.sources.insert(index, repository_extension.clone());

                    updated_extensions.push((repository_extension.id.clone(), HashMap::new()));
                }

                break;
            }

            if new {
                self.sources.push(repository_extension.clone());

                updated_extensions.push((repository_extension.id.clone(), HashMap::new()));
            }
        }

        if updated_extensions.is_empty() {
            error!("There are no extensions to update");
            return Err(());
        }

        self.build_time = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        self.built_with.toolchain = repository_versioning.built_with.toolchain.clone();
        self.built_with.types = repository_versioning.built_with.types.clone();

        if self.repository.name.is_empty() {
            /*
             * The version in the repository name is hard coded until the types package and app version are in sync again, alternative:
             * format!("Paperback Community Extensions({}.{})", self.built_with.types.parse::<Version>().unwrap().major, self.built_with.types.parse::<Version>().unwrap().minor);
             */
            self.repository.name = String::from("Paperback Community Extensions (0.9)");
            self.repository.description = String::from(
                "All extensions from the Paperback Community combined into a single repository.",
            );
        }

        Ok(updated_extensions)
    }
}
