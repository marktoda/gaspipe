use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "gaspipe",
    about = "A service for estimating gas of dependent transactions"
)]
pub struct Opt {
    /// The RPC url to fork from
    #[structopt(short, env = "FORK_URL")]
    pub fork_url: String,
}
