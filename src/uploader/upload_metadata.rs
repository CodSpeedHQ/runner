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
    use crate::uploader::{GhData, RunEvent, Runner, Sender, UploadMetadata};

    #[test]
    fn test_get_metadata_hash() {
        let upload_metadata = UploadMetadata {
            version: Some(1),
            tokenless: true,
            ref_: "refs/pull/29/merge".into(),
            head_ref: Some("chore/native-action-runner".into()),
            base_ref: Some("main".into()),
            owner: "CodSpeedHQ".into(),
            repository: "codspeed-node".into(),
            commit_hash: "ea4005444338762d85163c8e8787387e2ba97fb6".into(),
            event: RunEvent::PullRequest,
            profile_md5: "jp/k05RKuqP3ERQuIIvx4Q==".into(),
            gh_data: Some(GhData {
                run_id: 7044765741,
                job: "codspeed".into(),
                sender: Some(Sender {
                    id: 19605940,
                    login: "adriencaccia".into(),
                }),
            }),
            runner: Runner {
                name: "codspeed-runner".into(),
                version: "2.0.1-beta.6".into(),
            },
            platform: "github-actions".into(),
            repository_root_path: "/home/runner/work/codspeed-node/codspeed-node/".into(),
        };

        let hash = upload_metadata.get_hash();
        assert_eq!(
            hash,
            "ca9a71d06e61517fabd2784eb690b1cb8a9736f93122d07f20abb8e5356761ca"
        )
    }
}
