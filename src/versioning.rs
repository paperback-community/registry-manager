use base64::prelude::*;
use chrono::Utc;
use node_semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltWith {
    toolchain: String,
    types: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    name: String,
    description: String,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Versioning {
    build_time: String,
    built_with: BuiltWith,
    repository: Repository,
    sources: Vec<Source>,
}

pub fn parse_versioning(response_content: &String) -> Result<Versioning, u8> {
    match BASE64_STANDARD.decode(response_content.replace("\n", "")) {
        Ok(bytes) => match serde_json::from_slice(bytes.as_slice()) {
            Ok(versioning) => Ok(versioning),
            Err(err) => {
                eprintln!(
                    "An error occurred while deserializing the response content to JSON: {}",
                    &err
                );
                Err(0x1)
            }
        },
        Err(err) => {
            eprintln!(
                "An error occurred while base64 decoding the response content: {}",
                &err
            );
            Err(0x1)
        }
    }
}

pub fn update_registry_versioning(
    registry_versioning: &mut Versioning,
    repository_versioning: &Versioning,
) -> Result<Vec<(String, bool)>, u8> {
    let mut updated_extensions = vec![];

    if registry_versioning
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
        eprintln!(
            "The repository was build with an invalid @paperback/types version {}, expected version {} or higher",
            repository_versioning.built_with.types,
            registry_versioning.built_with.types
        );
        return Err(0x1);
    }

    for repository_extension in repository_versioning.sources.iter() {
        let mut new = true;

        for (index, registry_extension) in registry_versioning.sources.iter().enumerate() {
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
                registry_versioning
                    .sources
                    .insert(index, repository_extension.clone());

                updated_extensions.push((repository_extension.id.clone(), new));
            }

            break;
        }

        if new {
            registry_versioning
                .sources
                .push(repository_extension.clone());

            updated_extensions.push((repository_extension.id.clone(), new));
        }
    }

    if updated_extensions.len() == 0 {
        eprintln!("There are no extensions to update");
        return Err(0x1);
    }

    registry_versioning.build_time = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    registry_versioning.built_with.toolchain = repository_versioning.built_with.toolchain.clone();
    registry_versioning.built_with.types = repository_versioning.built_with.types.clone();

    Ok(updated_extensions)
}
