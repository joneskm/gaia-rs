use bytes::Bytes;

use tendermint::{
    application::ABCIApplication,
    types::{
        chain_id::ChainId,
        proto::{
            block::BlockId,
            consensus::{Consensus, ConsensusParams},
            header::{Header, PartSetHeader},
            info::LastCommitInfo,
            validator::ValidatorUpdate,
        },
        request::{
            begin_block::RequestBeginBlock, deliver_tx::RequestDeliverTx,
            end_block::RequestEndBlock, init_chain::RequestInitChain, query::RequestQuery,
        },
        response::{deliver_tx::ResponseDeliverTx, query::ResponseQuery},
        time::timestamp::Timestamp,
    },
};
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Debug)]
pub struct InitState<G> {
    pub time: Timestamp,
    pub chain_id: ChainId,
    pub consensus_params: ConsensusParams,
    pub validators: Vec<ValidatorUpdate>,
    pub app_genesis: G,
    pub initial_height: u32,
}

impl<G> From<InitState<G>> for RequestInitChain<G> {
    fn from(init_state: InitState<G>) -> Self {
        Self {
            time: init_state.time,
            chain_id: init_state.chain_id,
            consensus_params: init_state.consensus_params,
            validators: init_state.validators,
            app_genesis: init_state.app_genesis,
            initial_height: init_state.initial_height,
        }
    }
}

#[derive(Debug)]
pub struct MockNode<App, G> {
    app: App,
    app_hash: Bytes,
    height: u32,
    chain_id: ChainId,
    time: Timestamp,
    last_block_id: BlockId,
    // last_header: Header,
    _phantom: std::marker::PhantomData<G>,
}

#[derive(Debug)]
pub struct StepResponse {
    pub app_hash: Bytes,
    pub tx_responses: Vec<ResponseDeliverTx>,
    pub height: u32,
}

impl<G: Clone, App: ABCIApplication<G>> MockNode<App, G> {
    pub fn new(app: App, init_state: InitState<G>) -> Self {
        // NOTE: the use of height here is complicated. Usually the init_state height will be 1. This
        // gets passed to the app in the init_chain call. This is consistent with TM which usually
        // passes 1 as the initial height (this is presumably taken from the genesis file - however
        // setting this to zero seems to break gaia - TM makes no progress - probably why it defaults to 1).
        // If (as usual) the height passed to the app is
        // 1 then the app subtracts 1 from the height in the header.
        // On the first begin block the header height passed by TM is also 1. The app uses this without modification.
        // To achieve this we initialise the height in this mock state to 0 such that on the first step this gets incremented to the correct height of 1.

        let res = app.init_chain(init_state.clone().into());

        Self {
            app,
            app_hash: res.app_hash,
            height: 0,
            chain_id: init_state.chain_id,
            time: init_state.time,
            last_block_id: BlockId {
                hash: vec![],
                part_set_header: Some(PartSetHeader {
                    total: 0,
                    hash: vec![],
                }),
            },
            _phantom: Default::default(),
        }
    }

    pub fn app_hash(&self) -> &Bytes {
        &self.app_hash
    }

    pub fn step(&mut self, txs: Vec<Bytes>, block_time: Timestamp) -> StepResponse {
        self.height += 1;
        self.time = block_time;
        let header = self.calculate_header();

        // TODO: update last_block_id

        let request_begin_block = RequestBeginBlock {
            header,
            last_commit_info: LastCommitInfo {
                round: 0,
                votes: vec![],
            },
            byzantine_validators: vec![],
            hash:  b"\xaaw\xbd^\x9d\x041\xfdc\x17\x11\x82\xb9iU\xde2\xd0\x19\xca\xdeV\x0e\x7fK\x1c\x88\xb6\xa3\xe3\x8b\x89".as_slice().into(),
        };
        self.app.begin_block(request_begin_block);

        let mut tx_responses = Vec::with_capacity(txs.len());
        for tx in txs {
            tx_responses.push(self.app.deliver_tx(RequestDeliverTx { tx }));
        }

        self.app.end_block(RequestEndBlock {
            height: self.height as i64,
        });

        let res_commit = self.app.commit();

        self.app_hash = res_commit.data;

        StepResponse {
            app_hash: self.app_hash.clone(),
            tx_responses,
            height: self.height,
        }
    }

    pub fn query(&self, req: RequestQuery) -> ResponseQuery {
        self.app.query(req)
    }

    fn calculate_header(&self) -> Header {
        Header {
            version: Consensus { block: 11, app: 10 },
            chain_id: self.chain_id.clone(),
            height: self.height,
            time: self.time,
            last_block_id: self.last_block_id.clone(),
            //TODO: need to calculate this
            last_commit_hash: vec![
                227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39,
                174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85,
            ],
            //TODO: need to calculate this
            data_hash: vec![
                227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39,
                174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85,
            ],
            //TODO: need to calculate this
            validators_hash: vec![
                105, 109, 157, 224, 221, 36, 139, 200, 18, 31, 171, 146, 191, 69, 50, 98, 210, 209,
                111, 225, 255, 132, 34, 75, 183, 135, 230, 89, 52, 173, 104, 13,
            ],
            //TODO: need to calculate this
            next_validators_hash: vec![
                105, 109, 157, 224, 221, 36, 139, 200, 18, 31, 171, 146, 191, 69, 50, 98, 210, 209,
                111, 225, 255, 132, 34, 75, 183, 135, 230, 89, 52, 173, 104, 13,
            ],
            //TODO: need to calculate this
            consensus_hash: vec![
                4, 128, 145, 188, 125, 220, 40, 63, 119, 191, 191, 145, 215, 60, 68, 218, 88, 195,
                223, 138, 156, 188, 134, 116, 5, 216, 183, 243, 218, 173, 162, 47,
            ],
            app_hash: self.app_hash.clone().into(),
            //TODO: need to calculate this
            last_results_hash: vec![
                227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39,
                174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85,
            ],
            //TODO: need to calculate this
            evidence_hash: vec![
                227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39,
                174, 65, 228, 100, 155, 147, 76, 164, 149, 153, 27, 120, 82, 184, 85,
            ],
            //TODO: need to calculate this
            proposer_address: vec![
                139, 66, 235, 161, 172, 24, 201, 229, 172, 156, 56, 187, 215, 206, 138, 87, 207,
                173, 214, 85,
            ],
        }
    }

    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    pub fn skip_steps(&mut self, steps: usize) {
        for _ in 0..steps {
            let _ = self.step(vec![], Timestamp::UNIX_EPOCH);
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
