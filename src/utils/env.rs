use std::env;

use tracing::error;

#[cfg(feature = "dotenv")]
use dotenvy;

#[cfg(feature = "dotenv")]
pub fn load_dotenv() -> Result<(), ()> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
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
    /*
     * Excluded because ${{ secrets.GITHUB_TOKEN }} will be used by default instead:
     *match env::var("GITHUB_TOKEN") {
     *  Ok(value) => {
     *      if !value.to_string().starts_with("github_pat_") || value.to_string().len() != 93 {
     *          error!("The provided (personal access) token is invalid, for more info on personal access tokens check https://github.blog/security/application-security/introducing-fine-grained-personal-access-tokens-for-github/");
     *          return Err(());
     *      }
     *  }
     *  Err(_) => {
     *      error!("The GITHUB_TOKEN environment variable was not set");
     *      return Err(());
     *  }
     *};
     */

    match env::var("REPOSITORY") {
        Ok(value) => {
            if !value.to_string().starts_with("paperback-community/")
                || value.to_string().len() < 20
            {
                error!(
                    "The provided repository is invalid, it should be of the structure \"paperback-community/<repository_name>\", consider using \"$${{ github.repository_name }}\""
                );
                return Err(());
            }
        }
        Err(_) => {
            error!("The REPOSITORY environment variable was not set");
            return Err(());
        }
    };

    match env::var("BRANCH") {
        Ok(value) => {
            if (!value.to_string().ends_with("/stable") && !value.to_string().ends_with("/testing"))
                || value.to_string().len() < 7
            {
                error!(
                    "The provided branch is invalid, it should be of the structure \"<paperback_major_minor_semver/<stable/testing>>\", consider using \"$${{ github.ref_name }}\""
                );
                return Err(());
            }
        }
        Err(_) => {
            error!("The BRANCH environment variable was not set");
            return Err(());
        }
    };

    Ok(())
}
