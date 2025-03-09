use std::{collections::HashMap, env, process::ExitCode};

mod requests;
use requests::Requests;
mod utils;
mod versioning;
use tracing::{error, info, warn};
use versioning::{Metadata, UpdateTypes, Versioning};

fn main() -> ExitCode {
    #[cfg(feature = "dotenv")]
    {
        println!("Loading the .env file");
        if let Err(()) = utils::env::load_dotenv() {
            eprintln!("Exiting the program");
            return ExitCode::from(1);
        };
    }

    println!("Initializing the logger");
    if let Err(()) = utils::logger::new() {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    };

    info!("Validating the environment variables (REPOSITORY, BRANCH)");
    if let Err(()) = utils::env::validate() {
        error!("Exiting the program");
        return ExitCode::from(1);
    };

    info!("Initializing the request client");
    let Ok(request_client) = Requests::new() else {
        error!("Exiting the program");
        return ExitCode::from(1);
    };

    info!("Requesting the registry versioning file");
    let (mut registry_versioning, mut registry_metadata, versioning_update_type) =
        match request_client.get_files(
            &String::from("paperback-community/extensions"),
            &(env::var("BRANCH").unwrap() + "/versioning.json"),
            &String::from("master"),
        ) {
            Ok(requests::GetContentResponse::Struct(response)) => {
                match Versioning::new(&response.content) {
                    Ok(registry_versioning) => {
                        info!("Requesting the registry metadata file");
                        match request_client.get_files(
                            &String::from("paperback-community/extensions"),
                            &(env::var("BRANCH").unwrap() + "/metadata.json"),
                            &String::from("master"),
                        ) {
                            Ok(requests::GetContentResponse::Struct(response)) => {
                                match Metadata::new(&response.content) {
                                    Ok(registry_metadata) => (
                                        registry_versioning,
                                        registry_metadata,
                                        UpdateTypes::Update,
                                    ),
                                    Err(()) => {
                                        error!("Exiting the program");
                                        return ExitCode::from(1);
                                    }
                                }
                            }
                            Ok(requests::GetContentResponse::List(_)) => {
                                panic!("this API request should return a single file")
                            }
                            Err(_) => {
                                error!("Exiting the program");
                                return ExitCode::from(1);
                            }
                        }
                    }
                    Err(()) => {
                        error!("Exiting the program");
                        return ExitCode::from(1);
                    }
                }
            }
            Ok(requests::GetContentResponse::List(_)) => {
                panic!("this API request should return a single file")
            }
            Err(false) => {
                warn!(
                    "No registry versioning file found for this branch, assuming it's being created for the first time."
                );
                (
                    Versioning::default(),
                    Metadata::default(),
                    UpdateTypes::Addition,
                )
            }
            Err(true) => {
                error!("Exiting the program");
                return ExitCode::from(1);
            }
        };

    info!("Requesting the repository versioning file");
    let repository_versioning = match request_client.get_files(
        &env::var("REPOSITORY").unwrap(),
        &(env::var("BRANCH").unwrap() + "/versioning.json"),
        &String::from("gh-pages"),
    ) {
        Ok(requests::GetContentResponse::Struct(response)) => {
            match Versioning::new(&response.content) {
                Ok(repository_versioning) => repository_versioning,
                Err(()) => {
                    error!("Exiting the program");
                    return ExitCode::from(1);
                }
            }
        }
        Ok(requests::GetContentResponse::List(_)) => {
            panic!("this API request should return a single file")
        }
        Err(_) => {
            error!("Exiting the program");
            return ExitCode::from(1);
        }
    };

    info!("Updating the local copy of the registry versioning file");
    let mut updated_extensions =
        match registry_versioning.update(&mut registry_metadata, repository_versioning) {
            Ok(updated_extensions) => updated_extensions,
            Err(()) => {
                error!("Exiting the program");
                return ExitCode::from(1);
            }
        };

    if updated_extensions.is_empty() {
        info!("Exiting the program");
        return ExitCode::from(0);
    }

    info!(
        "Fetching the updated extensions from the repository and creating blobs for them in the registry"
    );
    for updated_extension in updated_extensions.iter_mut() {
        if updated_extension.1 == UpdateTypes::Deletion {
            continue;
        }

        info!("Updating extension: {}", updated_extension.0);

        match request_client.get_files(
            &env::var("REPOSITORY").unwrap(),
            &(env::var("BRANCH").unwrap() + "/" + &updated_extension.0 + "/index.js"),
            &String::from("gh-pages"),
        ) {
            Ok(requests::GetContentResponse::List(_)) => {
                panic!("this API request should return a single file")
            }
            Ok(requests::GetContentResponse::Struct(response)) => {
                match request_client.create_blob(response.content, String::from("base64")) {
                    Ok(blob) => {
                        updated_extension.2.insert(response.path, blob.sha);
                    }
                    Err(()) => {
                        error!("Exiting the program");
                        return ExitCode::from(1);
                    }
                }
            }
            Err(_) => {
                error!("Exiting the program");
                return ExitCode::from(1);
            }
        }

        match request_client.get_files(
            &env::var("REPOSITORY").unwrap(),
            &(env::var("BRANCH").unwrap() + "/" + &updated_extension.0 + "/static"),
            &String::from("gh-pages"),
        ) {
            Ok(requests::GetContentResponse::List(response)) => {
                for file in response.iter() {
                    if file._type != "file" {
                        continue;
                    }

                    match request_client.get_files(
                        &env::var("REPOSITORY").unwrap(),
                        &file.path,
                        &String::from("gh-pages"),
                    ) {
                        Ok(requests::GetContentResponse::List(_)) => {
                            panic!("this API request should return a single file")
                        }
                        Ok(requests::GetContentResponse::Struct(response)) => {
                            match request_client
                                .create_blob(response.content, String::from("base64"))
                            {
                                Ok(blob) => {
                                    updated_extension.2.insert(response.path, blob.sha);
                                }
                                Err(()) => {
                                    error!("Exiting the program");
                                    return ExitCode::from(1);
                                }
                            }
                        }
                        Err(_) => {
                            error!("Exiting the program");
                            return ExitCode::from(1);
                        }
                    }
                }
            }
            Ok(requests::GetContentResponse::Struct(_)) => {
                panic!("this API request should return a list of files")
            }
            Err(_) => {
                error!("Exiting the program");
                return ExitCode::from(1);
            }
        }
    }

    info!(
        "Converting the local copy of the registry versioning file into base64 and creating a blob for it in the registry"
    );
    match registry_versioning.to_base64() {
        Ok(registry_versioning_base64) => {
            match request_client.create_blob(registry_versioning_base64, String::from("base64")) {
                Ok(blob) => {
                    updated_extensions.push((
                        String::from("Versioning"),
                        versioning_update_type.clone(),
                        HashMap::from([(
                            env::var("BRANCH").unwrap() + "/" + "versioning.json",
                            blob.sha,
                        )]),
                    ));
                }
                Err(()) => {
                    error!("Exiting the program");
                    return ExitCode::from(1);
                }
            }
        }
        Err(()) => {
            error!("Exiting the program");
            return ExitCode::from(1);
        }
    }

    info!(
        "Converting the local copy of the registry metadata file into base64 and creating a blob for it in the registry"
    );
    match registry_metadata.to_base64() {
        Ok(registry_metadata_base64) => {
            match request_client.create_blob(registry_metadata_base64, String::from("base64")) {
                Ok(blob) => {
                    updated_extensions.push((
                        String::from("Metadata"),
                        versioning_update_type,
                        HashMap::from([(
                            env::var("BRANCH").unwrap() + "/" + "metadata.json",
                            blob.sha,
                        )]),
                    ));
                }
                Err(()) => {
                    error!("Exiting the program");
                    return ExitCode::from(1);
                }
            }
        }
        Err(()) => {
            error!("Exiting the program");
            return ExitCode::from(1);
        }
    }

    info!("Fetching the latest commit and tree in the registry");
    let (registry_parent_tree, registry_parent_commit) = match request_client.get_branch(
        &String::from("paperback-community/extensions"),
        &String::from("master"),
    ) {
        Ok(registry_branch) => (
            registry_branch.commit.commit.tree.clone(),
            registry_branch.commit,
        ),
        Err(()) => {
            error!("Exiting the program");
            return ExitCode::from(1);
        }
    };

    info!("Creating a new tree in the registry");
    let Ok(registry_update_tree) =
        request_client.create_tree(registry_parent_tree.sha, updated_extensions)
    else {
        error!("Exiting the program");
        return ExitCode::from(1);
    };

    info!("Creating a new commit in the registry");
    let Ok(registry_update_commit) = request_client.create_commit(
        env::var("COMMIT_MESSAGE").unwrap_or_else(|_| String::from("Registry update")),
        registry_update_tree.sha,
        registry_parent_commit.sha,
        env::var("COMMIT_AUTHOR_NAME").unwrap_or_else(|_| String::from("github-actions[bot]")),
        env::var("COMMIT_AUTHOR_EMAIL")
            .unwrap_or_else(|_| String::from("github-actions[bot]@users.noreply.github.com")),
    ) else {
        error!("Exiting the program");
        return ExitCode::from(1);
    };

    info!("Updating the reference in the registry");
    if let Err(()) = request_client.update_reference(registry_update_commit.sha) {
        error!("Exiting the program");
        return ExitCode::from(1);
    }

    info!("Succesfully published the extensions to the registry");
    info!("Exiting the program");
    ExitCode::from(0)
}
