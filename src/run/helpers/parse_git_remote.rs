use anyhow::{Result, anyhow};
use lazy_static::lazy_static;

lazy_static! {
    static ref REMOTE_REGEX: regex::Regex = regex::Regex::new(
        r"(?P<domain>[^/@\.]+\.\w+)[:/](?P<owner>[^/]+)/(?P<repository>[^/]+?)(\.git)?/?$"
    )
    .unwrap();
}

#[derive(Debug)]
pub struct GitRemote {
    pub domain: String,
    pub owner: String,
    pub repository: String,
}

pub fn parse_git_remote(remote: &str) -> Result<GitRemote> {
    let captures = REMOTE_REGEX.captures(remote).ok_or_else(|| {
        anyhow!("Could not extract owner and repository from remote url: {remote}")
    })?;

    let domain = captures.name("domain").unwrap().as_str();
    let owner = captures.name("owner").unwrap().as_str();
    let repository = captures.name("repository").unwrap().as_str();

    Ok(GitRemote {
        domain: domain.to_string(),
        owner: owner.to_string(),
        repository: repository.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_remote() {
        let remote = "git@github.com:CodSpeedHQ/runner.git";
        let git_remote = parse_git_remote(remote).unwrap();
        insta::assert_debug_snapshot!(git_remote, @r###"
        GitRemote {
            domain: "github.com",
            owner: "CodSpeedHQ",
            repository: "runner",
        }
        "###);

        let remote = "https://github.com/CodSpeedHQ/runner.git";
        let git_remote = parse_git_remote(remote).unwrap();
        insta::assert_debug_snapshot!(git_remote, @r###"
        GitRemote {
            domain: "github.com",
            owner: "CodSpeedHQ",
            repository: "runner",
        }
        "###);

        let remote = "https://github.com/CodSpeedHQ/runner";
        let git_remote = parse_git_remote(remote).unwrap();
        insta::assert_debug_snapshot!(git_remote, @r###"
        GitRemote {
            domain: "github.com",
            owner: "CodSpeedHQ",
            repository: "runner",
        }
        "###);

        let remote = "git@gitlab.com:codspeed/runner.git";
        let git_remote = parse_git_remote(remote).unwrap();
        insta::assert_debug_snapshot!(git_remote, @r###"
        GitRemote {
            domain: "gitlab.com",
            owner: "codspeed",
            repository: "runner",
        }
        "###);

        let remote = "https://gitlab.com/codspeed/runner.git";
        let git_remote = parse_git_remote(remote).unwrap();
        insta::assert_debug_snapshot!(git_remote, @r###"
        GitRemote {
            domain: "gitlab.com",
            owner: "codspeed",
            repository: "runner",
        }
        "###);

        let remote = "https://github.com/codspeed/runner/";
        let git_remote = parse_git_remote(remote).unwrap();
        insta::assert_debug_snapshot!(git_remote, @r###"
        GitRemote {
            domain: "github.com",
            owner: "codspeed",
            repository: "runner",
        }
        "###);
    }
}
