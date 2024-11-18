use std::env;

#[cfg(feature = "dotenv")]
use dotenvy;

#[cfg(feature = "dotenv")]
pub fn load_dotenv() -> Result<(), ()> {
    match dotenvy::dotenv() {
        Ok(_) => {
            println!("Loaded the .env file");
            Ok(())
        }
        Err(err) => {
            eprintln!(
                "An error occurred wile trying to load the .env file: {}",
                &err
            );
            Err(())
        }
    }
}

pub fn validate() -> Result<(), ()> {
    match env::var("PAT") {
        Ok(value) => {
            if !value.to_string().starts_with("github_pat_") || value.to_string().len() != 93 {
                eprintln!("The provided personal access token is invalid, for more info check https://github.blog/security/application-security/introducing-fine-grained-personal-access-tokens-for-github/");
                return Err(());
            }
        }
        Err(_) => {
            eprintln!("The PAT environment variable was not set");
            return Err(());
        }
    };

    match env::var("REPOSITORY") {
        Ok(value) => {
            if !value.to_string().starts_with("paperback-community/")
                || value.to_string().len() < 20
            {
                eprintln!("The provided repository is invalid, it should be of the structure \"paperback-community/<repository_name>\", consider using \"$${{ github.repository_name }}\"");
                return Err(());
            }
        }
        Err(_) => {
            eprintln!("The REPOSITORY environment variable was not set");
            return Err(());
        }
    };

    match env::var("BRANCH") {
        Ok(value) => {
            if (!value.to_string().ends_with("/stable") && !value.to_string().ends_with("/testing"))
                || value.to_string().len() < 7
            {
                eprintln!("The provided branch is invalid, it should be of the structure \"<paperback_major_minor_semver/<stable/testing>>\", consider using \"$${{ github.ref_name }}\"");
                return Err(());
            }
        }
        Err(_) => {
            eprintln!("The BRANCH environment variable was not set");
            return Err(());
        }
    };

    println!("Validated the presence and correctness of the following environment variables: PAT, REPOSITORY, BRANCH");
    Ok(())
}
