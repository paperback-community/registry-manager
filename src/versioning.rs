use std::collections::HashMap;

use base64::prelude::*;
use chrono::Utc;
use node_semver::Version;
use serde::{Deserialize, Serialize};

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

impl Versioning {
    pub fn new(response_content: &String) -> Result<Versioning, ()> {
        match BASE64_STANDARD.decode(response_content.replace("\n", "")) {
            Ok(bytes) => match serde_json::from_slice(bytes.as_slice()) {
                Ok(versioning) => {
                    println!("Parsed the requested versioning file");
                    Ok(versioning)
                }
                Err(err) => {
                    eprintln!(
                        "An error occurred while deserializing the response content to JSON: {}",
                        &err
                    );
                    Err(())
                }
            },
            Err(err) => {
                eprintln!(
                    "An error occurred while base64 decoding the response content: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn to_base64(&self) -> Result<String, ()> {
        let p_versioning_string = serde_json::to_string(&self);

        match p_versioning_string {
            Ok(versioning_string) => Ok(BASE64_STANDARD.encode(&versioning_string)),
            Err(err) => {
                eprintln!(
                    "An error occurred while serializing the versioning struct: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn update(
        &mut self,
        repository_versioning: &Versioning,
    ) -> Result<Vec<(String, HashMap<String, String>)>, ()> {
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
            eprintln!("The repository was build with an invalid @paperback/types version {}, expected version {} or higher", repository_versioning.built_with.types, self.built_with.types);
            return Err(());
        }

        println!("Comparing the extensions of both versioning files:");

        for repository_extension in repository_versioning.sources.iter() {
            let mut new = true;

            print!("{}: ", repository_extension.name);

            for (index, registry_extension) in self.sources.iter().enumerate() {
                if repository_extension.id != registry_extension.id {
                    continue;
                }

                println!("Already exists in the registry");

                new = false;

                if repository_extension
                    .version
                    .parse::<Version>()
                    .unwrap_or_else(|_| Version::parse("0.0.0").unwrap())
                    > registry_extension.version.parse::<Version>().unwrap()
                {
                    self.sources.insert(index, repository_extension.clone());

                    updated_extensions.push((repository_extension.id.clone(), HashMap::new()));

                    println!("A newer version was found -> Updating");
                } else {
                    println!("The version was unchanged -> Leaving untouched");
                }

                break;
            }

            if new {
                self.sources.push(repository_extension.clone());

                updated_extensions.push((repository_extension.id.clone(), HashMap::new()));

                println!("Does not exist in the registry -> Adding");
            }
        }

        if updated_extensions.len() == 0 {
            eprintln!("There are no extensions to update");
            return Err(());
        }

        self.build_time = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        self.built_with.toolchain = repository_versioning.built_with.toolchain.clone();
        self.built_with.types = repository_versioning.built_with.types.clone();

        if self.repository.name == "" {
            self.repository.name = String::from("Paperback Community Extensions");
            self.repository.description = String::from(
                "All extensions from the Paperback Community combined into a single repository.",
            );
        }

        println!("Updated the local copy of the registry versioning file");

        Ok(updated_extensions)
    }
}
