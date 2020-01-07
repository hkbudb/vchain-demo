use crate::{
    service::VChainService,
    transactions::{InitParam, RawObject, TxAddObjs},
};

use exonum::{
    api::node::public::explorer::TransactionQuery,
    crypto::{self, Hash, SecretKey},
    messages::{AnyTx, Verified},
    runtime::rust::Transaction,
};
use exonum_merkledb::ObjectHash;
use exonum_testkit::{ApiKind, TestKit, TestKitApi};
use serde_json::json;
use vchain::acc;

const INSTANCE_ID: u32 = 1;
const INSTANCE_NAME: &str = "vchain";

struct VChainApi {
    pub inner: TestKitApi,
}

impl VChainApi {
    fn add_objs(&self, input: TxAddObjs) -> (Verified<AnyTx>, SecretKey) {
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

fn create_testkit(param: InitParam) -> (TestKit, VChainApi) {
    let mut testkit = TestKit::for_rust_service(VChainService, INSTANCE_NAME, INSTANCE_ID, param);
    let api = VChainApi {
        inner: testkit.api(),
    };
    (testkit, api)
}

#[test]
fn test_initialize() {
    let (_, api) = create_testkit(InitParam {
        v_bit_len: vec![16],
        is_acc2: true,
        intra_index: true,
        skip_list_max_level: 2,
    });
    let param = api.get_param();
    assert_eq!(param.v_bit_len, vec![16]);
    assert_eq!(param.acc_type, acc::Type::ACC2);
    assert_eq!(param.use_sk, false);
    assert_eq!(param.intra_index, true);
    assert_eq!(param.skip_list_max_level, 2);
}

#[test]
fn test_add_objs() {
    let (mut testkit, api) = create_testkit(InitParam {
        v_bit_len: vec![16],
        is_acc2: true,
        intra_index: true,
        skip_list_max_level: 2,
    });
    let tx_input = TxAddObjs {
        objs: vec![
            RawObject {
                v_data: vec![1],
                w_data: vec!["a".to_owned()],
            },
            RawObject {
                v_data: vec![2],
                w_data: vec!["b".to_owned()],
            },
        ],
    };

    let (tx1, _) = api.add_objs(tx_input.clone());
    let (tx2, _) = api.add_objs(tx_input);
    testkit.create_block();
    api.assert_tx_status(tx1.object_hash(), &json!({ "type": "success" }));
    api.assert_tx_status(tx2.object_hash(), &json!({ "type": "success" }));
}
