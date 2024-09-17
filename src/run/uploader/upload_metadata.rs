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
    use insta::assert_json_snapshot;

    use crate::run::{
        check_system::SystemInfo,
        ci_provider::interfaces::{GhData, ProviderMetadata, RunEvent, Sender},
        instruments::InstrumentNames,
        runner::ExecutorName,
        uploader::{Runner, UploadMetadata},
    };

    #[test]
    fn test_get_metadata_hash() {
        let upload_metadata = UploadMetadata {
            version: Some(3),
            tokenless: true,
            profile_md5: "jp/k05RKuqP3ERQuIIvx4Q==".into(),
            runner: Runner {
                name: "codspeed-runner".into(),
                version: "2.1.0".into(),
                instruments: vec![InstrumentNames::MongoDB],
                executor: ExecutorName::Valgrind,
                system_info: SystemInfo::test(),
            },
            platform: "github-actions".into(),
            commit_hash: "5bd77cb0da72bef094893ed45fb793ff16ecfbe3".into(),
            provider_metadata: ProviderMetadata {
                ref_: "refs/pull/29/merge".into(),
                head_ref: Some("chore/native-action-runner".into()),
                base_ref: Some("main".into()),
                owner: "CodSpeedHQ".into(),
                repository: "codspeed-node".into(),
                event: RunEvent::PullRequest,
                gh_data: Some(GhData {
                    run_id: 7044765741,
                    job: "codspeed".into(),
                    sender: Some(Sender {
                        id: 19605940,
                        login: "adriencaccia".into(),
                    }),
                }),
                repository_root_path: "/home/runner/work/codspeed-node/codspeed-node/".into(),
            },
        };

        let hash = upload_metadata.get_hash();
        assert_eq!(
            hash,
            "0d3ce9d6174eb0d6aa197b429237035291a662c46c7809198162202ca94aff5e"
        );
        assert_json_snapshot!(upload_metadata);
    }
}
