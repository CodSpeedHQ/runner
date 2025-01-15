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
        ci_provider::interfaces::{
            CIProviderMetadata, GhData, Platform, RepositoryProvider, RunEvent, RunPart, Sender,
        },
        instruments::InstrumentName,
        runner::ExecutorName,
        uploader::{Runner, UploadMetadata},
    };

    #[test]
    fn test_get_metadata_hash() {
        let upload_metadata = UploadMetadata {
            repository_provider: RepositoryProvider::GitHub,
            version: Some(6),
            tokenless: true,
            profile_md5: "jp/k05RKuqP3ERQuIIvx4Q==".into(),
            runner: Runner {
                name: "codspeed-runner".into(),
                version: "2.1.0".into(),
                instruments: vec![InstrumentName::MongoDB],
                executor: ExecutorName::Valgrind,
                system_info: SystemInfo::test(),
            },
            platform: Platform::GithubActions,
            commit_hash: "5bd77cb0da72bef094893ed45fb793ff16ecfbe3".into(),
            ci_provider_metadata: CIProviderMetadata {
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
            @"78ff6843b32f6e0bd1f459534c35e69f0bf639b9d6c9a3cf7524af2595b305df"
        );
        assert_json_snapshot!(upload_metadata);
    }
}
