use std::{collections::HashMap, env};

use chrono::Utc;
use node_semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeSeq};
use tracing::{error, warn};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Versioning {
    build_time: String,
    built_with: BuiltWith,
    repository: Repository,
    #[serde(
        serialize_with = "sources_serialize",
        deserialize_with = "sources_deserialize"
    )]
    sources: HashMap<String, Source>,
}

fn sources_serialize<S>(sources: &HashMap<String, Source>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(sources.len()))?;
    for source in sources.values() {
        seq.serialize_element(&source)?;
    }
    seq.end()
}

fn sources_deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Source>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Vec::<Source>::deserialize(deserializer)?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect())
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(flatten)]
    repositories: HashMap<String, MetadataRepository>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataRepository {
    #[serde(flatten)]
    extensions: HashMap<String, MetadataExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataExtension {
    build_time: String,
    built_with: BuiltWith,
}

pub type UpdatedExtensions = Vec<(String, UpdateTypes, HashMap<String, Option<String>>)>;

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateTypes {
    Addition,
    Update,
    Deletion,
}

impl Versioning {
    pub fn new(response_content: &str) -> Result<Versioning, ()> {
        match serde_json::from_str(response_content) {
            Ok(versioning) => Ok(versioning),
            Err(err) => {
                error!(
                    "An error occurred while deserializing the response content to JSON: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn to_string(&self) -> Result<String, ()> {
        match serde_json::to_string_pretty(&self) {
            Ok(versioning_string) => Ok(versioning_string),
            Err(err) => {
                error!(
                    "An error occurred while serializing the versioning file into UTF-8: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn update(
        &mut self,
        metadata: &mut Metadata,
        repository_versioning: Versioning,
    ) -> Result<UpdatedExtensions, ()> {
        if self
            .built_with
            .types
            .parse::<Version>()
            .unwrap_or_else(|_| Version::parse("0.9.0").unwrap())
            > repository_versioning
                .built_with
                .types
                .parse::<Version>()
                .unwrap()
        {
            error!(
                "The repository was build with a @paperback/types version {} which was too low, expected version {} or higher",
                repository_versioning.built_with.types, self.built_with.types
            );
            return Err(());
        }

        let mut updated_extensions = vec![];

        let mut shared_extensions = vec![];

        let mut repository_extensions = repository_versioning
            .sources
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        let mut registry_extensions = metadata
            .repositories
            .get(&env::var("REPOSITORY").unwrap())
            .cloned()
            .unwrap_or_default()
            .extensions
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        for repository_index in (0..repository_extensions.len()).rev() {
            for registry_index in (0..registry_extensions.len()).rev() {
                if repository_extensions[repository_index] == registry_extensions[registry_index] {
                    shared_extensions.push(repository_extensions[repository_index].clone());
                    repository_extensions.remove(repository_index);
                    registry_extensions.remove(registry_index);

                    break;
                }
            }
        }

        metadata
            .repositories
            .entry(env::var("REPOSITORY").unwrap())
            .or_insert_with(|| MetadataRepository {
                extensions: HashMap::new(),
            });

        for extension in repository_extensions.iter() {
            self.sources.insert(
                extension.to_string(),
                repository_versioning
                    .sources
                    .get(extension)
                    .unwrap()
                    .clone(),
            );

            metadata
                .repositories
                .get_mut(&env::var("REPOSITORY").unwrap())
                .unwrap()
                .extensions
                .insert(
                    extension.to_string(),
                    MetadataExtension {
                        build_time: repository_versioning.build_time.clone(),
                        built_with: repository_versioning.built_with.clone(),
                    },
                );

            updated_extensions.push((extension.clone(), UpdateTypes::Addition, HashMap::new()));
        }

        for extension in shared_extensions.iter() {
            if repository_versioning
                .sources
                .get(extension)
                .unwrap()
                .version
                .parse::<Version>()
                .unwrap_or_else(|_| Version::parse("0.0.0").unwrap())
                > self
                    .sources
                    .get(extension)
                    .unwrap()
                    .version
                    .parse::<Version>()
                    .unwrap()
            {
                self.sources.insert(
                    extension.to_string(),
                    repository_versioning
                        .sources
                        .get(extension)
                        .unwrap()
                        .clone(),
                );

                updated_extensions.push((extension.clone(), UpdateTypes::Update, HashMap::new()));
            }
        }

        for extension in registry_extensions.iter() {
            self.sources.remove(extension);

            metadata
                .repositories
                .get_mut(&env::var("REPOSITORY").unwrap())
                .unwrap()
                .extensions
                .remove(extension);

            updated_extensions.push((extension.clone(), UpdateTypes::Deletion, HashMap::new()));
        }

        if updated_extensions.is_empty() {
            warn!("There are no extensions to update");
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

impl Metadata {
    pub fn new(response_content: &str) -> Result<Metadata, ()> {
        match serde_json::from_str(response_content) {
            Ok(metadata) => Ok(metadata),
            Err(err) => {
                error!(
                    "An error occurred while deserializing the response content to JSON: {}",
                    &err
                );
                Err(())
            }
        }
    }

    pub fn to_string(&self) -> Result<String, ()> {
        match serde_json::to_string(&self) {
            Ok(metadata_string) => Ok(metadata_string),
            Err(err) => {
                error!(
                    "An error occurred while serializing the metadata file into UTF-8: {}",
                    &err
                );
                Err(())
            }
        }
    }
}
