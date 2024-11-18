use std::{collections::HashMap, env, process::ExitCode};

mod requests;
use requests::Requests;
mod utils;
mod versioning;
use versioning::Versioning;

fn main() -> ExitCode {
    #[cfg(feature = "dotenv")]
    if let Err(()) = utils::env::load_dotenv() {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    }

    // TODO: Load logger

    if let Err(()) = utils::env::validate() {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    }

    let Ok(request_client) = Requests::new() else {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    };

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
                    eprintln!("Exiting the program");
                    return ExitCode::from(1);
                }
            };
        }
        Ok(requests::GetContentResponse::List(_)) => {
            panic!("this API request should return a single file")
        }
        Err(not_found) => match not_found {
            true => registry_versioning = Versioning::default(),
            false => {
                eprintln!("Exiting the program");
                return ExitCode::from(1);
            }
        },
    }

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
                    eprintln!("Exiting the program");
                    return ExitCode::from(1);
                }
            };
        }
        Ok(requests::GetContentResponse::List(_)) => {
            panic!("this API request should return a single file")
        }
        Err(_) => {
            eprintln!("Exiting the program");
            return ExitCode::from(1);
        }
    }

    let Ok(mut updated_extensions) = registry_versioning.update(&repository_versioning) else {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    };

    for updated_extension in updated_extensions.iter_mut() {
        println!("Requesting the updated files for {}", updated_extension.0);

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
                        eprintln!("Exiting the program");
                        return ExitCode::from(1);
                    }
                }
            }
            Err(_) => {
                eprintln!("Exiting the program");
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
                                    eprintln!("Exiting the program");
                                    return ExitCode::from(1);
                                }
                            }
                        }
                        Err(_) => {
                            eprintln!("Exiting the program");
                            return ExitCode::from(1);
                        }
                    }
                }
            }
            Ok(requests::GetContentResponse::Struct(_)) => {
                panic!("this API request should return a list of files")
            }
            Err(_) => {
                eprintln!("Exiting the program");
                return ExitCode::from(1);
            }
        }
    }

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
                    eprintln!("Exiting the program");
                    return ExitCode::from(1);
                }
            }
        }
        Err(()) => {
            eprintln!("Exiting the program");
            return ExitCode::from(1);
        }
    }

    let (registry_parent_commit, registry_parent_tree);
    match request_client.get_branch(
        &"paperback-community/extensions-test".to_string(),
        &"master".to_string(),
    ) {
        Ok(registry_branch) => {
            registry_parent_commit = registry_branch.clone().commit;
            registry_parent_tree = registry_branch.commit.commit.tree;
        }
        Err(()) => {
            eprintln!("Exiting the program");
            return ExitCode::from(1);
        }
    };

    println!("Got branch");

    let Ok(registry_update_tree) =
        request_client.create_tree(registry_parent_tree.sha, updated_extensions)
    else {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    };

    println!("Created tree");

    let Ok(registry_update_commit) = request_client.create_commit(
        env::var("COMMIT_MESSAGE").unwrap_or_else(|_| String::from("Registry update")),
        registry_update_tree.sha,
        registry_parent_commit.sha,
        env::var("COMMIT_AUTHOR_NAME").unwrap_or_else(|_| String::from("Paperback Community")),
        env::var("COMMIT_AUTHOR_EMAIL").unwrap_or_else(|_| {
            String::from("github-action@actions-registry-manager.noreply.github.com")
        }),
    ) else {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    };

    println!("Created commit");

    if let Err(()) = request_client.update_reference(registry_update_commit.sha) {
        eprintln!("Exiting the program");
        return ExitCode::from(1);
    }

    println!("Updated reference");

    // TODO:
    // V - Get all files of the updated extensions from the extension repository
    // V - Get last commit and tree of the master branch in the registry repository
    // V - Create a new tree in the registry repository using the new files
    // V - Create a commit using that new tree in the registry repository
    // V - Point the master branch reference in the registry to the new commit
    // V - Switch to better Result match cases
    // - Clean code up
    // - Switch from std prints to tracing and clean logs up

    ExitCode::from(0)
}
