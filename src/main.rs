use std::{collections::HashMap, env, process::ExitCode};

mod requests;
use requests::{FileOutputFormat, Requests};
mod utils;
mod versioning;
use serde::Serialize;
use tracing::{error, info, warn};
use versioning::{JsonFileAsStruct, ManageTypes, ManagedExtensions, Metadata, Versioning};

fn main() -> ExitCode {
    if let Ok(()) = run() {
        info!("Exiting the tool");
        ExitCode::from(0)
    } else {
        error!("Exiting the tool");
        ExitCode::from(1)
    }
}

fn run() -> Result<(), ()> {
    let request_client = initialization()?;

    info!("Requesting the registry versioning file");
    let (mut registry_versioning, mut registry_metadata, versioning_manage_type) =
        request_registry_versioning_metadata_files(&request_client)?;

    info!("Requesting the repository versioning file");
    let repository_versioning = request_repository_versioning_file(&request_client)?;

    info!("Updating the local copy of the registry versioning and metadata files");
    let mut managed_extensions =
        registry_versioning.update(&mut registry_metadata, &repository_versioning)?;

    if managed_extensions.is_empty() {
        return Ok(());
    }

    info!(
        "Fetching the added and updated extensions from the repository and creating blobs for them in the registry"
    );
    extension_management(&request_client, &mut managed_extensions)?;

    info!("Creating a blob from the local copy of the registry versioning file in the registry.");
    create_registry_json_file_blob::<Versioning>(
        &request_client,
        &registry_versioning,
        &versioning_manage_type,
        "Versioning",
        &mut managed_extensions,
    )?;

    info!("Creating a blob from the local copy of the registry metadata file in the registry.");
    create_registry_json_file_blob::<Metadata>(
        &request_client,
        &registry_metadata,
        &versioning_manage_type,
        "Metadata",
        &mut managed_extensions,
    )?;

    info!("Fetching the latest commit and tree in the registry");
    let registry_branch = request_client.get_branch(
        &String::from("paperback-community/extensions"),
        &String::from("master"),
    )?;

    info!("Creating a new tree in the registry");
    let registry_update_tree = request_client.create_tree(
        registry_branch.commit.commit.tree.sha.clone(),
        managed_extensions,
    )?;

    info!("Creating a new commit in the registry");
    let registry_update_commit = request_client.create_commit(
        env::var("COMMIT_MESSAGE").unwrap_or_else(|_| {
            format!(
                "Registry management ({}, {})",
                env::var("REPOSITORY")
                    .unwrap()
                    .strip_prefix("paperback-community/")
                    .unwrap(),
                env::var("BRANCH").unwrap(),
            )
        }),
        registry_update_tree.sha,
        registry_branch.commit.sha,
        env::var("COMMIT_AUTHOR_NAME").unwrap_or_else(|_| String::from("github-actions[bot]")),
        env::var("COMMIT_AUTHOR_EMAIL")
            .unwrap_or_else(|_| String::from("github-actions[bot]@users.noreply.github.com")),
    )?;

    info!("Updating the reference in the registry");
    request_client.update_reference(registry_update_commit.sha)?;

    info!("Succesfully updated the registry");
    Ok(())
}

fn initialization() -> Result<Requests, ()> {
    #[cfg(feature = "dotenv")]
    {
        println!("Loading the .env file");
        utils::env::load_dotenv()?;
    }

    println!("Initializing the logger");
    utils::logger::new()?;

    info!("Validating the environment variables (REPOSITORY, BRANCH)");
    utils::env::validate()?;

    info!("Initializing the request client");
    Requests::new()
}

fn request_registry_versioning_metadata_files(
    request_client: &Requests,
) -> Result<(Box<Versioning>, Box<Metadata>, ManageTypes), ()> {
    match request_client.get_file(
        &String::from("paperback-community/extensions"),
        &(env::var("BRANCH").unwrap() + "/versioning.json"),
        &String::from("master"),
        &FileOutputFormat::UTF8,
    ) {
        Ok(response) => {
            if let Ok(registry_versioning) = Versioning::new(&response) {
                info!("Requesting the registry metadata file");
                if let Ok(response) = request_client.get_file(
                    &String::from("paperback-community/extensions"),
                    &(env::var("BRANCH").unwrap() + "/metadata.json"),
                    &String::from("master"),
                    &FileOutputFormat::UTF8,
                ) {
                    if let Ok(registry_metadata) = Metadata::new(&response) {
                        return Ok((registry_versioning, registry_metadata, ManageTypes::Update));
                    }
                }
            }
        }
        Err(false) => {
            warn!(
                "No registry versioning file found for this branch, assuming it's being created for the first time."
            );
            return Ok((
                Box::new(Versioning::default()),
                Box::new(Metadata::default()),
                ManageTypes::Addition,
            ));
        }
        Err(true) => (),
    }

    Err(())
}

