use injective_std::types::injective::auction::v1beta1::{
    MsgBid, MsgBidResponse, MsgUpdateParams, MsgUpdateParamsResponse, QueryAuctionParamsRequest,
    QueryAuctionParamsResponse, QueryCurrentAuctionBasketRequest,
    QueryCurrentAuctionBasketResponse, QueryLastAuctionResultRequest,
    QueryLastAuctionResultResponse, QueryModuleStateRequest, QueryModuleStateResponse,
};

use injective_std::types::cosmos::auth::v1beta1::{
    QueryModuleAccountsRequest, QueryModuleAccountsResponse,
};

use test_tube_inj::{fn_execute, fn_query};

use test_tube_inj::module::Module;
use test_tube_inj::runner::Runner;

pub struct Auction<'a, R: Runner<'a>> {
    runner: &'a R,
}

impl<'a, R: Runner<'a>> Module<'a, R> for Auction<'a, R> {
    fn new(runner: &'a R) -> Self {
        Self { runner }
    }
}

impl<'a, R> Auction<'a, R>
where
    R: Runner<'a>,
{
    fn_execute! {
        pub update_params: MsgUpdateParams["/injective.auction.v1beta1.MsgUpdateParams"] => MsgUpdateParamsResponse
    }

    fn_execute! {
        pub msg_bid: MsgBid["/injective.auction.v1beta1.MsgBid"] => MsgBidResponse
    }

    fn_query! {
        pub query_auction_params ["/injective.auction.v1beta1.Query/AuctionParams"]: QueryAuctionParamsRequest => QueryAuctionParamsResponse
    }

    fn_query! {
        pub query_current_auction_basket ["/injective.auction.v1beta1.Query/CurrentAuctionBasket"]: QueryCurrentAuctionBasketRequest => QueryCurrentAuctionBasketResponse
    }

    fn_query! {
        pub query_module_state ["/injective.auction.v1beta1.Query/ModuleState"]: QueryModuleStateRequest => QueryModuleStateResponse
    }

    fn_query! {
        pub query_module_accounts ["/cosmos.auth.v1beta1.Query/ModuleAccounts"]: QueryModuleAccountsRequest => QueryModuleAccountsResponse
    }

    fn_query! {
        pub query_last_auction_result ["/injective.auction.v1beta1.Query/LastAuctionResult"]: QueryLastAuctionResultRequest => QueryLastAuctionResultResponse
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Coin;
    use injective_std::types::{
        cosmos::{
            auth::v1beta1::{ModuleAccount, QueryModuleAccountsRequest},
            bank::v1beta1::MsgSend,
            base::v1beta1::Coin as TubeCoin,
        },
        injective::auction::v1beta1::{
            MsgBid, Params, QueryAuctionParamsRequest, QueryCurrentAuctionBasketRequest,
            QueryLastAuctionResultRequest,
        },
    };
    use prost::Message;

    use crate::{Auction, Bank, Gov, InjectiveTestApp};
    use test_tube_inj::{Account, Module};

    #[test]
    fn auction_integration() {
        let app = InjectiveTestApp::new();

        let auction = Auction::new(&app);

        let response = auction
            .query_auction_params(&QueryAuctionParamsRequest {})
            .unwrap();
        assert_eq!(
            response.params,
            Some(Params {
                auction_period: 604800,
                min_next_bid_increment_rate: 2_500_000_000_000_000u128.to_string()
            })
        );

        let response = auction
            .query_last_auction_result(&QueryLastAuctionResultRequest {})
            .unwrap();
        assert!(response.last_auction_result.is_some(),);
    }

    #[test]
    fn auction_bid() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let gov = Gov::new(&app);

        let signer = app
            .init_account(&[
                Coin::new(100_000_000_000_000_000_000_000u128, "inj"),
                Coin::new(100_000_000_000_000_000_000u128, "usdt"),
            ])
            .unwrap();

        let validator = app
            .get_first_validator_signing_account("inj".to_string(), 1.2f64)
            .unwrap();

        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: validator.address(),
                amount: vec![TubeCoin {
                    amount: "1000000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }],
            },
            &signer,
        )
        .unwrap();

        let auction = Auction::new(&app);

        let response = auction
            .query_auction_params(&QueryAuctionParamsRequest {})
            .unwrap();
        assert_eq!(
            response.params,
            Some(Params {
                auction_period: 604800,
                min_next_bid_increment_rate: 2_500_000_000_000_000u128.to_string()
            })
        );

        let response = auction
            .query_last_auction_result(&QueryLastAuctionResultRequest {})
            .unwrap();

        assert!(response.last_auction_result.is_some(),);

        let msg_bid_response = auction.msg_bid(
            MsgBid {
                bid_amount: Some(TubeCoin {
                    amount: "1000000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }),
                round: 0,
                sender: signer.address(),
            },
            &signer,
        );

        assert!(msg_bid_response.is_ok());

        app.increase_time(604800u64);

        let response = auction
            .query_last_auction_result(&QueryLastAuctionResultRequest {})
            .unwrap();

        assert!(response.last_auction_result.unwrap().winner == signer.address());
    }

    #[test]
    fn auction_event_auction_start() {
        let app = InjectiveTestApp::new();
        let bank = Bank::new(&app);
        let gov = Gov::new(&app);

        let signer = app
            .init_account(&[
                Coin::new(100_000_000_000_000_000_000_000u128, "inj"),
                Coin::new(100_000_000_000_000_000_000u128, "usdt"),
            ])
            .unwrap();

        let validator = app
            .get_first_validator_signing_account("inj".to_string(), 1.2f64)
            .unwrap();

        bank.send(
            MsgSend {
                from_address: signer.address(),
                to_address: String::from("inj1j4yzhgjm00ch3h0p9kel7g8sp6g045qf32pzlj"),
                amount: vec![TubeCoin {
                    amount: "1000000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }],
            },
            &signer,
        )
        .unwrap();

        let auction = Auction::new(&app);

        let r = auction
            .query_module_accounts(&QueryModuleAccountsRequest {})
            .unwrap();

        r.accounts.iter().for_each(|account| {
            let decoded_account = ModuleAccount::decode(account.value.as_ref()).unwrap();
            // println!("{:?}", decoded_account);
        });

        let response = auction
            .query_current_auction_basket(&QueryCurrentAuctionBasketRequest {})
            .unwrap();

        println!("{:?}", app.get_block_time_seconds());
        let msg_bid_response = auction.msg_bid(
            MsgBid {
                bid_amount: Some(TubeCoin {
                    amount: "1000000000000000000000".to_string(),
                    denom: "inj".to_string(),
                }),
                round: 0,
                sender: signer.address(),
            },
            &signer,
        );

        assert!(msg_bid_response.is_ok());

        app.increase_time(1000000);

        println!("{:?}", app.get_block_time_seconds());

        let response = auction
            .query_current_auction_basket(&QueryCurrentAuctionBasketRequest {})
            .unwrap();

        print!("{:?}", response);

        // assert_eq!(
        //     response.params,
        //     Some(Params {
        //         auction_period: 604800,
        //         min_next_bid_increment_rate: 2_500_000_000_000_000u128.to_string()
        //     })
        // );

        // let response = auction
        //     .query_last_auction_result(&QueryLastAuctionResultRequest {})
        //     .unwrap();

        // assert!(response.last_auction_result.is_some(),);

        // let msg_bid_response = auction.msg_bid(
        //     MsgBid {
        //         bid_amount: Some(TubeCoin {
        //             amount: "1000000000000000000000".to_string(),
        //             denom: "inj".to_string(),
        //         }),
        //         round: 3,
        //         sender: signer.address(),
        //     },
        //     &signer,
        // );

        // assert!(msg_bid_response.is_ok());

        // // app.increase_time(11u64);

        // let response = auction
        //     .query_last_auction_result(&QueryLastAuctionResultRequest {})
        //     .unwrap();

        // assert!(response.last_auction_result.unwrap().winner == signer.address());
    }
}
