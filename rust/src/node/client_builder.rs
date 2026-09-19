use bdk_esplora::esplora_client;

use crate::{local_node_manager::resolve_selected_node, node::client::Error};

use super::{
    Node,
    client::{NodeClient, NodeClientOptions},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeClientBuilder {
    pub node: Node,
    pub options: NodeClientOptions,
}

impl NodeClientBuilder {
    pub async fn build(&self) -> Result<NodeClient, Error> {
        let node = if self.node.name == crate::node_connect::LOCAL_NODE_NAME {
            resolve_selected_node().await.map_err(|e| {
                Error::EsploraConnect(esplora_client::Error::HttpResponse {
                    status: 0,
                    message: e.to_string(),
                })
            })?
        } else {
            self.node.clone()
        };

        NodeClient::new_with_options(&node, self.options).await
    }
}
