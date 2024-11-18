use std::{collections::HashMap, env, process::ExitCode};

mod requests;
use requests::Requests;
mod utils;
mod versioning;
use tracing::{error, info, warn};
use versioning::Versioning;

fn main() -> ExitCode {
    #[cfg(feature = "dotenv")]
    {
        println!("Loading the .env file");
        if let Err(()) = utils::env::load_dotenv() {
            eprintln!("Exiting the program");
            return ExitCode::from(1);
        }
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
    }

    info!("Initializing the request client");
    let Ok(request_client) = Requests::new() else {
        error!("Exiting the program");
        return ExitCode::from(1);
    };

    info!("Requesting the registry versioning file");
    let mut registry_versioning;
    match request_client.get_files(
        &String::from("paperback-community/extensions"),
        &(env::var("BRANCH").unwrap() + "/versioning.json"),
        &String::from("master"),
    ) {
        Ok(requests::GetContentResponse::Struct(response)) => {
            match Versioning::new(&response.content) {
                Ok(r_registry_versioning) => registry_versioning = r_registry_versioning,
                Err(()) => {
                    error!("Exiting the program");
                    return ExitCode::from(1);
                }
            };
        }
        Ok(requests::GetContentResponse::List(_)) => {
            panic!("this API request should return a single file")
        }
        Err(not_found) => match not_found {
            true => {
                warn!(
                    "No registry versioning file found for this branch, assuming it's being created for the first time."
                );
                registry_versioning = Versioning::default()
            }
            false => {
                error!("Exiting the program");
                return ExitCode::from(1);
            }
        },
    }

    info!("Requesting the repository versioning file");
    let repository_versioning;
    match request_client.get_files(
        &env::var("REPOSITORY").unwrap(),
        &(env::var("BRANCH").unwrap() + "/versioning.json"),
        &String::from("gh-pages"),
    ) {
        Ok(requests::GetContentResponse::Struct(response)) => {
            match Versioning::new(&response.content) {
                Ok(r_repository_versioning) => repository_versioning = r_repository_versioning,
                Err(()) => {
                    error!("Exiting the program");
                    return ExitCode::from(1);
                }
            };
        }
        Ok(requests::GetContentResponse::List(_)) => {
            panic!("this API request should return a single file")
        }
        Err(_) => {
            error!("Exiting the program");
            return ExitCode::from(1);
        }
    }

    info!("Updating the local copy of the registry versioning file");
    let Ok(mut updated_extensions) = registry_versioning.update(&repository_versioning) else {
        error!("Exiting the program");
        return ExitCode::from(1);
    };

    info!(
        "Fetching the updated extensions from the repository and creating blobs for them in the registry"
    );
    for updated_extension in updated_extensions.iter_mut() {
        info!("Updating exetension: {}", updated_extension.0);
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
                        updated_extension.1.insert(response.path, blob.sha);
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
                for file in response {
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
                                    updated_extension.1.insert(response.path, blob.sha);
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
        "Converting the local copy of the registry file into base64 and creating a blob for it in the registry"
    );
    match registry_versioning.to_base64() {
        Ok(registry_versioning_base64) => {
            match request_client.create_blob(registry_versioning_base64, String::from("base64")) {
                Ok(blob) => {
                    updated_extensions.push((
                        String::from("Versioning"),
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

    info!("Fetching the latest commit and tree in the registry");
    let (registry_parent_commit, registry_parent_tree);
    match request_client.get_branch(
        &"paperback-community/extensions".to_string(),
        &"master".to_string(),
    ) {
        Ok(registry_branch) => {
            registry_parent_commit = registry_branch.clone().commit;
            registry_parent_tree = registry_branch.commit.commit.tree;
        }
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
