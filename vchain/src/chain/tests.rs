use super::*;
use crate::acc;
use crate::digest::{Digest, Digestible};
use anyhow::Context;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Default)]
struct FakeInMemChain {
    param: Option<Parameter>,
    block_headers: HashMap<IdType, BlockHeader>,
    block_data: HashMap<IdType, BlockData>,
    intra_index_nodes: HashMap<IdType, IntraIndexNode>,
    skip_list_nodes: HashMap<IdType, SkipListNode>,
    objects: HashMap<IdType, Object>,
}

#[async_trait::async_trait]
impl LightNodeInterface for FakeInMemChain {
    async fn lightnode_get_parameter(&self) -> Result<Parameter> {
        self.get_parameter()
    }
    async fn lightnode_read_block_header(&self, id: IdType) -> Result<BlockHeader> {
        self.read_block_header(id)
    }
}

impl ReadInterface for FakeInMemChain {
    fn get_parameter(&self) -> Result<Parameter> {
        self.param.clone().context("failed to get param")
    }
    fn read_block_header(&self, id: IdType) -> Result<BlockHeader> {
        self.block_headers
            .get(&id)
            .cloned()
            .context("failed to read block header")
    }
    fn read_block_data(&self, id: IdType) -> Result<BlockData> {
        self.block_data
            .get(&id)
            .cloned()
            .context("failed to read block data")
    }
    fn read_intra_index_node(&self, id: IdType) -> Result<IntraIndexNode> {
        self.intra_index_nodes
            .get(&id)
            .cloned()
            .context("failed to read intra index")
    }
    fn read_skip_list_node(&self, id: IdType) -> Result<SkipListNode> {
        self.skip_list_nodes
            .get(&id)
            .cloned()
            .context("failed to read skip list")
    }
    fn read_object(&self, id: IdType) -> Result<Object> {
        self.objects
            .get(&id)
            .cloned()
            .context("failed to read object")
    }
}

impl WriteInterface for FakeInMemChain {
    fn set_parameter(&mut self, param: Parameter) -> Result<()> {
        self.param = Some(param);
        Ok(())
    }
    fn write_block_header(&mut self, header: BlockHeader) -> Result<()> {
        let id = header.block_id;
        self.block_headers.insert(id, header);
        Ok(())
    }
    fn write_block_data(&mut self, data: BlockData) -> Result<()> {
        let id = data.block_id;
        self.block_data.insert(id, data);
        Ok(())
    }
    fn write_intra_index_node(&mut self, node: IntraIndexNode) -> Result<()> {
        let id = node.id();
        self.intra_index_nodes.insert(id, node);
        Ok(())
    }
    fn write_skip_list_node(&mut self, node: SkipListNode) -> Result<()> {
        let id = node.id;
        self.skip_list_nodes.insert(id, node);
        Ok(())
    }
    fn write_object(&mut self, obj: Object) -> Result<()> {
        let id = obj.id;
        self.objects.insert(id, obj);
        Ok(())
    }
}

impl FakeInMemChain {
    fn new() -> Self {
        Default::default()
    }

    fn build_chain(&mut self, data: &str, param: &Parameter) -> Result<()> {
        info!("build chain");
        self.set_parameter(param.clone())?;
        let mut prev_hash = Digest::default();
        for (id, objs) in load_raw_obj_from_str(data)?.iter() {
            let header = build_block(*id, prev_hash, objs.iter(), self)?;
            prev_hash = header.to_digest();
        }
        Ok(())
    }
}

const TEST_DATA_1: &str = r#"
1 [ 1 ] { a }
1 [ 2 ] { a }
1 [ 3 ] { a }
1 [ 4 ] { a }
2 [ 1 ] { b }
2 [ 2 ] { b }
2 [ 3 ] { b }
2 [ 4 ] { b }
"#;

const TEST_DATA_2: &str = r#"
1 [ 1 ] { a }
2 [ 1 ] { b }
3 [ 1 ] { b }
4 [ 1 ] { b }
5 [ 1 ] { a }
6 [ 1 ] { b }
7 [ 1 ] { b }
8 [ 1 ] { b }
9 [ 1 ] { b }
10 [ 1 ] { a }
11 [ 1 ] { b }
12 [ 1 ] { b }
13 [ 1 ] { b }
14 [ 1 ] { b }
15 [ 1 ] { b }
16 [ 1 ] { b }
17 [ 1 ] { b }
18 [ 1 ] { b }
19 [ 1 ] { a }
20 [ 1 ] { b }
"#;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[actix_rt::test]
async fn test_data1_acc1_flat() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC1,
        use_sk: true,
        intra_index: false,
        skip_list_max_level: 0,
    };
    chain.build_chain(TEST_DATA_1, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 2,
        "range": [
            [1],
            [1],
        ],
        "bool": [["a"]],
    }))
    .unwrap();
    let res: OverallResult<acc::Acc1Proof> = historical_query(&query, &chain).unwrap();
    assert_eq!(res.vo_stats.num_of_objs, 1);
    assert!(res.verify(&chain).await.unwrap().0.is_ok());
}

