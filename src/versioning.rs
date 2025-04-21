use std::{
    collections::{BTreeMap, HashMap},
    env,
};

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
    sources: BTreeMap<String, Source>,
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
    repositories: BTreeMap<String, MetadataRepository>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataRepository {
    #[serde(flatten)]
    extensions: BTreeMap<String, MetadataExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataExtension {
    build_time: String,
    built_with: BuiltWith,
}

pub type ManagedExtensions = Vec<(String, ManageTypes, HashMap<String, Option<String>>)>;

#[derive(Debug, Clone, PartialEq)]
pub enum ManageTypes {
    Addition,
    Update,
    Deletion,
}

pub trait JsonFileAsStruct {
    fn new<'s>(response: &'s str) -> Result<Box<Self>, ()>
    where
        Self: Deserialize<'s>,
    {
        match serde_json::from_str(response) {
            Ok(file) => Ok(Box::new(file)),
            Err(err) => {
                error!(
                    "An error occurred while deserializing the response JSON to a struct: {}",
                    &err
                );
                Err(())
            }
        }
    }

    fn to_utf8(&self) -> Result<String, ()>
    where
        Self: Serialize,
    {
        match serde_json::to_string_pretty(&self) {
            Ok(string) => Ok(string),
            Err(err) => {
                error!(
                    "An error occurred while serializing the struct into UTF-8 JSON: {}",
                    &err
                );
                Err(())
            }
        }
    }
}

fn sources_serialize<S>(
    sources: &BTreeMap<String, Source>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(sources.len()))?;
    for source in sources.values() {
        seq.serialize_element(&source)?;
    }
    seq.end()
}

fn sources_deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<String, Source>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Vec::<Source>::deserialize(deserializer)?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect())
}

impl JsonFileAsStruct for Versioning {}

impl Versioning {
    pub fn update(
        &mut self,
        metadata: &mut Metadata,
        repository_versioning: &Versioning,
    ) -> Result<ManagedExtensions, ()> {
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

        let mut managed_extensions = vec![];

        let mut shared_extensions = vec![];

        let mut repository_extensions = repository_versioning
            .sources
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        let mut registry_extensions = metadata
            .repositories
            .get(
                env::var("REPOSITORY")
                    .unwrap()
                    .strip_prefix("paperback-community/")
                    .unwrap(),
            )
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
            .entry(
                env::var("REPOSITORY")
                    .unwrap()
                    .strip_prefix("paperback-community/")
                    .unwrap()
                    .to_string(),
            )
            .or_insert_with(|| MetadataRepository {
                extensions: BTreeMap::new(),
            });

        self.extension_additions(
            repository_versioning,
            metadata,
            &repository_extensions,
            &mut managed_extensions,
        );

        self.extension_updates(
            repository_versioning,
            &shared_extensions,
            &mut managed_extensions,
        );

        self.extension_deletions(metadata, &registry_extensions, &mut managed_extensions);

        if managed_extensions.is_empty() {
            warn!("There are no extensions to manage");
            return Ok(managed_extensions);
        }

        self.build_time = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        self.built_with
            .toolchain
            .clone_from(&repository_versioning.built_with.toolchain);
        self.built_with
            .types
            .clone_from(&repository_versioning.built_with.types);

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

        Ok(managed_extensions)
    }

    fn extension_additions(
        &mut self,
        repository_versioning: &Versioning,
        metadata: &mut Metadata,
        repository_extensions: &Vec<String>,
        managed_extensions: &mut ManagedExtensions,
    ) {
        for extension in repository_extensions {
            if extension.ends_with("Template") {
                warn!("Detected a template extension, ignoring it");
                continue;
            }

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
                .get_mut(
                    env::var("REPOSITORY")
                        .unwrap()
                        .strip_prefix("paperback-community/")
                        .unwrap(),
                )
                .unwrap()
                .extensions
                .insert(
                    extension.to_string(),
                    MetadataExtension {
                        build_time: repository_versioning.build_time.clone(),
                        built_with: repository_versioning.built_with.clone(),
                    },
                );

            managed_extensions.push((extension.clone(), ManageTypes::Addition, HashMap::new()));
        }
    }

    fn extension_updates(
        &mut self,
        repository_versioning: &Versioning,
        shared_extensions: &Vec<String>,
        managed_extensions: &mut ManagedExtensions,
    ) {
        for extension in shared_extensions {
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

                managed_extensions.push((extension.clone(), ManageTypes::Update, HashMap::new()));
            }
        }
    }

    fn extension_deletions(
        &mut self,
        metadata: &mut Metadata,
        registry_extensions: &Vec<String>,
        managed_extensions: &mut ManagedExtensions,
    ) {
        for extension in registry_extensions {
            self.sources.remove(extension);

            metadata
                .repositories
                .get_mut(
                    env::var("REPOSITORY")
                        .unwrap()
                        .strip_prefix("paperback-community/")
                        .unwrap(),
                )
                .unwrap()
                .extensions
                .remove(extension);

            if metadata
                .repositories
                .get(
                    env::var("REPOSITORY")
                        .unwrap()
                        .strip_prefix("paperback-community/")
                        .unwrap(),
                )
                .unwrap()
                .extensions
                .is_empty()
            {
                metadata.repositories.remove(
                    env::var("REPOSITORY")
                        .unwrap()
                        .strip_prefix("paperback-community/")
                        .unwrap(),
                );
            }

            managed_extensions.push((extension.clone(), ManageTypes::Deletion, HashMap::new()));
        }
    }
}

impl JsonFileAsStruct for Metadata {}
