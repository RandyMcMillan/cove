// Node connection constants

pub const BITCOIN_ESPLORA: [(&str, &str); 2] = [
    ("blockstream.info", "https://blockstream.info/api/"),
    ("mempool.space", "https://mempool.space/api/"),
];

pub const BITCOIN_ELECTRUM: [(&str, &str); 3] = [
    (
        "electrum.blockstream.info",
        "ssl://electrum.blockstream.info:50002",
    ),
    ("mempool.space electrum", "ssl://mempool.space:50002"),
    ("electrum.diynodes.com", "ssl://electrum.diynodes.com:50022"),
];

pub const TESTNET_ESPLORA: [(&str, &str); 2] = [
    ("mempool.space", "https://mempool.space/testnet/api/"),
    ("blockstream.info", "https://blockstream.info/testnet/api/"),
];

pub const TESTNET_ELECTRUM: [(&str, &str); 1] =
    [("testnet.hsmiths.com", "ssl://testnet.hsmiths.com:53012")];

pub const SIGNET_ESPLORA: [(&str, &str); 1] = [("mutinynet", "https://mutinynet.com/api")];