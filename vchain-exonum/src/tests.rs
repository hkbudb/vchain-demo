use crate::{
    service::VChainService,
    transactions::{Config, TxSetParam},
};

use exonum::{
    api::{self, node::public::explorer::TransactionQuery},
    crypto::{self, Hash, SecretKey},
    messages::{AnyTx, Verified},
    runtime::rust::Transaction,
};
use exonum_merkledb::ObjectHash;
use exonum_testkit::{ApiKind, TestKit, TestKitApi};
use serde_json::json;

const INSTANCE_ID: u32 = 1;
const INSTANCE_NAME: &str = "vchain";

struct VChainApi {
    pub inner: TestKitApi,
}

impl VChainApi {
    fn set_param(&self, input: TxSetParam) -> (Verified<AnyTx>, SecretKey) {
        let (pubkey, key) = crypto::gen_keypair();
        let tx = input.sign(INSTANCE_ID, pubkey, &key);
        let tx_info: serde_json::Value = self
            .inner
            .public(ApiKind::Explorer)
            .query(&json!({ "tx_body": tx }))
            .post("v1/transactions")
            .unwrap();
        assert_eq!(tx_info, json!({ "tx_hash": tx.object_hash() }));
        (tx, key)
    }

    fn get_param(&self) -> vchain::Parameter {
        self.inner
            .public(ApiKind::Service(INSTANCE_NAME))
            .get("get/param")
            .unwrap()
    }

    fn assert_tx_status(&self, tx_hash: Hash, expected_status: &serde_json::Value) {
        let info: serde_json::Value = self
            .inner
            .public(ApiKind::Explorer)
            .query(&TransactionQuery::new(tx_hash))
            .get("v1/transactions")
            .unwrap();

        if let serde_json::Value::Object(mut info) = info {
            let tx_status = info.remove("status").unwrap();
            assert_eq!(tx_status, *expected_status);
        } else {
            panic!("Invalid transaction info format, object expected");
        }
    }
}

fn create_testkit() -> (TestKit, VChainApi) {
    let mut testkit = TestKit::for_rust_service(VChainService, INSTANCE_NAME, INSTANCE_ID, Config);
    let api = VChainApi {
        inner: testkit.api(),
    };
    (testkit, api)
}

#[test]
fn test_set_param() {
    let (mut testkit, api) = create_testkit();
    let tx_input = TxSetParam {
        v_bit_len: vec![16],
        is_acc2: true,
        intra_index: true,
        skip_list_max_level: 2,
    };

    let (tx, _) = api.set_param(tx_input);
    testkit.create_block();
    api.assert_tx_status(tx.object_hash(), &json!({ "type": "success" }));

    let param = api.get_param();
    dbg!(param);
}