#[actix_rt::test]
async fn test_data1_acc1() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC1,
        use_sk: true,
        intra_index: true,
        skip_list_max_level: 0,
    };
    chain.build_chain(TEST_DATA_1, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 2,
        "range": [
            [1],
            [1],
        ],
        "bool": [["a"]],
    }))
    .unwrap();
    let res: OverallResult<acc::Acc1Proof> = historical_query(&query, &chain).unwrap();
    assert_eq!(res.vo_stats.num_of_objs, 1);
    assert!(res.verify(&chain).await.unwrap().0.is_ok());
}

#[actix_rt::test]
async fn test_data1_acc2_flat() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC2,
        use_sk: true,
        intra_index: false,
        skip_list_max_level: 0,
    };
    chain.build_chain(TEST_DATA_1, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 2,
        "range": [
            [1],
            [1],
        ],
        "bool": [["a"]],
    }))
    .unwrap();
    let res: OverallResult<acc::Acc2Proof> = historical_query(&query, &chain).unwrap();
    assert_eq!(res.vo_stats.num_of_objs, 1);
    assert!(res.verify(&chain).await.unwrap().0.is_ok());
}

#[actix_rt::test]
async fn test_data1_acc2() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC2,
        use_sk: true,
        intra_index: true,
        skip_list_max_level: 0,
    };
    chain.build_chain(TEST_DATA_1, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 2,
        "range": [
            [1],
            [1],
        ],
        "bool": [["a"]],
    }))
    .unwrap();
    let res: OverallResult<acc::Acc2Proof> = historical_query(&query, &chain).unwrap();
    assert_eq!(res.vo_stats.num_of_objs, 1);
    assert!(res.verify(&chain).await.unwrap().0.is_ok());
}

#[actix_rt::test]
async fn test_data2_acc2() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC2,
        use_sk: true,
        intra_index: true,
        skip_list_max_level: 0,
    };
    chain.build_chain(TEST_DATA_2, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 20,
        "range": [
            [1],
            [1],
        ],
        "bool": [["a"]],
    }))
    .unwrap();
    let res: OverallResult<acc::Acc2Proof> = historical_query(&query, &chain).unwrap();
    assert_eq!(res.vo_stats.num_of_objs, 4);
    assert!(res.verify(&chain).await.unwrap().0.is_ok());
}

#[actix_rt::test]
async fn test_data2_acc2_skip_list() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC2,
        use_sk: true,
        intra_index: true,
        skip_list_max_level: 2,
    };
    chain.build_chain(TEST_DATA_2, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 20,
        "range": [
            [1],
            [1],
        ],
        "bool": [["a"]],
    }))
    .unwrap();
    let res: OverallResult<acc::Acc2Proof> = historical_query(&query, &chain).unwrap();
    assert_eq!(res.vo_stats.num_of_objs, 4);
    assert!(res.verify(&chain).await.unwrap().0.is_ok());
}

#[actix_rt::test]
async fn test_data2_acc1_skip_list() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC1,
        use_sk: true,
        intra_index: true,
        skip_list_max_level: 2,
    };
    chain.build_chain(TEST_DATA_2, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 20,
        "range": [
            [1],
            [1],
        ],
        "bool": [["a"]],
    }))
    .unwrap();
    let res: OverallResult<acc::Acc1Proof> = historical_query(&query, &chain).unwrap();
    assert_eq!(res.vo_stats.num_of_objs, 4);
    assert!(res.verify(&chain).await.unwrap().0.is_ok());
}

#[actix_rt::test]
async fn test_data1_incomplete() {
    init_logger();
    let mut chain = FakeInMemChain::new();
    let param = Parameter {
        v_bit_len: vec![3],
        acc_type: acc::Type::ACC2,
        use_sk: true,
        intra_index: true,
        skip_list_max_level: 2,
    };
    chain.build_chain(TEST_DATA_1, &param).unwrap();
    let query = serde_json::from_value::<Query>(json!({
        "start_block": 1,
        "end_block": 2,
        "range": [
            [1],
            [1],
        ],
        "bool": null,
    }))
    .unwrap();
    let mut res: OverallResult<acc::Acc2Proof> = historical_query(&query, &chain).unwrap();
    let new_range = Range([vec![Some(1)], vec![Some(2)]]);
    res.query.q_range = Some(new_range);
    assert!(!res.verify(&chain).await.unwrap().0.is_ok());
}
