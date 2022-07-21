#![feature(proc_macro_hygiene, decl_macro, test)]
#[macro_use]
extern crate rocket;
use ethers::types::U256;
use rocket::{
    serde::{json::Json, Deserialize},
    State,
};
use structopt::StructOpt;
extern crate test;

mod opt;
use opt::Opt;
mod execute;
mod fork;
use execute::{execute, GasEstimate, Transaction};

#[launch]
fn rocket() -> _ {
    let opt = Opt::from_args();
    rocket::build().manage(opt).mount("/", routes![estimate])
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct RequestTransaction {
    from: String,
    to: String,
    data: String,
    value: String,
}

impl RequestTransaction {
    pub fn into(&self) -> Transaction {
        Transaction {
            from: self.from.parse().unwrap_or_default(),
            to: self.to.parse().unwrap_or_default(),
            data: hex::decode(&self.data.strip_prefix("0x").unwrap_or_default())
                .unwrap_or_default()
                .into(),
            value: U256::from_dec_str(&self.value).unwrap_or_default(),
        }
    }
}

#[post("/estimate", format = "json", data = "<transactions>")]
async fn estimate(
    opt: &State<Opt>,
    transactions: Json<Vec<RequestTransaction>>,
) -> Json<Vec<GasEstimate>> {
    Json(
        execute(
            &opt.fork_url,
            transactions
                .into_inner()
                .iter()
                .map(|t| t.into())
                .collect::<Vec<Transaction>>(),
        )
        .await
        .expect("Unable to execute transactions"),
    )
}

#[cfg(test)]
mod tests {
    use super::rocket;
    use crate::estimate;
    use crate::execute::GasEstimate;
    use crate::opt::Opt;
    use rocket::{
        http::{ContentType, Status},
        local::blocking::Client,
        serde::json,
        Build, Rocket,
    };
    use test::Bencher;
    const FORK_URL: &str = "https://mainnet.infura.io/v3/beb7a84398ad438caf3c2cf7e6802973";

    fn test_rocket() -> Rocket<Build> {
        let opt = Opt {
            fork_url: FORK_URL.to_string(),
        };
        rocket::build().manage(opt).mount("/", routes![estimate])
    }

    fn get_client() -> Client {
        Client::tracked(test_rocket()).expect("valid rocket instance")
    }

    #[test]
    fn test_estimate() {
        let client = get_client();
        let response = client
            .post("/estimate")
            .header(ContentType::JSON)
            .body(
                r#"
                  [
                  {
                      "from": "0x0000000000000000000000000000000000000000",
                      "to": "0x1111111111111111111111111111111111111111",
                      "value": "1000000000000000000",
                      "data": ""
                  }
                  ]
                  "#,
            )
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let data: Vec<GasEstimate> =
            json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(data.len(), 1);
        assert!(!data[0].reverted);
        assert_eq!(data[0].gas, 21000);
    }

    #[test]
    fn test_estimate_transfer_success() {
        let client = get_client();
        let response = client.post("/estimate")
            .header(ContentType::JSON)
            .body(r#"
                  [
                  {
                      "from": "0x28c6c06298d514db089934071355e5743bf21d60",
                      "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                      "value": "0",
                      "data": "0xa9059cbb000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000186a0"
                  }
                  ]
                  "#)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let data: Vec<GasEstimate> =
            json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(data.len(), 1);
        assert!(!data[0].reverted);
        assert!(data[0].gas > 21000);
    }

    #[test]
    fn test_estimate_transfer_from_failure() {
        let client = get_client();
        let response = client.post("/estimate")
            .header(ContentType::JSON)
            .body(r#"
                  [
                  {
                      "from": "0x1111111111111111111111111111111111111111",
                      "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                      "value": "0",
                      "data": "0x23b872dd00000000000000000000000028c6c06298d514db089934071355e5743bf21d60000000000000000000000000111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000f4240"
                  }
                  ]
                  "#)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let data: Vec<GasEstimate> =
            json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(data.len(), 1);
        assert!(data[0].reverted);
    }

    #[test]
    fn test_estimate_approve_transfer_from() {
        let client = get_client();
        let response = client.post("/estimate")
            .header(ContentType::JSON)
            .body(r#"
                  [
                  {
                      "from": "0x28c6c06298d514db089934071355e5743bf21d60",
                      "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                      "value": "0",
                      "data": "0x095ea7b3000000000000000000000000111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000f4240"
                  },
                  {
                      "from": "0x1111111111111111111111111111111111111111",
                      "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                      "value": "0",
                      "data": "0x23b872dd00000000000000000000000028c6c06298d514db089934071355e5743bf21d60000000000000000000000000111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000f4240"
                  }
                  ]
                  "#)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let data: Vec<GasEstimate> =
            json::from_str(response.into_string().unwrap().as_str()).unwrap();
        assert_eq!(data.len(), 2);
        assert!(!data[0].reverted);
        assert!(!data[1].reverted);
        assert!(data[0].gas > 21000);
        assert!(data[1].gas > 21000);
    }

    #[bench]
    fn bench_estimate_approve_transfer_from(b: &mut Bencher) {
        let client = get_client();
        b.iter(|| client.post("/estimate")
            .header(ContentType::JSON)
            .body(r#"
                  [
                  {
                      "from": "0x28c6c06298d514db089934071355e5743bf21d60",
                      "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                      "value": "0",
                      "data": "0x095ea7b3000000000000000000000000111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000f4240"
                  },
                  {
                      "from": "0x1111111111111111111111111111111111111111",
                      "to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                      "value": "0",
                      "data": "0x23b872dd00000000000000000000000028c6c06298d514db089934071355e5743bf21d60000000000000000000000000111111111111111111111111111111111111111100000000000000000000000000000000000000000000000000000000000f4240"
                  }
                  ]
                  "#)
            .dispatch());
    }
}
