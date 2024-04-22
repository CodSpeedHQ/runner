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

    use crate::{
        ci_provider::interfaces::{GhData, ProviderMetadata, RunEvent, Sender},
        instruments::InstrumentNames,
        uploader::{Runner, UploadMetadata},
    };

    #[test]
    fn test_get_metadata_hash() {
        let upload_metadata = UploadMetadata {
            version: Some(2),
            tokenless: true,
            profile_md5: "jp/k05RKuqP3ERQuIIvx4Q==".into(),
            runner: Runner {
                name: "codspeed-runner".into(),
                version: "2.1.0".into(),
                instruments: vec![InstrumentNames::MongoDB],
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
            "8beb149c4645c666156e24fe0f68d24a63cec1d7756f35dd17cab1d84528ed7b"
        );
        assert_json_snapshot!(upload_metadata);
    }
}