fn request_repository_versioning_file(request_client: &Requests) -> Result<Box<Versioning>, ()> {
    if let Ok(response) = request_client.get_file(
        &env::var("REPOSITORY").unwrap(),
        &(env::var("BRANCH").unwrap() + "/versioning.json"),
        &String::from("gh-pages"),
        &FileOutputFormat::UTF8,
    ) {
        return Versioning::new(&response);
    }

    Err(())
}

fn extension_management(
    request_client: &Requests,
    managed_extensions: &mut ManagedExtensions,
) -> Result<(), ()> {
    for managed_extension in managed_extensions {
        let (repository, branch) = match managed_extension.1 {
            ManageTypes::Addition => {
                info!("Adding extension: {}", managed_extension.0);
                (&env::var("REPOSITORY").unwrap(), &String::from("gh-pages"))
            }
            ManageTypes::Update => {
                info!("Updating extension: {}", managed_extension.0);
                (&env::var("REPOSITORY").unwrap(), &String::from("gh-pages"))
            }
            ManageTypes::Deletion => {
                info!("Deleting extension: {}", managed_extension.0);
                (
                    &String::from("paperback-community/extensions"),
                    &String::from("master"),
                )
            }
        };

        if managed_extension.1 == ManageTypes::Deletion {
            managed_extension.2.insert(
                env::var("BRANCH").unwrap() + "/" + &managed_extension.0 + "/index.js",
                None,
            );
        } else if let Ok(response) = request_client.get_file(
            &env::var("REPOSITORY").unwrap(),
            &(env::var("BRANCH").unwrap() + "/" + &managed_extension.0 + "/index.js"),
            &String::from("gh-pages"),
            &FileOutputFormat::UTF8,
        ) {
            if let Ok(blob) = request_client.create_blob(response, String::from("utf-8")) {
                managed_extension.2.insert(
                    env::var("BRANCH").unwrap() + "/" + &managed_extension.0 + "/index.js",
                    Some(blob.sha),
                );
            } else {
                return Err(());
            }
        } else {
            return Err(());
        }

        if let Ok(response) = request_client.get_directory(
            repository,
            &(env::var("BRANCH").unwrap() + "/" + &managed_extension.0 + "/static"),
            branch,
        ) {
            for file in response {
                if file.etype != "file" {
                    continue;
                }

                if managed_extension.1 == ManageTypes::Deletion {
                    managed_extension.2.insert(file.path.clone(), None);
                } else if let Ok(response) = request_client.get_file(
                    &env::var("REPOSITORY").unwrap(),
                    &file.path,
                    &String::from("gh-pages"),
                    &FileOutputFormat::BASE64,
                ) {
                    if let Ok(blob) = request_client.create_blob(response, String::from("base64")) {
                        managed_extension.2.insert(file.path, Some(blob.sha));
                    } else {
                        return Err(());
                    }
                } else {
                    return Err(());
                }
            }
        } else {
            return Err(());
        }
    }

    Ok(())
}

fn create_registry_json_file_blob<JFAS: JsonFileAsStruct + Serialize>(
    request_client: &Requests,
    registry_versioning: &JFAS,
    versioning_manage_type: &ManageTypes,
    name: &str,
    managed_extensions: &mut ManagedExtensions,
) -> Result<(), ()> {
    if let Ok(registry_versioning_string) = registry_versioning.to_utf8() {
        if let Ok(blob) =
            request_client.create_blob(registry_versioning_string, String::from("utf-8"))
        {
            managed_extensions.push((
                name.to_string(),
                versioning_manage_type.clone(),
                HashMap::from([(
                    env::var("BRANCH").unwrap() + "/" + name.to_lowercase().as_str() + ".json",
                    Some(blob.sha),
                )]),
            ));

            return Ok(());
        }
    }

    Err(())
}
