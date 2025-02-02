use home::home_dir;
use std::{ffi::OsStr, fs, io::Error};

use colored::*;
use inquire::{
    validator::{StringValidator, Validation},
    Confirm, Editor, InquireError, Select, Text,
};

use crate::git;

pub struct Config {
    pub pr_name: String,
    pub branch: String,
}

impl Config {
    pub fn set_github_token() -> Result<bool, InquireError> {
        let token = Text::new("Github token:").prompt_skippable()?;
        let token = match token {
            Some(token) => {
                if token.is_empty() {
                    return Ok(false);
                }
                token
            }
            None => return Ok(false),
        };
        let token = token.trim();

        let (dir_path, token_path, _) = Config::get_paths();

        if fs::read_dir(&dir_path).is_err() {
            fs::create_dir(&dir_path)?;
        }
        if fs::read(&token_path).is_err() {
            fs::File::create(&token_path)?;
        }
        fs::write(&token_path, token)?;

        Ok(true)
    }

    pub fn get_github_token() -> Result<String, Error> {
        let (_, token_path, _) = Config::get_paths();
        let token = fs::read_to_string(token_path)?;
        Ok(token)
    }

    pub fn set_default_desc() -> Result<bool, InquireError> {
        let actual = Config::get_default_desc().unwrap_or_default();
        let desc = Editor::new("Pull request description")
            .with_predefined_text(&actual)
            .with_editor_command(OsStr::new("vim"))
            .prompt_skippable()?;
        let desc = match desc {
            Some(desc) => {
                if desc.is_empty() {
                    return Ok(false);
                }
                desc
            }
            None => return Ok(false),
        };

        if actual == desc {
            return Ok(false);
        }

        let (dir_path, _, default_desc_path) = Config::get_paths();

        match fs::read_dir(&dir_path) {
            Ok(_) => {}
            Err(_) => {
                fs::create_dir(&dir_path)?;
            }
        };
        match fs::read(&default_desc_path) {
            Ok(_) => {}
            Err(_) => {
                fs::File::create(&default_desc_path)?;
            }
        }
        fs::write(&default_desc_path, desc)?;

        Ok(true)
    }

    pub fn get_default_desc() -> Result<String, Error> {
        let (_, _, default_desc_path) = Config::get_paths();
        let default_desc = fs::read_to_string(default_desc_path)?;
        Ok(default_desc)
    }

    pub fn ask_commit() -> Result<(String, String, String), InquireError> {
        let type_options: Vec<&str> = vec![
            "feat        A new feature",
            "fix         A bug fix",
            "docs        Documentation only changes",
            "style       Changes that do not affect the meaning of the code",
            "refactor    A code change that neither fixes a bug nor adds a feature",
            "perf        A code change that improves performance",
            "test        Adding missing tests or correcting existing tests",
            "build       Changes that affect the build system or external dependencies",
            "ci          Changes to our CI configuration files and scripts",
            "chore       Other changes that don't modify src or test files",
            "revert      Reverts a previous commit",
        ];

        let _type = Select::new("Type:", type_options).prompt()?;
        let _type = _type.split_whitespace().collect::<Vec<&str>>()[0];
        let _type = String::from(_type);

        let scope = Text::new("Scope (optional):").prompt_skippable()?;

        let name = Text::new("Name:")
            .with_validators(&[Box::new(get_not_empty_validator())])
            .prompt()?;
        let name = String::from(name.trim());

        let commit_name = match scope {
            Some(scope) => {
                if scope.is_empty() {
                    format!("{}: {}", _type, name)
                } else {
                    format!("{}({}): {}", _type, scope.trim(), name)
                }
            }
            None => format!("{}: {}", _type, name),
        };

        Ok((commit_name, _type, name))
    }

    pub fn ask_init() -> Result<(String, String, String), InquireError> {
        let (commit_name, _type, name) = Config::ask_commit()?;

        let branch = &name.replace(' ', "-").replace('\'', "").to_lowercase();
        let branch = format!("{}/{}", _type, branch);

        let repo = git::get_current_repo()?;
        let gh_compare_url = format!("https://github.com/{}/compare/{}?expand=1", repo, branch);

        println!(
            "\
This will:
1. Create a branch called {}.
2. Create a commit called {}.
3. Push to the remote repository.",
            branch.bright_cyan(),
            commit_name.bright_cyan(),
        );
        let confirm = Confirm::new("Confirm? (y/n)").prompt();
        match confirm {
            Ok(true) => Ok((commit_name, branch, gh_compare_url)),
            Ok(false) => Err(InquireError::OperationCanceled),
            Err(e) => Err(e),
        }
    }

    pub fn ask_pr() -> Result<Config, InquireError> {
        let linear_branch = Text::new("Linear branch name:")
            .with_validator(get_not_empty_validator())
            .prompt()?;

        let (mut pr_name, _type, _) = Config::ask_commit()?;

        let splited_branch = linear_branch.split('-').collect::<Vec<&str>>();
        if splited_branch.len() > 1 && splited_branch[1].parse::<u32>().is_ok() {
            pr_name = format!(
                "{} [{}-{}]",
                pr_name,
                splited_branch[0].to_uppercase(),
                splited_branch[1]
            )
        }

        let branch = format!("{}/{}", _type, &linear_branch);

        Ok(Config { pr_name, branch })
    }

    pub fn confirm_pr(&self) -> Result<bool, InquireError> {
        println!(
            "\
This will:
1. Create a branch called {}.
2. Create an empty commit.
3. Push to the remote repository.
4. Create a pull request named {}.
5. Assign you the pull request.",
            self.branch.bright_cyan(),
            self.pr_name.bright_cyan(),
        );
        Confirm::new("Confirm? (y/n)").prompt()
    }

    fn get_paths() -> (String, String, String) {
        let home = home_dir().unwrap();
        let home = home.to_str().unwrap();
        let dir_path = home.to_owned() + "/.ghl";
        let token_path = dir_path.to_owned() + "/token";
        let default_desc_path = dir_path.to_owned() + "/desc.md";
        (dir_path, token_path, default_desc_path)
    }
}

fn get_not_empty_validator() -> impl StringValidator {
    |value: &str| match value.is_empty() {
        true => Ok(Validation::Invalid("You must enter a value.".into())),
        false => Ok(Validation::Valid),
    }
}
