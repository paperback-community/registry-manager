use std::{env, process::ExitCode};

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

    let request_client = requests::new();

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

    let updated_extensions;
    match versioning::update_registry_versioning(&mut registry_versioning, &repository_versioning) {
        Ok(r_updated_extensions) => updated_extensions = r_updated_extensions,
        Err(exitcode) => {
            eprintln!("Exiting the program");
            return ExitCode::from(exitcode);
        }
    }

    println!("{:#?}", updated_extensions);

    // TODO:
    // - Get all files of the updated extensions from the extension repository
    // - Create blobs for the files in the registry repository
    // - Create a new tree using the new blobs in the registry repository
    // - Create a commit using that new tree in the registry repository
    // - Switch from std prints to tracing

    ExitCode::from(0x0)
}
