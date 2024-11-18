use std::env;

#[cfg(feature = "dotenv")]
use dotenvy;

#[cfg(feature = "dotenv")]
pub fn load_dotenv() -> Result<(), u8> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(err) => {
            eprintln!(
                "An error occurred wile trying to load the .env file: {}",
                &err
            );
            Err(0x1)
        }
    }
}

pub fn validate() -> Result<(), u8> {
    match env::var("REGISTRY_MANAGER_PAT") {
        Ok(value) => {
            if !value.to_string().starts_with("github_pat_") || value.to_string().len() != 93 {
                eprintln!("The provided personal_access_token is invalid, for more info check https://github.blog/security/application-security/introducing-fine-grained-personal-access-tokens-for-github/");
                return Err(0x1);
            }
        }
        Err(_) => {
            eprintln!("The REGISTRY_MANAGER_PAT environment variable was not found");
            return Err(0x1);
        }
    };

    match env::var("REGISTRY_MANAGER_REPOSITORY") {
        Ok(value) => {
            if !value.to_string().starts_with("paperback-community/")
                || value.to_string().len() < 20
            {
                eprintln!("The provided repository is invalid, it should be of the structure \"paperback-community/<repository_name>\", consider using \"$${{ github.repository_name }}\"");
                return Err(0x1);
            }
        }
        Err(_) => {
            eprintln!("The REGISTRY_MANAGER_REPOSITORY environment variable was not found");
            return Err(0x1);
        }
    };

    match env::var("REGISTRY_MANAGER_BRANCH") {
        Ok(value) => {
            if !value.to_string().starts_with("stable/") || value.to_string().len() < 7 {
                eprintln!("The provided branch is invalid, it should be of the structure \"stable/<paperback_semver>\", consider using \"$${{ github.ref_name }}\"");
                return Err(0x1);
            }
        }
        Err(_) => {
            eprintln!("The REGISTRY_MANAGER_BRANCH environment variable was not found");
            return Err(0x1);
        }
    };

    Ok(())
}
