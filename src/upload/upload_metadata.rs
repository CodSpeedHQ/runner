use serde_json::json;

use super::UploadMetadata;

impl UploadMetadata {
    pub fn get_hash(&self) -> String {
        let upload_metadata_string = json!(&self).to_string();
        sha256::digest(upload_metadata_string)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use insta::{assert_json_snapshot, assert_snapshot};

    use crate::executor::ExecutorName;
    use crate::instruments::InstrumentName;
    use crate::run_environment::{
        GhData, RepositoryProvider, RunEnvironment, RunEnvironmentMetadata, RunEvent, RunPart,
        Sender,
    };
    use crate::system::SystemInfo;
    use crate::upload::{LATEST_UPLOAD_METADATA_VERSION, Runner, UploadMetadata};

    #[test]
    fn test_get_metadata_hash() {
        let upload_metadata = UploadMetadata {
            repository_provider: RepositoryProvider::GitHub,
            version: Some(LATEST_UPLOAD_METADATA_VERSION),
            tokenless: true,
            profile_md5: "jp/k05RKuqP3ERQuIIvx4Q==".into(),
            profile_encoding: Some("gzip".into()),
            runner: Runner {
                name: "codspeed-runner".into(),
                version: "2.1.0".into(),
                instruments: vec![InstrumentName::MongoDB],
                executor: ExecutorName::Valgrind,
                system_info: SystemInfo::test(),
            },
            run_environment: RunEnvironment::GithubActions,
            commit_hash: "5bd77cb0da72bef094893ed45fb793ff16ecfbe3".into(),
            allow_empty: false,
            run_environment_metadata: RunEnvironmentMetadata {
                ref_: "refs/pull/29/merge".into(),
                head_ref: Some("chore/native-action-runner".into()),
                base_ref: Some("main".into()),
                owner: "CodSpeedHQ".into(),
                repository: "codspeed-node".into(),
                event: RunEvent::PullRequest,
                gh_data: Some(GhData {
                    run_id: "7044765741".into(),
                    job: "codspeed".into(),
                }),
                sender: Some(Sender {
                    id: "19605940".into(),
                    login: "adriencaccia".into(),
                }),
                gl_data: None,
                repository_root_path: "/home/runner/work/codspeed-node/codspeed-node/".into(),
            },
            run_part: Some(RunPart {
                run_id: "7044765741".into(),
                run_part_id: "benchmarks_3.2.2".into(),
                job_name: "codspeed".into(),
                metadata: BTreeMap::from([
                    ("someKey".into(), "someValue".into()),
                    ("anotherKey".into(), "anotherValue".into()),
                ]),
            }),
        };

        let hash = upload_metadata.get_hash();
        assert_snapshot!(
            hash,
            // Caution: when changing this value, we need to ensure that
            // the related backend snapshot remains the same
            @"11f363bd959389e57c79f6fc7d5c965d168c7b2f3cb2b566b647588b23322013"
        );
        assert_json_snapshot!(upload_metadata);
    }
}
