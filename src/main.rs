use std::{env, process::ExitCode};

use crate::requests::Requests;

mod requests;
mod utils;
mod versioning;

fn main() -> ExitCode {
    #[cfg(feature = "dotenv")]
    if let Err(exit_code) = utils::env::load_dotenv() {
        eprintln!("Exiting the program");
        return ExitCode::from(exit_code);
    }

    if let Err(exit_code) = utils::env::validate() {
        eprintln!("Exiting the program");
        return ExitCode::from(exit_code);
    }

    let request_client: Requests<T>;
    match requests::new() {
        Ok(r_request_client) => request_client = r_request_client,
        Err(exit_code) => {
            eprintln!("Exiting the program");
            return ExitCode::from(exit_code);
        }
    }

    let mut registry_versioning;
    match request_client.get_files(
        &"paperback-community/extensions".to_string(),
        &"versioning.json".to_string(),
        &"master".to_string(),
    ) {
        Ok(requests::GetContent::Struct(response)) => {
            match versioning::parse_versioning(&response.content) {
                Ok(versioning) => {
                    registry_versioning = versioning;
                }
                Err(exitcode) => {
                    eprintln!("Exiting the program");
                    return ExitCode::from(exitcode);
                }
            }
        }
        Ok(requests::GetContent::List(_)) => {
            panic!("this API request should return a single file")
        }
        Err(exitcode) => {
            eprintln!("Exiting the program");
            return ExitCode::from(exitcode);
        }
    }

    let repository_versioning;
    match request_client.get_files(
        &env::var("REGISTRY_MANAGER_REPOSITORY").unwrap(),
        &"versioning.json".to_string(),
        &"gh-pages".to_string(),
    ) {
        Ok(requests::GetContent::Struct(response)) => {
            match versioning::parse_versioning(&response.content) {
                Ok(versioning) => {
                    repository_versioning = versioning;
                }
                Err(exitcode) => {
                    eprintln!("Exiting the program");
                    return ExitCode::from(exitcode);
                }
            }
        }
        Ok(requests::GetContent::List(_)) => {
            panic!("this API request should return a single file")
        }
        Err(exitcode) => {
            eprintln!("Exiting the program");
            return ExitCode::from(exitcode);
        }
    }

    let mut updated_extensions;
    match versioning::update_registry_versioning(&mut registry_versioning, &repository_versioning) {
        Ok(r_updated_extensions) => updated_extensions = r_updated_extensions,
        Err(exitcode) => {
            eprintln!("Exiting the program");
            return ExitCode::from(exitcode);
        }
    }

    println!("{:#?}", updated_extensions);

    for updated_extension in updated_extensions.iter_mut() {
        match request_client.get_files(
            &env::var("REGISTRY_MANAGER_REPOSITORY").unwrap(),
            &format!("{}/index.js", &updated_extension.0),
            &"gh-pages".to_string(),
        ) {
            Ok(requests::GetContent::List(_)) => {
                panic!("this API request should return a single file")
            }
            Ok(requests::GetContent::Struct(response)) => {
                println!("Requested file: {}", response.path);
                updated_extension.2.insert(response.path, response.content);
            }
            Err(exit_code) => {
                eprintln!("Exiting the program");
                return ExitCode::from(exit_code);
            }
        }

        match request_client.get_files(
            &env::var("REGISTRY_MANAGER_REPOSITORY").unwrap(),
            &format!("{}/static", &updated_extension.0),
            &"gh-pages".to_string(),
        ) {
            Ok(requests::GetContent::List(response)) => {
                for file in response {
                    match request_client.get_files(
                        &env::var("REGISTRY_MANAGER_REPOSITORY").unwrap(),
                        &file.path,
                        &"gh-pages".to_string(),
                    ) {
                        Ok(requests::GetContent::List(_)) => {
                            panic!("this API request should return a single file")
                        }
                        Ok(requests::GetContent::Struct(response)) => {
                            println!("Requested file: {}", response.path);
                            updated_extension.2.insert(response.path, response.content);
                        }
                        Err(exit_code) => {
                            eprintln!("Exiting the program");
                            return ExitCode::from(exit_code);
                        }
                    }
                }
            }
            Ok(requests::GetContent::Struct(_)) => {
                panic!("this API request should return a list of files")
            }
            Err(exit_code) => {
                eprintln!("Exiting the program");
                return ExitCode::from(exit_code);
            }
        }
    }

    // TODO:
    // - Get all files of the updated extensions from the extension repository
    // - Create blobs for the files in the registry repository
    // - Create a new tree using the new blobs in the registry repository
    // - Create a commit using that new tree in the registry repository
    // - Switch to specific exit codesj
    // - Switch from std prints to tracing

    ExitCode::from(0x0)
}
