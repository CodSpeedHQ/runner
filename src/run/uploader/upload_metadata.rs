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

    use crate::run::{
        check_system::SystemInfo,
        instruments::InstrumentName,
        run_environment::{
            GhData, RepositoryProvider, RunEnvironment, RunEnvironmentMetadata, RunEvent, RunPart,
            Sender,
        },
        runner::ExecutorName,
        uploader::{Runner, UploadMetadata},
    };

    #[test]
    fn test_get_metadata_hash() {
        let upload_metadata = UploadMetadata {
            repository_provider: RepositoryProvider::GitHub,
            version: Some(7),
            tokenless: true,
            profile_md5: "jp/k05RKuqP3ERQuIIvx4Q==".into(),
            runner: Runner {
                name: "codspeed-runner".into(),
                version: "2.1.0".into(),
                instruments: vec![InstrumentName::MongoDB],
                executor: ExecutorName::Valgrind,
                system_info: SystemInfo::test(),
            },
            run_environment: RunEnvironment::GithubActions,
            commit_hash: "5bd77cb0da72bef094893ed45fb793ff16ecfbe3".into(),
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
            @"f827f6a834c26d39900c0a9e2dddfaaf22956494c8db911fc06fef72878b0c70"
        );
        assert_json_snapshot!(upload_metadata);
    }
}
